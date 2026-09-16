use std::env;
use std::sync::Arc;
use cold_clear_2::bot::BotConfig;
use cold_clear_2::try_create_bot;
use cold_clear_2::data::{Board, Piece};
use cold_clear_2::tbp::{Randomizer, Start};
use cold_clear_2::tetrio;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

const VISIBLE_QUEUE: usize = 6;
#[derive(Clone, Copy, Debug, Default)]
struct Metrics {
    pieces: u64, attack: u64, surge: u64, surge_releases: u64,
    max_b2b: u32, max_height: u32, topouts: u64,
}
impl Metrics {
    fn add(&mut self, m: Metrics) {
        self.pieces += m.pieces; self.attack += m.attack; self.surge += m.surge;
        self.surge_releases += m.surge_releases; self.topouts += m.topouts;
        self.max_b2b = self.max_b2b.max(m.max_b2b); self.max_height = self.max_height.max(m.max_height);
    }
    fn app(self) -> f64 { if self.pieces == 0 { 0.0 } else { self.attack as f64 / self.pieces as f64 } }
}
fn main() -> Result<(), String> {
    let seeds = arg_u64("--seeds", 10)?;
    let pieces = arg_u64("--pieces", 200)?;
    let nodes = arg_u64("--nodes", 2_000)?;
    if seeds == 0 || pieces == 0 || nodes == 0 { return Err("seeds, diagnostic length and node budget must be positive".into()); }
    println!("Draft S2 no-garbage diagnostic, NOT a win-rate or conformance test");
    println!("seeds={seeds} pieces/seed={pieces} node_budget/move={nodes}");
    println!("Both start with empty hold and current + NEXT x5. Legacy is the legacy evaluator on the modified core.\n");
    let mut legacy = Metrics::default();
    let mut s2 = Metrics::default();
    for seed in 0..seeds {
        legacy.add(run(seed, pieces, nodes, BotConfig::legacy())?);
        s2.add(run(seed, pieces, nodes, BotConfig::default())?);
    }
    print_metrics("legacy", legacy, seeds);
    print_metrics("s2-draft", s2, seeds);
    println!("\ndelta s2 - legacy\nAPP: {:+.4}\npieces survived: {:+}\ntopouts: {:+}", s2.app()-legacy.app(), s2.pieces as i64-legacy.pieces as i64, s2.topouts as i64-legacy.topouts as i64);
    Ok(())
}
fn print_metrics(name: &str, m: Metrics, seeds: u64) {
    println!("{name}\n  pieces: {}\n  attack: {}\n  APP: {:.4}\n  surge released: {} in {} releases\n  max B2B: {}\n  max height: {}\n  topouts: {}/{}", m.pieces, m.attack, m.app(), m.surge, m.surge_releases, m.max_b2b, m.max_height, m.topouts, seeds);
}
fn run(seed: u64, target: u64, budget: u64, config: BotConfig) -> Result<Metrics, String> {
    let sequence = piece_sequence(seed, target as usize + VISIBLE_QUEUE + 2);
    let start = Start { board: Board::default(), queue: sequence[..VISIBLE_QUEUE].to_vec(), hold: None, combo: 0, back_to_back: false, b2b_count: 0, randomizer: Randomizer::Unknown };
    let mut bot = try_create_bot(start, Arc::new(config))?;
    let mut next_piece = VISIBLE_QUEUE;
    let mut m = Metrics::default();
    for _ in 0..target {
        if bot.preview_refill_needed() != 0 { return Err("incomplete player-visible queue".into()); }
        let mut searched = 0;
        while searched < budget {
            let stats = bot.do_work();
            searched += stats.nodes;
            if stats.nodes == 0 { break; }
        }
        let suggestion = bot.suggest();
        let mv = match suggestion.first() {
            Some(&mv) => mv,
            None if bot.has_legal_move() => return Err("search failure is not a topout".into()),
            None => { m.topouts += 1; break; }
        };
        let info = bot.try_advance(mv)?;
        let attack = tetrio::attack(&info);
        m.pieces += 1;
        m.attack += attack.total as u64;
        m.surge += attack.surge_released as u64;
        m.surge_releases += u64::from(attack.surge_released > 0);
        m.max_b2b = m.max_b2b.max(info.b2b_count_after);
        m.max_height = m.max_height.max(bot.state().board.cols.iter().map(|&c| 64-c.leading_zeros()).max().unwrap_or(0));
        let refill = bot.preview_refill_needed();
        for _ in 0..refill {
            let piece = *sequence.get(next_piece).ok_or("diagnostic sequence unexpectedly exhausted")?;
            bot.new_piece(piece);
            next_piece += 1;
        }
    }
    Ok(m)
}
fn piece_sequence(seed: u64, len: usize) -> Vec<Piece> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        let mut bag = [Piece::I, Piece::O, Piece::T, Piece::L, Piece::J, Piece::S, Piece::Z];
        bag.shuffle(&mut rng); out.extend_from_slice(&bag);
    }
    out.truncate(len); out
}
fn arg_u64(name: &str, default: u64) -> Result<u64, String> {
    let mut args = env::args();
    while let Some(arg) = args.next() {
        if arg == name { return args.next().ok_or_else(|| format!("missing {name} value"))?.parse().map_err(|_| format!("invalid {name} value")); }
    }
    Ok(default)
}
