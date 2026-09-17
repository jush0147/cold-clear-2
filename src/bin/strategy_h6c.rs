//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.
use std::{collections::VecDeque, env, sync::Arc, time::Instant};
use cold_clear_2::{bot::{Bot, BotConfig, Statistics}, data::{Board, Piece, Placement},
    forecast::Forecast, ko_support::{cancel_plan, with_search_seed}, tbp::{Start, Randomizer},
    tetrio::{self, garbage::GarbagePacket}, try_create_bot};
use enumset::EnumSet;
use rand::{rngs::StdRng, Rng, SeedableRng, seq::SliceRandom};
use serde_json::{json, Value};

const PREVIEW: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { row_transition_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { row_transition_scale: 1.0 } }
}

#[derive(Clone, Copy)]
struct Settings { seeds: u64, start: u64, nodes: u64, strategy: Strategy, incumbent: Strategy }

fn settings(args: &[String]) -> Result<Settings, String> {
    let mut s = Settings {
        seeds: 10,
        start: 3000,
        nodes: 10000,
        strategy: Strategy::default(),
        incumbent: Strategy::default(),
    };
    if args.len() % 2 != 0 { return Err("expected flag/value pairs".into()); }
    for p in args.chunks_exact(2) {
        match p[0].as_str() {
            "--seeds" => s.seeds = p[1].parse().map_err(|_| "invalid seeds")?,
            "--start" => s.start = p[1].parse().map_err(|_| "invalid start")?,
            "--nodes" => s.nodes = p[1].parse().map_err(|_| "invalid nodes")?,
            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
            _ => return Err(format!("unknown flag {}; no piece cap or tiebreak", p[0])),
        }
    }
    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C row-transition scales required".into());
    }
    s.start.checked_add(s.seeds).ok_or("seed range overflow")?;
    Ok(s)
}

struct Sequence { rng: StdRng, pieces: Vec<Piece> }
impl Sequence {
    fn new(seed: u64) -> Self { Self { rng: StdRng::seed_from_u64(seed), pieces: vec![] } }
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
struct Packet { lines: u32, hole: usize }

struct Player {
    bot: Bot,
    incoming: VecDeque<Packet>,
    revealed: usize,
    pieces: u32,
    sent: u32,
    stats: Statistics,
    unused: u64,
    pressure_turns: u64,
}

fn make_player(visible: Vec<Piece>, strategy: Strategy) -> Result<Player, String> {
    // Only the six revealed pieces are used to deduce the current bag remainder.
    let mut bag = EnumSet::all();
    for &p in &visible { bag.remove(p); }
    let start = Start {
        board: Board::default(),
        queue: visible,
        hold: None,
        combo: 0,
        back_to_back: false,
        b2b_count: 0,
        randomizer: Randomizer::SevenBag { bag_state: bag },
    };
    let mut config = BotConfig::legacy();
    // H1 + H2 winner are frozen for BOTH sides in H6C. Rejected H3/H4/H5/H6/H6B changes stay off.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    // H6B is explicitly reset. Only the always-on row-transition term changes.
    config.freestyle_weights.h6_base_holes_scale = 1.0;
    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.row_transitions *= strategy.row_transition_scale;
    Ok(Player {
        bot: try_create_bot(start, Arc::new(config))?,
        incoming: VecDeque::new(),
        revealed: PREVIEW,
        pieces: 0,
        sent: 0,
        stats: Statistics::default(),
        unused: 0,
        pressure_turns: 0,
    })
}

fn observable(incoming: &VecDeque<Packet>) -> Vec<GarbagePacket> {
    incoming.iter().map(|p| GarbagePacket { lines: p.lines, active: true }).collect()
}

/// No opponent state, true future sequence, authority hole RNG or future attack
/// can enter this function.
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
                    if stop { break; }
                }
            },
        );
        if stats.nodes > allocation { return Err("node allocation exceeded".into()); }
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
                let value = here.iter().find(|(m, _)| m == mv)
                    .ok_or("missing common root action")?.1;
                *score += value as f64;
            }
        }
    }
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| move_key(a.0).cmp(&move_key(b.0))));
    let mv = ranked[0].0;
    bot.set_forecast(Forecast::default());
    if bot.state() != before || bot.player_pieces() != visible {
        return Err("search mutated authority state".into());
    }
    Ok((mv, total))
}

fn move_key(m: Placement) -> (u8, i8, i8, u8, u8) {
    (m.location.piece as u8, m.location.x, m.location.y, m.location.rotation as u8, m.spin as u8)
}

fn consume(q: &mut VecDeque<Packet>, mut n: u32) {
    while n > 0 {
        let p = q.front_mut().expect("validated cancellation count");
        let k = n.min(p.lines);
        n -= k;
        p.lines -= k;
        if p.lines == 0 { q.pop_front(); }
    }
}

fn duel(seed: u64, swapped: bool, s: Settings) -> Result<Value, String> {
    let mut sequence = Sequence::new(seed);
    let visible: Vec<_> = (0..PREVIEW).map(|i| sequence.get(i)).collect();
    let incumbent = s.incumbent;
    let strategies = if swapped {
        [s.strategy, incumbent]
    } else {
        [incumbent, s.strategy]
    };
    let mut players = [
        make_player(visible.clone(), strategies[0])?,
        make_player(visible, strategies[1])?,
    ];
    let mut holes = [
        StdRng::seed_from_u64(seed ^ 0xA2E1_937F),
        StdRng::seed_from_u64(seed ^ 0x5BCA_903D),
    ];
    let mut digest = 0xcbf29ce484222325u64;
    let started = Instant::now();

    let (loser, reason) = 'game: loop {
        for active in 0..2 {
            if !players[active].bot.has_legal_move() {
                break 'game (active, "no_legal_placement");
            }
            let p = &mut players[active];
            let observed = observable(&p.incoming);
            if !observed.is_empty() { p.pressure_turns += 1; }
            let (mv, stats) = choose(&mut p.bot, &observed, p.pieces, p.sent, s.nodes)?;
            p.unused += s.nodes - stats.nodes;
            p.stats.accumulate(stats);
            let k = move_key(mv);
            for n in [active as u64, k.0 as u64, k.1 as u64, k.2 as u64, k.3 as u64, k.4 as u64] {
                digest = (digest ^ n).wrapping_mul(0x100000001b3);
            }

            let info = p.bot.try_advance(mv)?;
            let mut outgoing = vec![];
            for attack in tetrio::attack(&info).packets() {
                let pending = p.incoming.iter().map(|p| p.lines).sum();
                let (cancelled, sent) = cancel_plan(attack, pending, p.pieces, p.sent);
                consume(&mut p.incoming, cancelled);
                p.sent = p.sent.checked_add(sent).ok_or("sent counter overflow")?;
                if sent != 0 {
                    outgoing.push(Packet { lines: sent, hole: holes[active].gen_range(0..10) });
                }
            }
            p.pieces = p.pieces.checked_add(1).ok_or("piece counter overflow")?;

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
            if !(1..=2).contains(&refill) { return Err("invalid hold/preview consumption".into()); }
            for _ in 0..refill {
                p.bot.new_piece(sequence.get(p.revealed));
                p.revealed += 1;
            }
            if p.bot.player_pieces().next.len() != 5 { return Err("NEXT visibility leak".into()); }
            players[1 - active].incoming.extend(outgoing);
        }
    };

    let winner = 1 - loser;
    let candidate_slot = usize::from(!swapped);
    Ok(json!({
        "type": "game",
        "seed": seed,
        "swapped": swapped,
        "winner": if winner == candidate_slot { "candidate" } else { "incumbent" },
        "winner_slot": winner,
        "reason": reason,
        "pieces": [players[0].pieces, players[1].pieces],
        "node_budget": s.nodes,
        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
        "nodes": [players[0].stats.nodes, players[1].stats.nodes],
        "unused_nodes": [players[0].unused, players[1].unused],
        "pressure_turns": [players[0].pressure_turns, players[1].pressure_turns],
        "max_search_depth": [players[0].stats.max_depth, players[1].stats.max_depth],
        "speculative_expansions": [players[0].stats.speculative_expansions, players[1].stats.speculative_expansions],
        "trace_hash": format!("{digest:016x}"),
        "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
    }))
}

fn main() -> Result<(), String> {
    let s = settings(&env::args().skip(1).collect::<Vec<_>>())?;
    println!("{}", json!({
        "type": "protocol",
        "experiment": "H6C row-transition shaping scale",
        "incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",
        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
        "speculate": true,
        "information": "own current/hold/next5/board/b2b/combo/observable incoming/history only",
        "arena": "shared zero-gravity turn-based S2 authority; no human PPS model",
        "seed_start": s.start,
        "seed_count": s.seeds,
        "nodes_per_move": s.nodes,
        "result_rule": "KO only; errors/timeouts uncompleted, never a win or draw",
    }));

    let (mut candidate, mut incumbent) = (0, 0);
    for seed in s.start..s.start + s.seeds {
        let mut control_trace: Option<Value> = None;
        for swapped in [false, true] {
            let game = duel(seed, swapped, s)
                .map_err(|e| format!("ABORT seed={seed} swapped={swapped}: {e}; no winner assigned"))?;
            if s.strategy == s.incumbent {
                if let Some(t) = &control_trace {
                    if t != &game["trace_hash"] { return Err("H6C A/A paired game not repeatable".into()); }
                }
                control_trace = Some(game["trace_hash"].clone());
            }
            if game["winner"] == "candidate" { candidate += 1; } else { incumbent += 1; }
            println!("{game}");
        }
    }
    println!("{}", json!({
        "type": "summary",
        "completed_games": candidate + incumbent,
        "candidate_wins": candidate,
        "incumbent_wins": incumbent,
        "all_pairs_complete": true,
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_caps_negative_and_nan_rewards() {
        for args in [
            vec!["--pieces", "120"],
            vec!["--nodes", "0"],
            vec!["--row-transition-scale", "NaN"],
            vec!["--row-transition-scale", "-0.1"],
            vec!["--incumbent-row-transition-scale", "NaN"],
            vec!["--incumbent-row-transition-scale", "-0.1"],
        ] {
            assert!(settings(&args.iter().map(|x| x.to_string()).collect::<Vec<_>>()).is_err());
        }
    }

    #[test]
    fn accepts_direct_h6c_vs_h6c_configuration() {
        let args = vec![
            "--row-transition-scale", "0.5",
            "--incumbent-row-transition-scale", "1.5",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { row_transition_scale: 0.5 });
        assert_eq!(s.incumbent, Strategy { row_transition_scale: 1.5 });
    }

    #[test]
    fn hidden_holes_do_not_enter_observation() {
        let a = VecDeque::from([Packet { lines: 7, hole: 0 }]);
        let b = VecDeque::from([Packet { lines: 7, hole: 9 }]);
        assert_eq!(serde_json::to_value(observable(&a)).unwrap(), serde_json::to_value(observable(&b)).unwrap());
    }

    #[test]
    fn five_next_does_not_limit_search_depth() {
        let mut seq = Sequence::new(7);
        let visible = (0..6).map(|i| seq.get(i)).collect();
        let p = make_player(visible, Strategy::default()).unwrap();
        assert_eq!(p.bot.player_pieces().next.len(), 5);
        let mut stats = Statistics::default();
        with_search_seed(43, || for _ in 0..1000 { stats.accumulate(p.bot.do_work()); });
        assert!(stats.max_depth > 6);
        assert!(stats.speculative_expansions > 0);
    }

    #[test]
    fn hard_budget_and_authority_unchanged() {
        let mut seq = Sequence::new(1);
        let visible = (0..6).map(|i| seq.get(i)).collect();
        let mut p = make_player(visible, Strategy::default()).unwrap();
        let (_, stats) = choose(&mut p.bot, &[], 0, 0, 1003).unwrap();
        assert_eq!(stats.nodes, 1003);
        assert!(!p.bot.state().forecast.enabled);
    }

    #[test]
    fn authority_and_forecast_agree() {
        for pieces in [0, 13, 14, 40] {
            for attack in 0..10 {
                let incoming = [GarbagePacket { lines: 11, active: true }];
                let mut f = Forecast::snapshot(&incoming, pieces, 2, 4).unwrap();
                let mut board = Board::default();
                f.resolve(&mut board, &[attack], 0);
                let (cancelled, sent) = cancel_plan(attack, 11, pieces, 2);
                let remain = 11 - cancelled;
                let rise = remain.min(8);
                assert_eq!(f.remaining(), remain - rise);
                assert_eq!(f.sent, 2 + sent);
                assert_eq!(board.cols[0], (1 << rise) - 1);
                assert_eq!(board.cols[4], 0);
            }
        }
    }
}
