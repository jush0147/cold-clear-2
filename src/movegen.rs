use std::cmp::Ordering;
use std::collections::BinaryHeap;
use ahash::AHashMap;
use crate::data::*;

pub fn find_moves(board: &Board, piece: Piece) -> Vec<(Placement, u32)> {
    find_moves_with_clutch(board, piece, false)
}

/// Spawn-based SRS+ reachability; a preceding line clear permits Clutch rescue.
pub fn find_moves_with_clutch(board: &Board, piece: Piece, allow_clutch: bool) -> Vec<(Placement, u32)> {
    puffin::profile_function!();
    let mut queue = BinaryHeap::new();
    let mut values = AHashMap::new();
    let mut underground_locks = AHashMap::new();
    let mut locks = Vec::with_capacity(64);
    let collision_map = CollisionMaps::new(board, piece);
    let fast_mode = board.cols.iter().all(|&c| c.leading_zeros() > 64 - 16);
    if fast_mode {
        for &rotation in &[Rotation::North, Rotation::East, Rotation::South, Rotation::West] {
            for x in 0..10 {
                let mut location = PieceLocation { piece, rotation, x, y: 21 };
                if collision_map.obstructed(location) { continue; }
                let distance = location.drop_distance(board);
                location.y -= distance;
                let mv = Placement { location, spin: Spin::None };
                let mut update = update_position(&mut queue, &mut values, fast_mode, board);
                if let Some(mv) = shift(location, &collision_map, -1) { update(mv, distance as u32); }
                if let Some(mv) = shift(location, &collision_map, 1) { update(mv, distance as u32); }
                if let Some(mv) = rotate_cw(location, &collision_map, board) { update(mv, distance as u32); }
                if let Some(mv) = rotate_ccw(location, &collision_map, board) { update(mv, distance as u32); }
                if let Some(mv) = rotate_180(location, &collision_map, board) { update(mv, distance as u32); }
                if location.canonical_form() == location { locks.push((mv, 0)); }
            }
        }
    } else {
        let mut spawned = PieceLocation { piece, rotation: Rotation::North, x: 4, y: 21 };
        while collision_map.obstructed(spawned) {
            // Tetrp's spawn pivot is at top-down y=17.96. Clutch rescue moves
            // it upward in whole rows, but B.occupied rejects raw y<0 BEFORE
            // ceil(). The CC2 integer y=39 position corresponds to the illegal
            // Tetrp pivot -0.04; y=38 is the highest legal rescued pivot.
            if !allow_clutch || spawned.y >= 38 { return vec![]; }
            spawned.y += 1;
        }
        let spawned = Placement { location: spawned, spin: Spin::None };
        queue.push(Intermediate { soft_drops: 0, mv: spawned });
        values.insert(spawned, 0);
    }
    while let Some(expand) = queue.pop() {
        if expand.soft_drops != values.get(&expand.mv).copied().unwrap_or(40) { continue; }
        let drop_dist = expand.mv.location.drop_distance(board);
        let dropped = Placement {
            location: PieceLocation { y: expand.mv.location.y - drop_dist, ..expand.mv.location },
            spin: if drop_dist == 0 { expand.mv.spin } else { Spin::None },
        };
        let sds = underground_locks.entry(Placement { location: dropped.location.canonical_form(), ..dropped }).or_insert(expand.soft_drops);
        *sds = expand.soft_drops.min(*sds);
        let mut update = update_position(&mut queue, &mut values, fast_mode, board);
        update(dropped, expand.soft_drops + drop_dist as u32);
        if let Some(mv) = shift(expand.mv.location, &collision_map, -1) { update(mv, expand.soft_drops); }
        if let Some(mv) = shift(expand.mv.location, &collision_map, 1) { update(mv, expand.soft_drops); }
        if let Some(mv) = rotate_cw(expand.mv.location, &collision_map, board) { update(mv, expand.soft_drops); }
        if let Some(mv) = rotate_ccw(expand.mv.location, &collision_map, board) { update(mv, expand.soft_drops); }
        if let Some(mv) = rotate_180(expand.mv.location, &collision_map, board) { update(mv, expand.soft_drops); }
    }
    locks.extend(underground_locks.into_iter());
    locks.sort_by_key(|(m, sd)| (m.location.piece as u8, m.location.x, m.location.y, m.location.rotation as u8, m.spin as u8, *sd));
    locks.dedup_by_key(|(m, _)| *m);
    locks
}

fn update_position<'a>(queue: &'a mut BinaryHeap<Intermediate>, values: &'a mut AHashMap<Placement, u32>, fast_mode: bool, board: &'a Board) -> impl FnMut(Placement, u32) + 'a {
    move |target: Placement, soft_drops: u32| {
        if fast_mode && target.spin == Spin::None && target.location.above_stack(board) { return; }
        let prev_sds = values.entry(target).or_insert(40);
        if soft_drops < *prev_sds {
            *prev_sds = soft_drops;
            queue.push(Intermediate { soft_drops, mv: target });
        }
    }
}
fn shift(mut location: PieceLocation, collision_map: &CollisionMaps, dx: i8) -> Option<Placement> {
    location.x += dx;
    if collision_map.obstructed(location) { return None; }
    Some(Placement { location, spin: Spin::None })
}
fn rotate_cw(from: PieceLocation, map: &CollisionMaps, board: &Board) -> Option<Placement> {
    rotate_to(from, from.rotation.cw(), map, board, true)
}
fn rotate_ccw(from: PieceLocation, map: &CollisionMaps, board: &Board) -> Option<Placement> {
    rotate_to(from, from.rotation.ccw(), map, board, true)
}
fn rotate_180(from: PieceLocation, map: &CollisionMaps, board: &Board) -> Option<Placement> {
    rotate_to(from, from.rotation.flip(), map, board, false)
}
fn rotate_to(from: PieceLocation, to: Rotation, map: &CollisionMaps, board: &Board, allow_fin: bool) -> Option<Placement> {
    rotate(PieceLocation { rotation: to, ..from }, map, board,
        crate::srs_plus::kicks(from.piece, from.rotation, to).iter().copied(), allow_fin)
}
fn rotate(unkicked: PieceLocation, map: &CollisionMaps, board: &Board, kicks: impl Iterator<Item = (i8, i8)>, allow_fin: bool) -> Option<Placement> {
    for (i, (dx, dy)) in kicks.enumerate() {
        let target = PieceLocation { x: unkicked.x + dx, y: unkicked.y + dy, ..unkicked };
        if map.obstructed(target) { continue; }
        let immobile = is_immobile(target, map);
        let spin = if target.piece != Piece::T {
            if immobile { Spin::Mini } else { Spin::None }
        } else {
            let corners = [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().filter(|&&(cx, cy)| board.occupied((cx + target.x, cy + target.y))).count();
            let front = [(-1, 1), (1, 1)].iter().map(|&c| target.rotation.rotate_cell(c)).filter(|&(cx, cy)| board.occupied((cx + target.x, cy + target.y))).count();
            if corners < 3 { if immobile { Spin::Mini } else { Spin::None } }
            else if front == 2 || (allow_fin && i == 4 && dy == -2 && matches!(target.rotation, Rotation::East | Rotation::West)) { Spin::Full }
            else { Spin::Mini }
        };
        return Some(Placement { location: target, spin });
    }
    None
}
fn is_immobile(location: PieceLocation, map: &CollisionMaps) -> bool {
    [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().all(|&(dx, dy)| map.obstructed(PieceLocation { x: location.x + dx, y: location.y + dy, ..location }))
}

#[derive(Clone, Copy, Debug, Eq)]
struct Intermediate { mv: Placement, soft_drops: u32 }
impl PartialEq for Intermediate { fn eq(&self, other: &Self) -> bool { self.soft_drops == other.soft_drops } }
impl Ord for Intermediate {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap. Reverse the cost for shortest-cost search.
        other.soft_drops.cmp(&self.soft_drops)
    }
}
impl PartialOrd for Intermediate { fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) } }

struct CollisionMaps { boards: [[u64; 10]; 4] }
impl CollisionMaps {
    fn new(board: &Board, piece: Piece) -> Self {
        let mut boards = [[0; 10]; 4];
        // Pinned Tetrp spawn phase is y=buffer-2.04. With integer SRS+
        // shifts/kicks and g=0, newly spawned hypothetical pieces cannot occupy
        // storage row 0 (CC2 y=39): the corresponding raw Tetrp cell would be
        // slightly negative and is out of bounds. Treat y=39 as the movegen
        // ceiling for descendant spawn/clutch search. Actual replay roots are
        // enumerated by Tetrp authority geometry and bypass this spawn surrogate.
        const CEILING: u64 = !((1u64 << 39) - 1);
        for rot in [Rotation::North, Rotation::West, Rotation::South, Rotation::East] {
            for (dx, dy) in rot.rotate_cells(piece.cells()) {
                for x in 0..10 {
                    // Board::occupied treats cells above row 39 as occupied.
                    // Include those same walls in the optimized collision map.
                    let c = board.cols.get((x + dx) as usize).map(|c| *c | CEILING).unwrap_or(!0);
                    let c = if dy < 0 { !(!c << -dy) } else { c >> dy };
                    boards[rot as usize][x as usize] |= c;
                }
            }
        }
        Self { boards }
    }
    fn obstructed(&self, piece: PieceLocation) -> bool {
        piece.y < 0 || piece.y >= 40 || self.boards[piece.rotation as usize].get(piece.x as usize).map(|&c| c & (1u64 << piece.y) != 0).unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn immobility_requires_all_four_directions_blocked() {
        let mut board = Board::default();
        board.cols[3] |= 1;
        board.cols[0] |= 1 << 1;
        let location = PieceLocation { piece: Piece::L, rotation: Rotation::North, x: 1, y: 0 };
        let map = CollisionMaps::new(&board, Piece::L);
        assert!(!map.obstructed(location));
        assert!(is_immobile(location, &map));
        assert!(!is_immobile(location, &CollisionMaps::new(&Board::default(), Piece::L)));
    }
    #[test]
    fn optimized_collision_matches_cell_reference_including_ceiling() {
        for board in [Board::default(), Board { cols: [0x0080_0001_0180; 10], ..Board::default() }] {
            for piece in [Piece::I, Piece::O, Piece::T, Piece::L, Piece::J, Piece::S, Piece::Z] {
                let map = CollisionMaps::new(&board, piece);
                for rotation in [Rotation::North, Rotation::West, Rotation::South, Rotation::East] {
                    for x in -3..13 {
                        for y in -3..44 {
                            let loc = PieceLocation { piece, rotation, x, y };
                            assert_eq!(map.obstructed(loc), loc.obstructed(&board), "{loc:?}");
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn out_of_range_anchors_do_not_shift_by_64_or_more() {
        let map = CollisionMaps::new(&Board::default(), Piece::T);
        for y in [40, 63, 64, 127, -128] {
            assert!(map.obstructed(PieceLocation { piece: Piece::T, rotation: Rotation::North, x: 4, y }));
        }
    }
    #[test]
    fn frontier_pops_lowest_softdrop_cost_first() {
        let mv = Placement { location: PieceLocation { piece: Piece::O, rotation: Rotation::North, x: 0, y: 0 }, spin: Spin::None };
        let mut heap = BinaryHeap::new();
        for soft_drops in [7, 0, 3, 1] { heap.push(Intermediate { mv, soft_drops }); }
        assert_eq!((0..4).map(|_| heap.pop().unwrap().soft_drops).collect::<Vec<_>>(), vec![0, 1, 3, 7]);
    }
}
