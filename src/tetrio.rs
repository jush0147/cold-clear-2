//! Draft TETR.IO S2 scoring model.
//!
//! Regression tests protect internal behavior; they are not an independent
//! oracle for the production game. PC/mini/combo edge cases require fixtures
//! from a pinned game version. Garbage-special +1, opening cancellation and
//! Surge packet timing are not covered by attack(). See docs/s2-audit.md.

use crate::data::{PlacementInfo, Spin};

#[path = "garbage.rs"]
pub mod garbage;

pub const PARITY_VERIFIED: bool = false;

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

/// Inherited draft table. Do not infer production parity from its test names.
pub fn base_attack(spin: Spin, lines: u32) -> u32 {
    match spin {
        Spin::None | Spin::Mini => match lines { 0 | 1 => 0, 2 => 1, 3 => 2, 4 => 4, _ => lines },
        Spin::Full => match lines { 0 => 0, 1 => 2, 2 => 4, 3 => 6, 4 => 10, _ => 2 * lines + 2 },
    }
}

pub fn multiplier_attack(base_plus_b2b: u32, combo: u32) -> u32 {
    if base_plus_b2b == 0 {
        if combo <= 1 { 0 } else { ((1.25 * combo as f64 + 1.0).ln()).floor() as u32 }
    } else {
        ((base_plus_b2b as f64) * (1.0 + 0.25 * combo as f64)).floor() as u32
    }
}

pub fn surge_size(b2b_count: u32) -> u32 { if b2b_count >= 4 { b2b_count } else { 0 } }

pub fn attack(info: &PlacementInfo) -> AttackBreakdown {
    // A non-clear cannot send attack or release a Surge, even if a caller
    // accidentally supplies a stale combo, PC, spin or broken-B2B flag.
    if info.lines_cleared == 0 { return AttackBreakdown::default(); }
    let base = base_attack(info.placement.spin, info.lines_cleared);
    let difficult = info.lines_cleared == 4 || !matches!(info.placement.spin, Spin::None) || info.perfect_clear;
    let b2b_bonus = u32::from(difficult && info.back_to_back);
    // Retain the existing ADDITIVE +5 until independently verified. The phrase
    // "All Clears send 5" in patch notes does not establish replacement vs bonus.
    let perfect_clear_bonus = if info.perfect_clear { 5 } else { 0 };
    let line_clear_attack = multiplier_attack(base + b2b_bonus, info.combo) + perfect_clear_bonus;
    let surge_released = if info.b2b_broken { surge_size(info.b2b_count_before) } else { 0 };
    AttackBreakdown {
        base, b2b_bonus, combo: info.combo, perfect_clear_bonus, line_clear_attack,
        surge_released, total: line_clear_attack + surge_released,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Piece, PieceLocation, Placement, Rotation};
    fn info(spin: Spin, lines: u32, combo: u32, b2b: bool) -> PlacementInfo {
        PlacementInfo {
            placement: Placement { location: PieceLocation { piece: Piece::T, rotation: Rotation::North, x: 4, y: 1 }, spin },
            lines_cleared: lines, combo, back_to_back: b2b,
            b2b_count_before: if b2b { 3 } else { 0 }, b2b_count_after: if b2b { 4 } else { 0 },
            b2b_broken: false, perfect_clear: false,
        }
    }
    #[test]
    fn draft_base_table_regression() {
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
    fn multiplier_rounding_regression() {
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
        assert_eq!((a.base, a.b2b_bonus, a.line_clear_attack), (4, 1, 5));
    }
    #[test]
    fn surge_counter_regression() {
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
        assert_eq!((a.line_clear_attack, a.surge_released, a.total), (1, 8, 9));
    }
    #[test]
    fn additive_pc_behavior_is_not_changed_from_ambiguous_prose() {
        let mut i = info(Spin::None, 4, 0, false);
        i.perfect_clear = true;
        assert_eq!(attack(&i).total, 9);
    }
    #[test]
    fn nonclear_never_sends_even_with_stale_attack_flags() {
        for spin in [Spin::None, Spin::Mini, Spin::Full] {
            let mut i = info(spin, 0, 50, true);
            i.perfect_clear = true;
            i.b2b_broken = true;
            i.b2b_count_before = 30;
            assert_eq!(attack(&i), AttackBreakdown::default());
        }
    }
}
