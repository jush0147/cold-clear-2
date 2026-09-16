use std::collections::VecDeque;
use std::env;
use std::sync::Arc;

use cold_clear_2::bot::{Bot, BotConfig};
use cold_clear_2::create_bot;
use cold_clear_2::data::{Board, Piece};
use cold_clear_2::tbp::{Randomizer, Start};
use cold_clear_2::tetrio;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

const VISIBLE_NEXT: usize = 5;
const VISIBLE_QUEUE: usize = 1 + VISIBLE_NEXT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Legacy,
    S2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    P0,
    P1,
}

#[derive(Default)]
struct Score {
    s2_wins: u64,
    legacy_wins: u64,
}

struct Player {
    bot: Bot,
    incoming: VecDeque<usize>,
    next_piece: usize,
}

struct PieceSequence {
    rng: StdRng,
    pieces: Vec<Piece>,
}

impl PieceSequence {
    fn new(seed: u64) -> Self {
        let mut sequence = Self {
            rng: StdRng::seed_from_u64(seed),
            pieces: Vec::new(),
        };
        sequence.ensure(VISIBLE_QUEUE);
        sequence
    }

    fn ensure(&mut self, len: usize) {
        let base = [Piece::I, Piece::O, Piece::T, Piece::L, Piece::J, Piece::S, Piece::Z];
        while self.pieces.len() < len {
            let mut bag = base;
            bag.shuffle(&mut self.rng);
            self.pieces.extend_from_slice(&bag);
        }
    }

    fn get(&mut self, index: usize) -> Piece {
        self.ensure(index + 1);
        self.pieces[index]
    }
}

fn main() {
    let seeds = arg_u64("--seeds", 20);
    let nodes = arg_u64("--nodes", 500);
    let garbage_cap = arg_u64("--garbage-cap", 8) as usize;
    let charge_value = arg_f32("--charge", 0.5);
    let surge_value = arg_f32("--surge", 1.0);
    let shape_value = arg_f32("--shape", 0.5);

    println!("TL S2 paired duel benchmark (KO-only, simplified timing)");
    println!("seeds={seeds} node_budget/move={nodes} garbage_cap={garbage_cap} charge={charge_value:.2} surge={surge_value:.2} shape={shape_value:.2}");
    println!("Visibility: board + active piece + hold + five NEXT pieces + combo/B2B state. Hidden future pieces are not given to the bot.");
    println!("Each seed is played twice with sides swapped. A game ends only when one bot KOs; there is no piece cap or attack tiebreak.\n");

    let mut score = Score::default();
    for seed in 0..seeds {
        record(
            &mut score,
            duel(
                seed,
                Kind::Legacy,
                Kind::S2,
                nodes,
                garbage_cap,
                charge_value,
                surge_value,
                shape_value,
            ),
            Kind::Legacy,
            Kind::S2,
        );
        record(
            &mut score,
            duel(
                seed,
                Kind::S2,
                Kind::Legacy,
                nodes,
                garbage_cap,
                charge_value,
                surge_value,
                shape_value,
            ),
            Kind::S2,
            Kind::Legacy,
        );
    }

    let decisive = score.s2_wins + score.legacy_wins;
    println!("S2 wins: {}", score.s2_wins);
    println!("legacy wins: {}", score.legacy_wins);
    if decisive > 0 {
        println!(
            "S2 win rate: {:.1}%",
            100.0 * score.s2_wins as f64 / decisive as f64
        );
    }
    println!("\nThis remains an engineering benchmark, not an exact TETR.IO server simulation: incoming garbage is treated as immediately cancelable/active and Clutch Clears are not modeled yet.");
}

fn record(score: &mut Score, outcome: Outcome, p0: Kind, p1: Kind) {
    let winner = match outcome {
        Outcome::P0 => p0,
        Outcome::P1 => p1,
    };
    match winner {
        Kind::S2 => score.s2_wins += 1,
        Kind::Legacy => score.legacy_wins += 1,
    }
}

fn duel(
    seed: u64,
    p0_kind: Kind,
    p1_kind: Kind,
    nodes: u64,
    garbage_cap: usize,
    charge_value: f32,
    surge_value: f32,
    shape_value: f32,
) -> Outcome {
    let mut sequence = PieceSequence::new(seed);
    let initial = sequence.pieces[..VISIBLE_QUEUE].to_vec();
    let mut players = [
        make_player(&initial, p0_kind, charge_value, surge_value, shape_value),
        make_player(&initial, p1_kind, charge_value, surge_value, shape_value),
    ];

    let mut round = 0u64;
    loop {
        for active in 0..2 {
            let other = 1 - active;
            if play_turn(
                &mut players,
                active,
                other,
                seed,
                round,
                nodes,
                garbage_cap,
                &mut sequence,
            ) {
                return if active == 0 { Outcome::P1 } else { Outcome::P0 };
            }
        }
        round = round.wrapping_add(1);
    }
}

fn play_turn(
    players: &mut [Player; 2],
    active: usize,
    other: usize,
    seed: u64,
    round: u64,
    node_budget: u64,
    garbage_cap: usize,
    sequence: &mut PieceSequence,
) -> bool {
    let mut searched = 0;
    while searched < node_budget {
        let stats = players[active].bot.do_work();
        searched += stats.nodes;
        if stats.nodes == 0 {
            break;
        }
    }

    let Some(&mv) = players[active].bot.suggest().first() else {
        return true;
    };
    let info = players[active].bot.advance(mv);
    let attack = tetrio::attack(&info).total as usize;

    let cancel = attack.min(players[active].incoming.len());
    for _ in 0..cancel {
        players[active].incoming.pop_front();
    }
    let outgoing = attack - cancel;
    if outgoing > 0 {
        // TETR.IO garbage is change-on-attack: one attack is clean rather than
        // choosing a new hole independently for every line.
        let hole = garbage_hole(seed, active as u64, round, 0);
        for _ in 0..outgoing {
            players[other].incoming.push_back(hole);
        }
    }

    // Default combo blocking: line clears keep garbage in the queue. Garbage
    // rises on a placement that does not clear a line. Timing/activation is
    // still simplified here; the replay-facing bot will receive that state
    // explicitly instead of pretending every queued line is already active.
    if info.lines_cleared == 0 {
        let rise = garbage_cap.min(players[active].incoming.len());
        for _ in 0..rise {
            let hole = players[active].incoming.pop_front().unwrap();
            if players[active].bot.add_garbage_line(hole) {
                return true;
            }
        }
    }

    let p = sequence.get(players[active].next_piece);
    players[active].bot.new_piece(p);
    players[active].next_piece += 1;
    false
}

fn make_player(
    visible_queue: &[Piece],
    kind: Kind,
    charge_value: f32,
    surge_value: f32,
    shape_value: f32,
) -> Player {
    debug_assert_eq!(visible_queue.len(), VISIBLE_QUEUE);
    let start = Start {
        board: Board::default(),
        // TBP queue includes the active piece. Hold is genuinely empty at the
        // start of a TL game, so do not pre-hold the first generated piece.
        queue: visible_queue.to_vec(),
        hold: None,
        combo: 0,
        back_to_back: false,
        b2b_count: 0,
        randomizer: Randomizer::Unknown,
    };
    let config = match kind {
        Kind::Legacy => BotConfig::legacy(),
        Kind::S2 => BotConfig::tetrio_s2(charge_value, surge_value, shape_value),
    };
    Player {
        bot: create_bot(start, Arc::new(config)),
        incoming: VecDeque::new(),
        next_piece: VISIBLE_QUEUE,
    }
}

fn garbage_hole(seed: u64, sender: u64, round: u64, packet: u64) -> usize {
    let mut x = seed
        ^ sender.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ round.wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ packet.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x % 10) as usize
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

fn arg_f32(name: &str, default: f32) -> f32 {
    let mut args = env::args();
    while let Some(arg) = args.next() {
        if arg == name {
            return args.next().and_then(|v| v.parse().ok()).unwrap_or(default);
        }
    }
    default
}
