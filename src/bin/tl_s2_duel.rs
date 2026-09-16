use std::collections::VecDeque;
use std::env;
use std::sync::Arc;
use cold_clear_2::bot::{Bot, BotConfig};
use cold_clear_2::try_create_bot;
use cold_clear_2::data::{Board, Piece};
use cold_clear_2::tbp::{Randomizer, Start};
use cold_clear_2::tetrio;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

const VISIBLE_QUEUE: usize = 6;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind { Legacy, S2 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome { P0, P1 }
#[derive(Default)]
struct Score { s2_wins: u64, legacy_wins: u64 }
struct Player { bot: Bot, incoming: VecDeque<usize>, next_piece: usize }
struct PieceSequence { rng: StdRng, pieces: Vec<Piece> }
impl PieceSequence {
    fn new(seed: u64) -> Self {
        let mut s = Self { rng: StdRng::seed_from_u64(seed), pieces: vec![] };
        s.ensure(VISIBLE_QUEUE);
        s
    }
    fn ensure(&mut self, len: usize) {
        while self.pieces.len() < len {
            let mut bag = [Piece::I, Piece::O, Piece::T, Piece::L, Piece::J, Piece::S, Piece::Z];
            bag.shuffle(&mut self.rng);
            self.pieces.extend_from_slice(&bag);
        }
    }
    fn get(&mut self, index: usize) -> Piece { self.ensure(index + 1); self.pieces[index] }
}

#[derive(Clone, Copy)]
struct Settings { seeds: u64, nodes: u64, cap: usize, charge: f32, surge: f32, shape: f32 }
fn settings(args: &[String]) -> Result<Settings, String> {
    let mut s = Settings { seeds: 20, nodes: 500, cap: 8, charge: 0.5, surge: 1.0, shape: 0.5 };
    if args.len() % 2 != 0 { return Err("arguments must be flag/value pairs".into()); }
    for pair in args.chunks_exact(2) {
        let bad = || format!("invalid value for {}", pair[0]);
        match pair[0].as_str() {
            "--seeds" => s.seeds = pair[1].parse().map_err(|_| bad())?,
            "--nodes" => s.nodes = pair[1].parse().map_err(|_| bad())?,
            "--garbage-cap" => s.cap = pair[1].parse().map_err(|_| bad())?,
            "--charge" => s.charge = pair[1].parse().map_err(|_| bad())?,
            "--surge" => s.surge = pair[1].parse().map_err(|_| bad())?,
            "--shape" => s.shape = pair[1].parse().map_err(|_| bad())?,
            flag => return Err(format!("unknown flag {flag}; this KO-only duel has no --pieces option")),
        }
    }
    if s.seeds == 0 || s.nodes == 0 || s.cap == 0 || s.cap > 40 { return Err("seeds/nodes must be positive; garbage cap must be 1..40".into()); }
    if [s.charge, s.surge, s.shape].iter().any(|x| !x.is_finite() || *x < 0.0) { return Err("weights must be finite and nonnegative".into()); }
    Ok(s)
}
fn main() -> Result<(), String> {
    let s = settings(&env::args().skip(1).collect::<Vec<_>>())?;
    println!("Draft S2 KO-only duel: rules parity NOT verified, pending garbage NOT in search.");
    println!("seeds={} games={} node_budget/move={} garbage_cap={} charge={} surge={} shape={}", s.seeds, s.seeds.saturating_mul(2), s.nodes, s.cap, s.charge, s.surge, s.shape);
    println!("Current + hold + NEXT x5 only. Empty hold is preserved; the first actual hold reveals two new previews.");
    println!("No piece cap or attack tiebreak. External timeout/abort is incomplete, never a winner.\n");
    let mut score = Score::default();
    for seed in 0..s.seeds {
        for swapped in [false, true] {
            let kinds = if swapped { [Kind::S2, Kind::Legacy] } else { [Kind::Legacy, Kind::S2] };
            let outcome = duel(seed, kinds, s).map_err(|e| format!("ABORTED seed={seed} swapped={swapped}: {e}; no result assigned"))?;
            let winner = kinds[match outcome { Outcome::P0 => 0, Outcome::P1 => 1 }];
            match winner { Kind::S2 => score.s2_wins += 1, Kind::Legacy => score.legacy_wins += 1 }
            println!("seed={seed} swapped={swapped} winner={winner:?}");
        }
    }
    let total = score.s2_wins + score.legacy_wins;
    println!("S2 wins: {}\nlegacy wins: {}", score.s2_wins, score.legacy_wins);
    println!("Simplified-simulator S2 win rate: {:.1}%", 100.0 * score.s2_wins as f64 / total as f64);
    println!("Not a TL strength claim. Still missing SRS+/180, Clutch Clears, live timing, opener cancellation, garbage-special +1 and Surge packets. Legacy is the legacy evaluator on this same modified core.");
    Ok(())
}
fn duel(seed: u64, kinds: [Kind; 2], s: Settings) -> Result<Outcome, String> {
    let mut sequence = PieceSequence::new(seed);
    let initial = sequence.pieces[..VISIBLE_QUEUE].to_vec();
    let mut players = [make_player(&initial, kinds[0], s)?, make_player(&initial, kinds[1], s)?];
    let mut round = 0u64;
    loop {
        for active in 0..2 {
            if play_turn(&mut players, active, seed, round, s, &mut sequence)? {
                return Ok(if active == 0 { Outcome::P1 } else { Outcome::P0 });
            }
        }
        round = round.checked_add(1).ok_or("round counter overflow")?;
    }
}
fn play_turn(players: &mut [Player; 2], active: usize, seed: u64, round: u64, s: Settings, sequence: &mut PieceSequence) -> Result<bool, String> {
    let other = 1 - active;
    if players[active].bot.preview_refill_needed() != 0 { return Err("incomplete player-visible queue".into()); }
    let mut searched = 0;
    while searched < s.nodes {
        let stats = players[active].bot.do_work();
        searched += stats.nodes;
        if stats.nodes == 0 { break; }
    }
    let suggestion = players[active].bot.suggest();
    let mv = match suggestion.first() {
        Some(&mv) => mv,
        None if players[active].bot.has_legal_move() => return Err("search returned no move despite a supported legal placement".into()),
        None => return Ok(true),
    };
    let info = players[active].bot.try_advance(mv)?;
    let attack = tetrio::attack(&info).total as usize;
    let cancel = attack.min(players[active].incoming.len());
    for _ in 0..cancel { players[active].incoming.pop_front(); }
    let outgoing = attack - cancel;
    if outgoing != 0 {
        // Simulator-owned hole data never enters the bot's observation.
        let hole = garbage_hole(seed, active as u64, round);
        for _ in 0..outgoing { players[other].incoming.push_back(hole); }
    }
    // Deliberately simplified activation. Kept out of correctness certification.
    if info.lines_cleared == 0 {
        let rise = s.cap.min(players[active].incoming.len());
        for _ in 0..rise {
            let hole = players[active].incoming.pop_front().unwrap();
            if players[active].bot.add_garbage_line(hole) { return Ok(true); }
        }
    }
    // One reveal after a normal placement; TWO after the first actual hold.
    let refill = players[active].bot.preview_refill_needed();
    for _ in 0..refill {
        let p = sequence.get(players[active].next_piece);
        players[active].bot.new_piece(p);
        players[active].next_piece += 1;
    }
    Ok(false)
}
fn make_player(visible: &[Piece], kind: Kind, s: Settings) -> Result<Player, String> {
    let start = Start { board: Board::default(), queue: visible.to_vec(), hold: None, combo: 0, back_to_back: false, b2b_count: 0, randomizer: Randomizer::Unknown };
    let config = match kind { Kind::Legacy => BotConfig::legacy(), Kind::S2 => BotConfig::tetrio_s2(s.charge, s.surge, s.shape) };
    Ok(Player { bot: try_create_bot(start, Arc::new(config))?, incoming: VecDeque::new(), next_piece: VISIBLE_QUEUE })
}
fn garbage_hole(seed: u64, sender: u64, round: u64) -> usize {
    let mut x = seed ^ sender.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ round.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 30; x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27; x = x.wrapping_mul(0x94D0_49BB_1331_11EB); x ^= x >> 31;
    (x % 10) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(a: &[&str]) -> Vec<String> { a.iter().map(|x| x.to_string()).collect() }
    #[test]
    fn zero_budget_cannot_be_reported_as_a_ko() { assert!(settings(&args(&["--nodes", "0"])).is_err()); }
    #[test]
    fn obsolete_piece_cap_is_rejected_not_ignored() { assert!(settings(&args(&["--pieces", "120"])).is_err()); }
    #[test]
    fn nan_weights_are_rejected() { assert!(settings(&args(&["--shape", "NaN"])).is_err()); }
    #[test]
    fn generated_bags_contain_each_piece_once() {
        let mut s = PieceSequence::new(42);
        s.ensure(700);
        for bag in s.pieces.chunks_exact(7) {
            let set: enumset::EnumSet<Piece> = bag.iter().copied().collect();
            assert_eq!(set.len(), 7);
        }
    }
}
