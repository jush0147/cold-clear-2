//! Exploratory FT7 diagnostic: current best review evaluator vs corrected legacy.
//! APP is raw attack generated before cancellation divided by pieces placed.
//! This is diagnostic telemetry only; KO remains the only match outcome.

use std::{collections::VecDeque, env, sync::Arc, time::Instant};

use cold_clear_2::{
    bot::{Bot, BotConfig, Statistics},
    data::{Board, Piece, Placement},
    forecast::Forecast,
    ko_support::{cancel_plan, with_search_seed},
    tbp::{Randomizer, Start},
    tetrio::{self, garbage::GarbagePacket},
    try_create_bot,
};
use enumset::EnumSet;
use rand::{rngs::StdRng, Rng, SeedableRng, seq::SliceRandom};
use serde_json::{json, Value};

const PREVIEW: usize = 6;

#[derive(Clone, Copy)]
struct Settings {
    start: u64,
    target: u32,
    nodes: u64,
}

fn settings(args: &[String]) -> Result<Settings, String> {
    let mut s = Settings { start: 49000, target: 7, nodes: 200_000 };
    if args.len() % 2 != 0 {
        return Err("expected flag/value pairs".into());
    }
    for p in args.chunks_exact(2) {
        match p[0].as_str() {
            "--start" => s.start = p[1].parse().map_err(|_| "invalid start")?,
            "--target" => s.target = p[1].parse().map_err(|_| "invalid target")?,
            "--nodes" => s.nodes = p[1].parse().map_err(|_| "invalid nodes")?,
            _ => return Err(format!("unknown flag {}", p[0])),
        }
    }
    if s.target == 0 || s.nodes < 1000 {
        return Err("positive target and >=1000 nodes required".into());
    }
    Ok(s)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Profile {
    Best,
    Legacy,
}

fn config(profile: Profile) -> BotConfig {
    match profile {
        Profile::Best => BotConfig::review_h9_h12(),
        Profile::Legacy => {
            let mut c = BotConfig::legacy();
            c.freestyle_weights.softdrop = 0.0;
            c.freestyle_weights.pending_safety = 0.0;
            c.freestyle_weights.useful_attack_reward = 0.0;
            c.freestyle_weights.cancellation_reward = 0.0;
            c.freestyle_weights.h3_b2b_charge_value = 0.0;
            c.freestyle_weights.h3_surge_bank_value = 0.0;
            c.freestyle_weights.h6_base_holes_scale = 1.0;
            c.freestyle_weights.h6_base_coveredness_scale = 1.0;
            c.freestyle_weights.h9_cavity_excavation = 0.0;
            c.dag_backprop_best_demotion = true;
            c.dag_backprop_despeculated_values = false;
            c
        }
    }
}

struct Sequence {
    rng: StdRng,
    pieces: Vec<Piece>,
}
impl Sequence {
    fn new(seed: u64) -> Self {
        Self { rng: StdRng::seed_from_u64(seed), pieces: vec![] }
    }
    fn get(&mut self, index: usize) -> Piece {
        while self.pieces.len() <= index {
            let mut bag = [Piece::I, Piece::O, Piece::T, Piece::L, Piece::J, Piece::S, Piece::Z];
            bag.shuffle(&mut self.rng);
            self.pieces.extend(bag);
        }
        self.pieces[index]
    }
}

#[derive(Clone)]
struct Packet {
    lines: u32,
    hole: usize,
}

#[derive(Clone, Copy, Default)]
struct Totals {
    pieces: u64,
    raw_attack: u64,
    cancelled: u64,
    sent: u64,
}
impl Totals {
    fn add(&mut self, other: Self) {
        self.pieces += other.pieces;
        self.raw_attack += other.raw_attack;
        self.cancelled += other.cancelled;
        self.sent += other.sent;
    }
    fn raw_app(self) -> f64 {
        if self.pieces == 0 { 0.0 } else { self.raw_attack as f64 / self.pieces as f64 }
    }
    fn sent_app(self) -> f64 {
        if self.pieces == 0 { 0.0 } else { self.sent as f64 / self.pieces as f64 }
    }
}

struct Player {
    bot: Bot,
    incoming: VecDeque<Packet>,
    revealed: usize,
    totals: Totals,
    stats: Statistics,
    unused: u64,
    pressure_turns: u64,
}

fn make_player(visible: Vec<Piece>, profile: Profile) -> Result<Player, String> {
    let mut bag = EnumSet::all();
    for &p in &visible {
        bag.remove(p);
    }
    let start = Start {
        board: Board::default(),
        queue: visible,
        hold: None,
        combo: 0,
        back_to_back: false,
        b2b_count: 0,
        randomizer: Randomizer::SevenBag { bag_state: bag },
    };
    Ok(Player {
        bot: try_create_bot(start, Arc::new(config(profile)))?,
        incoming: VecDeque::new(),
        revealed: PREVIEW,
        totals: Totals::default(),
        stats: Statistics::default(),
        unused: 0,
        pressure_turns: 0,
    })
}

fn observable(incoming: &VecDeque<Packet>) -> Vec<GarbagePacket> {
    incoming
        .iter()
        .map(|p| GarbagePacket { lines: p.lines, active: true })
        .collect()
}

fn choose(
    bot: &mut Bot,
    incoming: &[GarbagePacket],
    pieces: u32,
    sent: u32,
    budget: u64,
) -> Result<(Placement, Statistics), String> {
    let before = bot.state();
    let visible = bot.player_pieces();
    if visible.current.is_none() || visible.next.len() != 5 {
        return Err("observation must have current + NEXT x5".into());
    }

    let scenarios = if incoming.is_empty() { 1 } else { 10 };
    let mut total = Statistics::default();
    let mut ranked: Vec<(Placement, f64)> = vec![];

    for scenario in 0..scenarios {
        bot.set_forecast(Forecast::snapshot(incoming, pieces, sent, scenario as u32)?);
        let allocation = budget / scenarios + u64::from(scenario < budget % scenarios);
        let mut stats = Statistics::default();
        let mut stalled = 0;

        with_search_seed(
            0xC01D_C1EAu64 ^ (pieces as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ scenario,
            || {
                while stats.nodes < allocation && stalled < 1024 {
                    let step = bot.do_work_limited(allocation - stats.nodes);
                    stalled = if step.nodes == 0 { stalled + 1 } else { 0 };
                    let stop = step.budget_exhausted;
                    stats.accumulate(step);
                    if stop {
                        break;
                    }
                }
            },
        );

        if stats.nodes > allocation {
            return Err("node allocation exceeded".into());
        }
        total.accumulate(stats);

        let here = bot.ranked_suggestions();
        if here.is_empty() {
            return Err("search produced no move despite legal authority state".into());
        }
        if scenario == 0 {
            ranked = here.iter().map(|&(m, v)| (m, v as f64)).collect();
        } else {
            if here.len() != ranked.len() {
                return Err("root action set changed before unknown holes revealed".into());
            }
            for (mv, score) in &mut ranked {
                let value = here
                    .iter()
                    .find(|(m, _)| m == mv)
                    .ok_or("missing common root action")?
                    .1;
                *score += value as f64;
            }
        }
    }

    ranked.sort_by(|a, b| {
        b.1.total_cmp(&a.1)
            .then_with(|| move_key(a.0).cmp(&move_key(b.0)))
    });
    let mv = ranked[0].0;

    bot.set_forecast(Forecast::default());
    if bot.state() != before || bot.player_pieces() != visible {
        return Err("search mutated authority state".into());
    }
    Ok((mv, total))
}

fn move_key(m: Placement) -> (u8, i8, i8, u8, u8) {
    (
        m.location.piece as u8,
        m.location.x,
        m.location.y,
        m.location.rotation as u8,
        m.spin as u8,
    )
}

fn consume(q: &mut VecDeque<Packet>, mut n: u32) {
    while n > 0 {
        let p = q.front_mut().expect("validated cancellation count");
        let k = n.min(p.lines);
        n -= k;
        p.lines -= k;
        if p.lines == 0 {
            q.pop_front();
        }
    }
}

struct RoundResult {
    json: Value,
    candidate_won: bool,
    candidate: Totals,
    legacy: Totals,
}

fn round(seed: u64, swapped: bool, nodes: u64) -> Result<RoundResult, String> {
    let mut sequence = Sequence::new(seed);
    let visible: Vec<_> = (0..PREVIEW).map(|i| sequence.get(i)).collect();

    // Alternate slots across rounds because the arena is turn-based rather than
    // truly simultaneous. This reduces fixed first-slot bias in a single FT set.
    let profiles = if swapped {
        [Profile::Best, Profile::Legacy]
    } else {
        [Profile::Legacy, Profile::Best]
    };
    let candidate_slot = usize::from(!swapped);

    let mut players = [
        make_player(visible.clone(), profiles[0])?,
        make_player(visible, profiles[1])?,
    ];
    let mut holes = [
        StdRng::seed_from_u64(seed ^ 0xA2E1_937F),
        StdRng::seed_from_u64(seed ^ 0x5BCA_903D),
    ];
    let started = Instant::now();

    let (loser, reason) = 'game: loop {
        for active in 0..2 {
            if !players[active].bot.has_legal_move() {
                break 'game (active, "no_legal_placement");
            }

            let p = &mut players[active];
            let observed = observable(&p.incoming);
            if !observed.is_empty() {
                p.pressure_turns += 1;
            }

            let (mv, stats) = choose(
                &mut p.bot,
                &observed,
                p.totals.pieces as u32,
                p.totals.sent as u32,
                nodes,
            )?;
            p.unused += nodes - stats.nodes;
            p.stats.accumulate(stats);

            let info = p.bot.try_advance(mv)?;
            let mut outgoing = vec![];

            for attack in tetrio::attack(&info).packets() {
                p.totals.raw_attack += u64::from(attack);
                let pending = p.incoming.iter().map(|p| p.lines).sum();
                let (cancelled, sent) = cancel_plan(
                    attack,
                    pending,
                    p.totals.pieces as u32,
                    p.totals.sent as u32,
                );
                p.totals.cancelled += u64::from(cancelled);
                consume(&mut p.incoming, cancelled);
                p.totals.sent += u64::from(sent);

                if sent != 0 {
                    outgoing.push(Packet {
                        lines: sent,
                        hole: holes[active].gen_range(0..10),
                    });
                }
            }
            p.totals.pieces += 1;

            if info.lines_cleared == 0 {
                for _ in 0..8 {
                    let hole = match p.incoming.front() {
                        Some(packet) => packet.hole,
                        None => break,
                    };
                    consume(&mut p.incoming, 1);
                    if p.bot.add_garbage_line(hole) {
                        break 'game (active, "garbage_overflow");
                    }
                }
            }

            let refill = p.bot.preview_refill_needed();
            if !(1..=2).contains(&refill) {
                return Err("invalid hold/preview consumption".into());
            }
            for _ in 0..refill {
                p.bot.new_piece(sequence.get(p.revealed));
                p.revealed += 1;
            }
            if p.bot.player_pieces().next.len() != 5 {
                return Err("NEXT visibility leak".into());
            }
            players[1 - active].incoming.extend(outgoing);
        }
    };

    let winner = 1 - loser;
    let candidate_won = winner == candidate_slot;
    let candidate = players[candidate_slot].totals;
    let legacy = players[1 - candidate_slot].totals;

    Ok(RoundResult {
        json: json!({
            "type": "round",
            "seed": seed,
            "swapped": swapped,
            "winner": if candidate_won { "candidate" } else { "legacy" },
            "reason": reason,
            "candidate": {
                "pieces": candidate.pieces,
                "raw_attack": candidate.raw_attack,
                "cancelled": candidate.cancelled,
                "sent": candidate.sent,
                "raw_app": candidate.raw_app(),
                "sent_app": candidate.sent_app(),
            },
            "legacy": {
                "pieces": legacy.pieces,
                "raw_attack": legacy.raw_attack,
                "cancelled": legacy.cancelled,
                "sent": legacy.sent,
                "raw_app": legacy.raw_app(),
                "sent_app": legacy.sent_app(),
            },
            "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        }),
        candidate_won,
        candidate,
        legacy,
    })
}

fn main() -> Result<(), String> {
    let s = settings(&env::args().skip(1).collect::<Vec<_>>())?;

    println!("{}", json!({
        "type": "protocol",
        "experiment": "exploratory FT7 APP diagnostic",
        "candidate": "BotConfig::review_h9_h12() = H1+H2+H6C+H9+H12",
        "legacy": "legacy evaluator on same corrected core, zero gravity, H12 on, H1/H2/H6C/H9 off",
        "nodes_per_move": s.nodes,
        "target_wins": s.target,
        "seed_start": s.start,
        "raw_app_definition": "sum raw tetrio::attack packets before cancellation / pieces placed",
        "sent_app_definition": "attack remaining after self-cancellation / pieces placed",
        "outcome": "KO only; APP is diagnostic and never a tiebreak",
        "slot_policy": "candidate/legacy slots alternate each round to reduce fixed turn-order bias",
        "information": "own current/hold/NEXT5/board/B2B/combo/observable incoming/history only",
        "arena": "same shared zero-gravity turn-based S2 authority as lineage KO tests; not an official simultaneous server simulation"
    }));

    let mut candidate_wins = 0u32;
    let mut legacy_wins = 0u32;
    let mut candidate_totals = Totals::default();
    let mut legacy_totals = Totals::default();
    let mut round_index = 0u64;

    while candidate_wins < s.target && legacy_wins < s.target {
        let r = round(s.start + round_index, round_index % 2 == 1, s.nodes)?;
        if r.candidate_won {
            candidate_wins += 1;
        } else {
            legacy_wins += 1;
        }
        candidate_totals.add(r.candidate);
        legacy_totals.add(r.legacy);
        println!("{}", r.json);
        round_index += 1;
    }

    println!("{}", json!({
        "type": "match_summary",
        "format": format!("FT{}", s.target),
        "rounds": round_index,
        "candidate_wins": candidate_wins,
        "legacy_wins": legacy_wins,
        "winner": if candidate_wins == s.target { "candidate" } else { "legacy" },
        "candidate": {
            "pieces": candidate_totals.pieces,
            "raw_attack": candidate_totals.raw_attack,
            "cancelled": candidate_totals.cancelled,
            "sent": candidate_totals.sent,
            "raw_app": candidate_totals.raw_app(),
            "sent_app": candidate_totals.sent_app(),
        },
        "legacy": {
            "pieces": legacy_totals.pieces,
            "raw_attack": legacy_totals.raw_attack,
            "cancelled": legacy_totals.cancelled,
            "sent": legacy_totals.sent,
            "raw_app": legacy_totals.raw_app(),
            "sent_app": legacy_totals.sent_app(),
        }
    }));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_profile_is_the_scored_h9_lineage() {
        let c = config(Profile::Best);
        let legacy = BotConfig::legacy();
        assert_eq!(c.freestyle_weights.pending_safety, 1.0);
        assert_eq!(c.freestyle_weights.useful_attack_reward, 1.0);
        assert_eq!(c.freestyle_weights.row_transitions, legacy.freestyle_weights.row_transitions * 2.5);
        assert_eq!(c.freestyle_weights.h9_cavity_excavation, -0.5);
        assert!(c.dag_backprop_best_demotion);
        assert!(!c.dag_backprop_despeculated_values);
    }

    #[test]
    fn diagnostic_legacy_uses_corrected_core_without_promoted_weights() {
        let c = config(Profile::Legacy);
        let legacy = BotConfig::legacy();
        assert_eq!(c.freestyle_weights.pending_safety, 0.0);
        assert_eq!(c.freestyle_weights.useful_attack_reward, 0.0);
        assert_eq!(c.freestyle_weights.row_transitions, legacy.freestyle_weights.row_transitions);
        assert_eq!(c.freestyle_weights.h9_cavity_excavation, 0.0);
        assert!(c.dag_backprop_best_demotion);
        assert!(!c.dag_backprop_despeculated_values);
    }

    #[test]
    fn app_is_attack_per_piece() {
        let t = Totals { pieces: 100, raw_attack: 93, cancelled: 8, sent: 85 };
        assert!((t.raw_app() - 0.93).abs() < 1e-12);
        assert!((t.sent_app() - 0.85).abs() < 1e-12);
    }
}
