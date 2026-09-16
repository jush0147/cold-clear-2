//! Shared KO-experiment mechanics and independent search RNG.
use std::cell::RefCell;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use crate::data::Board;
thread_local! { static SEARCH_RNG: RefCell<Option<StdRng>> = RefCell::new(None); }
/// The search seed must be independent of the arena's piece/garbage seeds.
pub fn with_search_seed<T>(seed: u64, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<StdRng>);
    impl Drop for Restore {
        fn drop(&mut self) { SEARCH_RNG.with(|r| *r.borrow_mut() = self.0.take()); }
    }
    let old = SEARCH_RNG.with(|r| r.replace(Some(StdRng::seed_from_u64(seed))));
    let _restore = Restore(old);
    f()
}
pub(crate) fn random_unit() -> f64 {
    SEARCH_RNG.with(|r| match r.borrow_mut().as_mut() {
        Some(r) => r.gen(), None => rand::thread_rng().gen(),
    })
}
pub(crate) fn random_index(len: usize) -> usize {
    SEARCH_RNG.with(|r| match r.borrow_mut().as_mut() {
        Some(r) => r.gen_range(0..len), None => rand::thread_rng().gen_range(0..len),
    })
}
/// Existing opening cancellation policy, shared by forecast and authority.
/// Returns canceled incoming and unused ordinary attack. Bonus never sends.
pub fn cancel_plan(attack: u32, incoming: u32, pieces: u32, sent: u32) -> (u32, u32) {
    let ordinary = attack.min(incoming);
    let bonus = if pieces < 14 && incoming >= sent { attack.min(incoming - ordinary) } else { 0 };
    (ordinary + bonus, attack - ordinary)
}
pub fn board_danger(board: &Board, max_cover: u32) -> (u32, u32, u32) {
    let (mut max_height, mut count, mut covered) = (0, 0, 0);
    for &col in &board.cols {
        let height = 64 - col.leading_zeros();
        max_height = max_height.max(height);
        let mut holes = !col & if height == 64 { u64::MAX } else { (1u64 << height) - 1 };
        count += holes.count_ones();
        while holes != 0 {
            covered += (height - holes.trailing_zeros()).min(max_cover);
            holes &= holes - 1;
        }
    }
    (max_height, count, covered)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::forecast::Forecast;
    use crate::tetrio::garbage::GarbagePacket;
    #[test]
    fn search_rng_repeatable() {
        let a = with_search_seed(7, || (random_unit(), random_index(7)));
        let b = with_search_seed(7, || (random_unit(), random_index(7)));
        assert_eq!(a, b);
    }
    #[test]
    fn opening_bonus_never_sends() {
        assert_eq!(cancel_plan(3,5,0,0),(5,0));
        assert_eq!(cancel_plan(3,5,14,0),(3,0));
        assert_eq!(cancel_plan(7,5,0,0),(5,2));
    }
    #[test]
    fn snapshot_does_not_invent_activation_times() {
        let mut f = Forecast::snapshot(&[GarbagePacket{lines:8,active:false}],20,0,0).unwrap();
        let mut b = Board::default();
        for _ in 0..1000 { f.resolve(&mut b,&[],0); }
        assert_eq!(b,Board::default());
        assert_eq!(f.remaining(),8);
    }
    #[test]
    fn active_garbage_applied_once_with_provenance() {
        let mut f = Forecast::snapshot(&[GarbagePacket{lines:8,active:true}],20,0,0).unwrap();
        let mut b = Board::default();
        f.resolve(&mut b,&[],0);
        assert_eq!((b.cols[0],b.cols[1],b.garbage_rows,f.remaining()),(0,255,255,0));
        f.resolve(&mut b,&[],0);
        assert_eq!(b.cols[1],255);
    }
}
