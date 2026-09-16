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

const PREVIEW: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Legacy,
    S2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    P0,
    P1,
    Draw,
}

#[derive(Default)]
struct Score {
    s2_wins: u64,
    legacy_wins: u64,
    draws: u64,
}

struct Player {
    bot: Bot,
    incoming: VecDeque<usize>,
    next_piece: usize,
    pieces: u64,
    attack: u64,
}

fn main() {
    let seeds = arg_u64("--seeds", 20);
    let max_pieces = arg_u64("--pieces", 200);
    let nodes = arg_u64("--nodes", 500);
    let garbage_cap = arg_u64("--garbage-cap", 8) as usize;
    let charge_value = arg_f32("--charge", 1.0);
    let surge_value = arg_f32("--surge", 1.0);

    println!("TL S2 paired duel benchmark (simplified timing)");
    println!("seeds={seeds} max_pieces/player={max_pieces} node_budget/move={nodes} garbage_cap={garbage_cap} charge={charge_value:.2} surge={surge_value:.2}");
    println!("Each seed is played twice with sides swapped. Incoming garbage can be cancelled on the next move, then uncancelled garbage rises.\n");

    let mut score = Score::default();
    for seed in 0..seeds {
        record(
            &mut score,
            duel(seed, Kind::Legacy, Kind::S2, max_pieces, nodes, garbage_cap, charge_value, surge_value),
            Kind::Legacy,
            Kind::S2,
        );
        record(
            &mut score,
            duel(seed, Kind::S2, Kind::Legacy, max_pieces, nodes, garbage_cap, charge_value, surge_value),
            Kind::S2,
            Kind::Legacy,
        );
    }

    let decisive = score.s2_wins + score.legacy_wins;
    println!("S2 wins: {}", score.s2_wins);
    println!("legacy wins: {}", score.legacy_wins);
    println!("draws: {}", score.draws);
    if decisive > 0 {
        println!("S2 decisive win rate: {:.1}%", 100.0 * score.s2_wins as f64 / decisive as f64);
    }
    println!("\nThis is an engineering benchmark, not an exact TETR.IO server simulation: it omits real-time PPS/travel delay, Surge packet timing, first-14-piece double-cancel, garbage-special +1, and Clutch Clears.");
}

fn record(score: &mut Score, outcome: Outcome, p0: Kind, p1: Kind) {
    match outcome {
        Outcome::Draw => score.draws += 1,
        Outcome::P0 => match p0 {
            Kind::S2 => score.s2_wins += 1,
            Kind::Legacy => score.legacy_wins += 1,
        },
        Outcome::P1 => match p1 {
            Kind::S2 => score.s2_wins += 1,
            Kind::Legacy => score.legacy_wins += 1,
        },
    }
}

fn duel(seed: u64, p0_kind: Kind, p1_kind: Kind, max_pieces: u64, nodes: u64, garbage_cap: usize, charge_value: f32, surge_value: f32) -> Outcome {
    let sequence = piece_sequence(seed, max_pieces as usize + PREVIEW + 8);
    let mut players = [
        make_player(&sequence, p0_kind, charge_value, surge_value),
        make_player(&sequence, p1_kind, charge_value, surge_value),
    ];

    for round in 0..max_pieces {
        for active in 0..2 {
            let other = 1 - active;
            let result = play_turn(
                &mut players,
                active,
                other,
                seed,
                round,
                nodes,
                garbage_cap,
                &sequence,
            );
            if result {
                return if active == 0 { Outcome::P1 } else { Outcome::P0 };
            }
        }
    }

    match players[0].attack.cmp(&players[1].attack) {
        std::cmp::Ordering::Greater => Outcome::P0,
        std::cmp::Ordering::Less => Outcome::P1,
        std::cmp::Ordering::Equal => Outcome::Draw,
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
    sequence: &[Piece],
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
    players[active].pieces += 1;
    players[active].attack += attack as u64;

    let cancel = attack.min(players[active].incoming.len());
    for _ in 0..cancel {
        players[active].incoming.pop_front();
    }
    let outgoing = attack - cancel;

    for i in 0..outgoing {
        players[other]
            .incoming
            .push_back(garbage_hole(seed, active as u64, round, i as u64));
    }

    let rise = garbage_cap.min(players[active].incoming.len());
    for _ in 0..rise {
        let hole = players[active].incoming.pop_front().unwrap();
        if players[active].bot.add_garbage_line(hole) {
            return true;
        }
    }

    if players[active].next_piece < sequence.len() {
        let p = sequence[players[active].next_piece];
        players[active].bot.new_piece(p);
        players[active].next_piece += 1;
    }

    false
}

fn make_player(sequence: &[Piece], kind: Kind, charge_value: f32, surge_value: f32) -> Player {
    let start = Start {
        board: Board::default(),
        queue: sequence[1..=PREVIEW].to_vec(),
        hold: Some(sequence[0]),
        combo: 0,
        back_to_back: false,
        b2b_count: 0,
        randomizer: Randomizer::Unknown,
    };
    let config = match kind {
        Kind::Legacy => BotConfig::legacy(),
        Kind::S2 => BotConfig::tetrio_s2(charge_value, surge_value),
    };
    Player {
        bot: create_bot(start, Arc::new(config)),
        incoming: VecDeque::new(),
        next_piece: PREVIEW + 1,
        pieces: 0,
        attack: 0,
    }
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

fn garbage_hole(seed: u64, sender: u64, round: u64, line: u64) -> usize {
    let mut x = seed
        ^ sender.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ round.wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ line.wrapping_mul(0x94D0_49BB_1331_11EB);
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
