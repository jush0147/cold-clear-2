use std::sync::Arc;

use cold_clear_2::bot::{Bot, BotConfig};
use cold_clear_2::data::{Board, GameState, Piece, PieceLocation, Placement, Rotation, Spin};
use cold_clear_2::movegen::find_moves;
use cold_clear_2::tbp::{Randomizer, Start};
use cold_clear_2::try_create_bot;
use enumset::EnumSet;

fn start(hold: Option<Piece>) -> Start {
    Start {
        board: Board::default(),
        queue: vec![Piece::O, Piece::I, Piece::T, Piece::L, Piece::J, Piece::S],
        hold, combo: 0, back_to_back: false, b2b_count: 0,
        randomizer: Randomizer::Unknown,
    }
}
fn bot(s: Start) -> Bot { try_create_bot(s, Arc::new(BotConfig::default())).unwrap() }
fn move_for(b: &Bot, p: Piece) -> Placement {
    find_moves(&b.state().board, p).into_iter().find(|(m, _)| m.spin == Spin::None).unwrap().0
}

#[test]
fn short_boards_are_bottom_up_and_padded_not_indexed_out_of_bounds() {
    let mut rows = vec![[None; 10]; 20];
    rows[0][2] = Some('G');
    rows[19][7] = Some('T');
    let board: Board = serde_json::from_str(&serde_json::to_string(&rows).unwrap()).unwrap();
    assert_eq!(board.cols[2], 1);
    assert_eq!(board.cols[7], 1 << 19);
    assert_eq!(board.cols.iter().map(|c| c.count_ones()).sum::<u32>(), 2);
    let empty: Board = serde_json::from_str("[]").unwrap();
    assert_eq!(empty, Board::default());
}
#[test]
fn excess_rows_and_wrong_width_return_errors() {
    let rows = vec![[None::<char>; 10]; 41];
    assert!(serde_json::from_str::<Board>(&serde_json::to_string(&rows).unwrap()).is_err());
    assert!(serde_json::from_str::<Board>("[[null,null]]").is_err());
}
#[test]
fn start_rejects_hidden_previews_and_inconsistent_counters() {
    let mut s = start(Some(Piece::Z));
    s.queue.push(Piece::Z);
    assert!(s.validate().is_err());
    let mut s = start(None);
    s.b2b_count = 4;
    assert!(s.validate().is_err());
    let mut s = start(None);
    s.combo = 256;
    assert!(s.validate().is_err());
    let mut s = start(None);
    s.queue.clear();
    assert!(s.validate().is_err());
}
#[test]
fn empty_hold_is_not_exposed_as_a_real_prehold() {
    let b = bot(start(None));
    let p = b.player_pieces();
    assert_eq!(p.current, Some(Piece::O));
    assert_eq!(p.hold, None);
    assert_eq!(p.next, vec![Piece::I, Piece::T, Piece::L, Piece::J, Piece::S]);
    assert_eq!(b.preview_refill_needed(), 0);
}
#[test]
fn no_hold_consumes_one_piece_and_remains_empty() {
    let mut b = bot(start(None));
    let mv = move_for(&b, Piece::O);
    b.try_play(mv, false).unwrap();
    assert_eq!(b.player_pieces().current, Some(Piece::I));
    assert_eq!(b.player_pieces().hold, None);
    assert_eq!(b.preview_refill_needed(), 1);
    b.new_piece(Piece::Z);
    assert_eq!(b.player_pieces().next, vec![Piece::T, Piece::L, Piece::J, Piece::S, Piece::Z]);
    assert_eq!(b.preview_refill_needed(), 0);
}
#[test]
fn first_actual_hold_consumes_two_pieces_and_reveals_two() {
    let mut b = bot(start(None));
    let mv = move_for(&b, Piece::I);
    b.try_play(mv, true).unwrap();
    assert_eq!(b.player_pieces().hold, Some(Piece::O));
    assert_eq!(b.player_pieces().current, Some(Piece::T));
    assert_eq!(b.preview_refill_needed(), 2);
    b.new_piece(Piece::Z);
    b.new_piece(Piece::O);
    assert_eq!(b.player_pieces().next, vec![Piece::L, Piece::J, Piece::S, Piece::Z, Piece::O]);
    assert_eq!(b.preview_refill_needed(), 0);
}
#[test]
fn identical_current_and_next_still_require_explicit_hold_for_replays() {
    let mut s = start(None);
    s.queue[1] = Piece::O;
    let mut no_hold = bot(s);
    let mut s = start(None);
    s.queue[1] = Piece::O;
    let mut hold = bot(s);
    let mv = move_for(&no_hold, Piece::O);
    no_hold.try_play(mv, false).unwrap();
    hold.try_play(mv, true).unwrap();
    assert_eq!(no_hold.state().board, hold.state().board);
    assert_eq!(no_hold.player_pieces().hold, None);
    assert_eq!(no_hold.player_pieces().current, Some(Piece::O));
    assert_eq!(no_hold.preview_refill_needed(), 1);
    assert_eq!(hold.player_pieces().hold, Some(Piece::O));
    assert_eq!(hold.player_pieces().current, Some(Piece::T));
    assert_eq!(hold.preview_refill_needed(), 2);
}
#[test]
fn occupied_hold_swap_consumes_only_one_queued_piece() {
    let mut b = bot(start(Some(Piece::Z)));
    let mv = move_for(&b, Piece::Z);
    b.try_play(mv, true).unwrap();
    assert_eq!(b.player_pieces().hold, Some(Piece::O));
    assert_eq!(b.player_pieces().current, Some(Piece::I));
    assert_eq!(b.preview_refill_needed(), 1);
}
#[test]
fn invalid_moves_do_not_mutate_the_board_or_queue() {
    let mut b = bot(start(None));
    let before = b.state();
    let pieces = b.player_pieces();
    let mut mv = move_for(&b, Piece::O);
    mv.location.x = 127;
    mv.location.y = -128;
    assert!(b.try_play(mv, false).is_err());
    assert_eq!(b.state(), before);
    assert_eq!(b.player_pieces(), pieces);
    let wrong_piece = move_for(&b, Piece::T);
    assert!(b.try_play(wrong_piece, false).is_err());
    assert_eq!(b.state(), before);
    let mut wrong_spin = move_for(&b, Piece::O);
    wrong_spin.spin = Spin::Full;
    assert!(b.try_play(wrong_spin, false).is_err());
    assert_eq!(b.state(), before);
}

fn rng(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}
#[test]
fn bitboard_line_compaction_matches_independent_row_grid_4096_cases() {
    let mut seed = 0x6a09_e667_f3bc_c909u64;
    for case in 0..4096 {
        let mut rows = [[false; 10]; 40];
        let mut b = Board::default();
        for y in 0..40 {
            let bits = rng(&mut seed);
            for x in 0..10 {
                rows[y][x] = bits & (1 << x) != 0;
                if rows[y][x] { b.cols[x] |= 1 << y; }
            }
        }
        // Independent row compaction, including non-adjacent rows and row 39.
        let removed = rng(&mut seed) & ((1u64 << 40) - 1);
        let retained: Vec<_> = rows.iter().enumerate().filter(|(y, _)| removed & (1 << y) == 0).map(|(_, row)| *row).collect();
        b.remove_lines(removed);
        for y in 0..40 {
            for x in 0..10 {
                let expected = retained.get(y).map(|r| r[x]).unwrap_or(false);
                assert_eq!(b.cols[x] & (1 << y) != 0, expected, "case={case} x={x} y={y}");
            }
        }
    }
}
#[test]
fn line_detection_matches_independent_row_grid_1024_cases() {
    let mut seed = 0xbb67_ae85_84ca_a73bu64;
    for _ in 0..1024 {
        let mut b = Board::default();
        let mut expected = 0;
        for y in 0..40 {
            let bits = if rng(&mut seed) % 4 == 0 { 1023 } else { rng(&mut seed) & 1023 };
            for x in 0..10 { if bits & (1 << x) != 0 { b.cols[x] |= 1 << y; } }
            if bits == 1023 { expected |= 1 << y; }
        }
        assert_eq!(b.line_clears(), expected);
    }
}
#[test]
fn generated_locks_are_in_bounds_nonoverlapping_and_grounded() {
    let mut seed = 0x3c6e_f372_fe94_f82bu64;
    for _ in 0..64 {
        let mut b = Board::default();
        for x in 0..10 {
            let height = rng(&mut seed) % 20;
            b.cols[x] = ((1 << height) - 1) & rng(&mut seed);
        }
        for p in [Piece::I, Piece::O, Piece::T, Piece::L, Piece::J, Piece::S, Piece::Z] {
            for (mv, _) in find_moves(&b, p) {
                assert!(!mv.location.obstructed(&b));
                assert!(mv.location.cells().iter().all(|&(x, y)| (0..10).contains(&x) && (0..40).contains(&y)));
                let down = PieceLocation { y: mv.location.y - 1, ..mv.location };
                assert!(down.obstructed(&b));
            }
        }
    }
}

fn state() -> GameState {
    GameState { board: Board::default(), bag: EnumSet::all(), reserve: Piece::O, back_to_back: false, b2b_count: 0, combo: 0, forecast: Default::default() }
}
fn quad(s: &mut GameState) -> cold_clear_2::data::PlacementInfo {
    s.board = Board { cols: [15; 10], ..Board::default() };
    s.board.cols[4] = 0;
    s.board.cols[0] |= 1 << 5; // Avoid a perfect clear.
    s.advance(Piece::I, Placement { location: PieceLocation { piece: Piece::I, rotation: Rotation::East, x: 4, y: 2 }, spin: Spin::None })
}
fn single(s: &mut GameState) -> cold_clear_2::data::PlacementInfo {
    s.board = Board { cols: [1; 10], ..Board::default() };
    s.board.cols[4] = 0;
    s.board.cols[5] = 0;
    s.advance(Piece::O, Placement { location: PieceLocation { piece: Piece::O, rotation: Rotation::North, x: 4, y: 0 }, spin: Spin::None })
}
#[test]
fn combo_b2b_and_nonclears_transition_independently() {
    let mut s = state();
    let first = quad(&mut s);
    assert_eq!(first.combo, 0);
    assert_eq!(first.b2b_count_after, 0);
    assert!(!first.back_to_back);
    assert!(s.back_to_back);
    let second = quad(&mut s);
    assert_eq!(second.combo, 1);
    assert_eq!(second.b2b_count_after, 1);
    assert!(second.back_to_back);
    s.board = Board::default();
    let none = s.advance(Piece::O, Placement { location: PieceLocation { piece: Piece::O, rotation: Rotation::North, x: 0, y: 0 }, spin: Spin::None });
    assert_eq!(none.lines_cleared, 0);
    assert_eq!(s.combo, 0);
    assert!(s.back_to_back);
    assert_eq!(s.b2b_count, 1);
    let normal = single(&mut s);
    assert_eq!(normal.combo, 0);
    assert!(normal.b2b_broken);
    assert!(!s.back_to_back);
    assert_eq!(s.b2b_count, 0);
}
