use std::env;
use std::sync::Arc;

use cold_clear_2::bot::BotConfig;
use cold_clear_2::create_bot;
use cold_clear_2::data::{Board, Piece};
use cold_clear_2::tbp::{Randomizer, Start};
use cold_clear_2::tetrio;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

// Active piece + five visible NEXT pieces. Hidden future bag contents are not
// exposed to the search; each turn reveals exactly one newly-visible preview.
const VISIBLE_QUEUE: usize = 6;

#[derive(Clone, Copy, Debug, Default)]
struct Metrics {
    pieces: u64,
    attack: u64,
    surge: u64,
    surge_releases: u64,
    max_b2b: u32,
    max_height: u32,
    topouts: u64,
}

impl Metrics {
    fn add(&mut self, other: Metrics) {
        self.pieces += other.pieces;
        self.attack += other.attack;
        self.surge += other.surge;
        self.surge_releases += other.surge_releases;
        self.max_b2b = self.max_b2b.max(other.max_b2b);
        self.max_height = self.max_height.max(other.max_height);
        self.topouts += other.topouts;
    }

    fn app(self) -> f64 {
        if self.pieces == 0 { 0.0 } else { self.attack as f64 / self.pieces as f64 }
    }
}

fn main() {
    let seeds = arg_u64("--seeds", 10);
    let pieces = arg_u64("--pieces", 200);
    let nodes = arg_u64("--nodes", 2_000);

    println!("TL S2 evaluator benchmark");
    println!("seeds={seeds} pieces/seed={pieces} node_budget/move={nodes}");
    println!("Visibility: board + active piece + hold + five NEXT pieces + combo/B2B state. Hidden future pieces are not given to the bot.");
    println!("No incoming garbage: this is an attack/survival health check, not a win-rate test.\n");

    let mut legacy = Metrics::default();
    let mut s2 = Metrics::default();

    for seed in 0..seeds {
        legacy.add(run(seed, pieces, nodes, BotConfig::legacy()));
        s2.add(run(seed, pieces, nodes, BotConfig::default()));
    }

    print_metrics("legacy", legacy, seeds);
    print_metrics("s2", s2, seeds);

    println!("\ndelta s2 - legacy");
    println!("APP: {:+.4}", s2.app() - legacy.app());
    println!("pieces survived: {:+}", s2.pieces as i64 - legacy.pieces as i64);
    println!("topouts: {:+}", s2.topouts as i64 - legacy.topouts as i64);
    println!("max B2B: {:+}", s2.max_b2b as i64 - legacy.max_b2b as i64);
    println!("max height: {:+}", s2.max_height as i64 - legacy.max_height as i64);
}

fn print_metrics(name: &str, m: Metrics, seeds: u64) {
    println!("{name}");
    println!("  pieces: {}", m.pieces);
    println!("  attack: {}", m.attack);
    println!("  APP: {:.4}", m.app());
    println!("  surge released: {} in {} releases", m.surge, m.surge_releases);
    println!("  max B2B: {}", m.max_b2b);
    println!("  max height: {}", m.max_height);
    println!("  topouts: {}/{}", m.topouts, seeds);
}

fn run(seed: u64, target_pieces: u64, node_budget: u64, config: BotConfig) -> Metrics {
    let sequence = piece_sequence(seed, target_pieces as usize + VISIBLE_QUEUE + 2);
    let start = Start {
        board: Board::default(),
        queue: sequence[1..=VISIBLE_QUEUE].to_vec(),
        hold: Some(sequence[0]),
        combo: 0,
        back_to_back: false,
        b2b_count: 0,
        randomizer: Randomizer::Unknown,
    };

    let mut bot = create_bot(start, Arc::new(config));
    let mut next_piece = VISIBLE_QUEUE + 1;
    let mut metrics = Metrics::default();

    for _ in 0..target_pieces {
        let mut searched = 0;
        while searched < node_budget {
            let stats = bot.do_work();
            searched += stats.nodes;
            if stats.nodes == 0 {
                break;
            }
        }

        let suggestion = bot.suggest();
        let Some(&mv) = suggestion.first() else {
            metrics.topouts += 1;
            break;
        };

        let info = bot.advance(mv);
        let attack = tetrio::attack(&info);
        metrics.pieces += 1;
        metrics.attack += attack.total as u64;
        metrics.surge += attack.surge_released as u64;
        metrics.surge_releases += u64::from(attack.surge_released > 0);
        metrics.max_b2b = metrics.max_b2b.max(info.b2b_count_after);
        metrics.max_height = metrics.max_height.max(board_height(&bot.state().board));

        if next_piece < sequence.len() {
            bot.new_piece(sequence[next_piece]);
            next_piece += 1;
        }
    }

    if metrics.pieces < target_pieces && metrics.topouts == 0 {
        metrics.topouts = 1;
    }
    metrics
}

fn piece_sequence(seed: u64, len: usize) -> Vec<Piece> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut out = Vec::with_capacity(len);
    let base = [Piece::I, Piece::O, Piece::T, Piece::L, Piece::J, Piece::S, Piece::Z];
    while out.len() < len {
        let mut bag = base;
        bag.shuffle(&mut rng);
        out.extend_from_slice(&bag);
    }
    out.truncate(len);
    out
}

fn board_height(board: &Board) -> u32 {
    board.cols.iter().map(|&c| 64 - c.leading_zeros()).max().unwrap_or(0)
}

fn arg_u64(name: &str, default: u64) -> u64 {
    let mut args = env::args();
    while let Some(arg) = args.next() {
        if arg == name {
            return args.next().and_then(|v| v.parse().ok()).unwrap_or(default);
        }
    }
    default
}
