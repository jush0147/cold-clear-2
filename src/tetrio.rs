//! TL profile observed in the 2026-09-10 replay (engine version 19).
//! Full game parity is still not certified. See docs/replay-validation.md.
use crate::data::{PlacementInfo, Spin};
use serde::Serialize;
#[path = "garbage.rs"]
pub mod garbage;
pub const PARITY_VERIFIED: bool = false;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AttackBreakdown {
    pub base: u32,
    pub b2b_bonus: u32,
    pub combo: u32,
    pub perfect_clear_bonus: u32,
    pub garbage_special_bonus: u32,
    pub line_clear_attack: u32,
    pub surge_released: u32,
    pub total: u32,
    /// Surge first (three packets), ordinary clear, then additive PC packet.
    /// Zero entries are omitted by packets().
    packet_values: [u32; 5],
}
impl AttackBreakdown {
    pub fn packets(&self) -> Vec<u32> { self.packet_values.iter().copied().filter(|&n| n > 0).collect() }
}
pub fn base_attack(spin: Spin, lines: u32) -> u32 {
    match spin {
        Spin::None => match lines { 0 | 1 => 0, 2 => 1, 3 => 2, 4 => 4, _ => lines },
        Spin::Mini => match lines { 0 | 1 => 0, 2 => 1, 3 => 2, 4 => 10, _ => 2 * lines + 2 },
        Spin::Full => match lines { 0 => 0, 1 => 2, 2 => 4, 3 => 6, 4 => 10, _ => 2 * lines + 2 },
    }
}
pub fn multiplier_attack(base_plus_b2b: u32, combo: u32) -> u32 {
    let scaled = base_plus_b2b as f64 * (1.0 + 0.25 * combo as f64);
    let minimum = if combo > 1 { (1.25 * combo as f64).ln_1p() } else { 0.0 };
    scaled.max(minimum).floor() as u32
}
pub fn surge_size(count: u32) -> u32 { if count >= 4 { count } else { 0 } }
pub fn attack(info: &PlacementInfo) -> AttackBreakdown {
    if info.lines_cleared == 0 { return AttackBreakdown::default(); }
    let base = base_attack(info.placement.spin, info.lines_cleared);
    let spin_or_quad = info.lines_cleared >= 4 || info.placement.spin != Spin::None;
    let b2b_bonus = u32::from((spin_or_quad || info.perfect_clear) && info.back_to_back);
    let garbage_special_bonus = u32::from(spin_or_quad && info.garbage_cleared > 0);
    let perfect_clear_bonus = if info.perfect_clear { 5 } else { 0 };
    let line_clear_attack = multiplier_attack(base + b2b_bonus, info.combo) + garbage_special_bonus;
    let surge_released = if info.b2b_broken { surge_size(info.b2b_count_before) } else { 0 };
    // Integer equivalent of JS Math.round(surge / 3); denominator 3 has no ties.
    let third = (surge_released + 1) / 3;
    AttackBreakdown {
        base, b2b_bonus, combo: info.combo, perfect_clear_bonus, garbage_special_bonus,
        line_clear_attack, surge_released,
        total: line_clear_attack + perfect_clear_bonus + surge_released,
        packet_values: [third, third, surge_released - 2 * third, line_clear_attack, perfect_clear_bonus],
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Piece, PieceLocation, Placement, Rotation};
    fn info(spin: Spin, lines: u32, combo: u32, b2b: bool) -> PlacementInfo {
        PlacementInfo {
            placement: Placement { location: PieceLocation { piece: Piece::T, rotation: Rotation::North, x: 4, y: 1 }, spin },
            lines_cleared: lines, garbage_cleared: 0, combo, back_to_back: b2b,
            b2b_count_before: if b2b { 3 } else { 0 }, b2b_count_after: if b2b { 4 } else { 0 }, b2b_broken: false, perfect_clear: false,
        }
    }
    #[test]
    fn pc_is_a_separate_additive_packet() {
        let mut i=info(Spin::None,2,1,false); i.perfect_clear=true;
        assert_eq!(attack(&i).packets(), vec![1,5]);
    }
    #[test]
    fn garbage_bonus_is_flat_and_requires_garbage() {
        let mut i=info(Spin::Full,2,4,true);
        assert_eq!(attack(&i).total,10);i.garbage_cleared=2;
        assert_eq!(attack(&i).total,11);
        i.placement.spin=Spin::None; i.back_to_back=false;
        assert_eq!(attack(&i).garbage_special_bonus,0);
    }
    #[test]
    fn surge_precedes_normal_attack_and_conserves_lines() {
        for n in 4..100 {
            let mut i=info(Spin::None,2,0,false);i.b2b_broken=true;i.b2b_count_before=n;
            let a=attack(&i);let packets=a.packets();
            assert_eq!(packets.iter().sum::<u32>(),n+1);
            assert_eq!(packets.last(),Some(&1));
        }
    }
    #[test]
    fn mini_table_includes_i_mini_quad() { assert_eq!(base_attack(Spin::Mini,4),10); }
    #[test]
    fn nonclear_never_sends_even_with_stale_flags() {
        let mut i=info(Spin::Full,0,50,true); i.perfect_clear=true;i.b2b_broken=true;i.b2b_count_before=30;i.garbage_cleared=4;
        assert_eq!(attack(&i),AttackBreakdown::default());
    }
    #[test]
    fn combo_rounding() { assert_eq!(multiplier_attack(4,1),5);assert_eq!(multiplier_attack(0,6),2); }
}
