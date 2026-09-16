//! TETR.IO Tetra League Season 2 attack rules.
//!
//! This module intentionally contains pure functions so rules can be unit-tested
//! separately from Cold Clear's search/evaluation heuristics.

use crate::data::{PlacementInfo, Spin};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttackBreakdown {
    pub base: u32,
    pub b2b_bonus: u32,
    pub combo: u32,
    pub perfect_clear_bonus: u32,
    pub line_clear_attack: u32,
    pub surge_released: u32,
    pub total: u32,
}

/// Base garbage before B2B, combo multiplier, perfect clear, Surge, or the
/// garbage-special +1 bonus.
///
/// Under TL S2 All-Mini+, Mini spins use the same base table as normal clears.
/// Full spins use TETR.IO's Spin table.
pub fn base_attack(spin: Spin, lines: u32) -> u32 {
    match spin {
        Spin::None | Spin::Mini => match lines {
            0 | 1 => 0,
            2 => 1,
            3 => 2,
            4 => 4,
            _ => lines,
        },
        Spin::Full => match lines {
            0 => 0,
            1 => 2,
            2 => 4,
            3 => 6,
            4 => 10,
            _ => 2 * lines + 2,
        },
    }
}

/// TL uses DOWN rounding for the Multiplier combo system.
pub fn multiplier_attack(base_plus_b2b: u32, combo: u32) -> u32 {
    if base_plus_b2b == 0 {
        if combo <= 1 {
            0
        } else {
            ((1.25 * combo as f64 + 1.0).ln()).floor() as u32
        }
    } else {
        ((base_plus_b2b as f64) * (1.0 + 0.25 * combo as f64)).floor() as u32
    }
}

/// In Tetra League, Surge starts at B2B x4 with power 4 and then grows 1:1
/// with the visible B2B counter.
pub fn surge_size(b2b_count: u32) -> u32 {
    if b2b_count >= 4 { b2b_count } else { 0 }
}

pub fn attack(info: &PlacementInfo) -> AttackBreakdown {
    let base = base_attack(info.placement.spin, info.lines_cleared);
    let difficult = info.lines_cleared == 4
        || !matches!(info.placement.spin, Spin::None)
        || info.perfect_clear;
    let b2b_bonus = u32::from(difficult && info.back_to_back);
    let perfect_clear_bonus = if info.perfect_clear { 5 } else { 0 };
    let line_clear_attack = multiplier_attack(base + b2b_bonus, info.combo)
        + perfect_clear_bonus;
    let surge_released = if info.b2b_broken {
        surge_size(info.b2b_count_before)
    } else {
        0
    };

    AttackBreakdown {
        base,
        b2b_bonus,
        combo: info.combo,
        perfect_clear_bonus,
        line_clear_attack,
        surge_released,
        total: line_clear_attack + surge_released,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Piece, PieceLocation, Placement, Rotation};

    fn info(spin: Spin, lines: u32, combo: u32, b2b: bool) -> PlacementInfo {
        PlacementInfo {
            placement: Placement {
                location: PieceLocation {
                    piece: Piece::T,
                    rotation: Rotation::North,
                    x: 4,
                    y: 1,
                },
                spin,
            },
            lines_cleared: lines,
            combo,
            back_to_back: b2b,
            b2b_count_before: if b2b { 3 } else { 0 },
            b2b_count_after: if b2b { 4 } else { 0 },
            b2b_broken: false,
            perfect_clear: false,
        }
    }

    #[test]
    fn base_table_matches_tl_s2() {
        assert_eq!(base_attack(Spin::None, 1), 0);
        assert_eq!(base_attack(Spin::None, 2), 1);
        assert_eq!(base_attack(Spin::None, 3), 2);
        assert_eq!(base_attack(Spin::None, 4), 4);
        assert_eq!(base_attack(Spin::Mini, 2), 1);
        assert_eq!(base_attack(Spin::Full, 1), 2);
        assert_eq!(base_attack(Spin::Full, 2), 4);
        assert_eq!(base_attack(Spin::Full, 3), 6);
        assert_eq!(base_attack(Spin::Full, 4), 10);
    }

    #[test]
    fn tl_rounds_multiplier_down() {
        assert_eq!(multiplier_attack(1, 1), 1);
        assert_eq!(multiplier_attack(4, 1), 5);
        assert_eq!(multiplier_attack(4, 2), 6);
        assert_eq!(multiplier_attack(0, 2), 1);
        assert_eq!(multiplier_attack(0, 5), 1);
        assert_eq!(multiplier_attack(0, 6), 2);
    }

    #[test]
    fn difficult_b2b_adds_one_before_combo_multiplier() {
        let a = attack(&info(Spin::Full, 2, 0, true));
        assert_eq!(a.base, 4);
        assert_eq!(a.b2b_bonus, 1);
        assert_eq!(a.line_clear_attack, 5);
    }

    #[test]
    fn surge_is_tl_visible_b2b_count() {
        assert_eq!(surge_size(3), 0);
        assert_eq!(surge_size(4), 4);
        assert_eq!(surge_size(8), 8);
        assert_eq!(surge_size(42), 42);
    }

    #[test]
    fn breaking_b2b_releases_surge() {
        let mut i = info(Spin::None, 2, 0, false);
        i.b2b_broken = true;
        i.b2b_count_before = 8;
        i.b2b_count_after = 0;
        let a = attack(&i);
        assert_eq!(a.line_clear_attack, 1);
        assert_eq!(a.surge_released, 8);
        assert_eq!(a.total, 9);
    }

    #[test]
    fn perfect_clear_adds_five() {
        let mut i = info(Spin::None, 4, 0, false);
        i.perfect_clear = true;
        assert_eq!(attack(&i).total, 9);
    }
}