from pathlib import Path


def replace(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    actual = text.count(old)
    assert actual == count, (path, old[:100], actual)
    p.write_text(text.replace(old, new))


# H2 adds two zero-default reward knobs. H1 and every existing configuration
# remain unchanged unless these fields are explicitly set by the H2 harness.
replace(
    'src/bot/freestyle.rs',
    '''    #[serde(default)]
    pub pending_safety: f32,
    pub cell_coveredness: f32,''',
    '''    #[serde(default)]
    pub pending_safety: f32,
    /// H2: reward garbage that remains after cancellation and is actually sent.
    #[serde(default)]
    pub useful_attack_reward: f32,
    /// H2: independently reward visible incoming garbage actually cancelled.
    #[serde(default)]
    pub cancellation_reward: f32,
    pub cell_coveredness: f32,'''
)

replace(
    'src/bot/freestyle.rs',
    '''                        new_stats.nodes += 1;
                        let mut state = state;
                        let info = state.advance(next, mv);

                        let (eval, reward) =
                            evaluate(&options.config.freestyle_weights, state, &info, sd_distance);''',
    '''                        new_stats.nodes += 1;
                        let mut state = state;
                        let incoming_before = state.forecast.remaining();
                        let sent_before = state.forecast.sent;
                        let info = state.advance(next, mv);

                        let (eval, reward) = evaluate(
                            &options.config.freestyle_weights,
                            state,
                            &info,
                            sd_distance,
                            incoming_before,
                            sent_before,
                        );'''
)

replace(
    'src/bot/freestyle.rs',
    '''fn evaluate(
    weights: &Weights,
    mut state: GameState,
    info: &PlacementInfo,
    softdrop: u32,
) -> (Eval, Reward) {''',
    '''fn evaluate(
    weights: &Weights,
    mut state: GameState,
    info: &PlacementInfo,
    softdrop: u32,
    incoming_before: u32,
    sent_before: u32,
) -> (Eval, Reward) {'''
)

replace(
    'src/bot/freestyle.rs',
    '''    } else {
        reward += legacy_shape;
    }

    if info.placement.location.piece == Piece::T''',
    '''    } else {
        reward += legacy_shape;
    }

    // H2 is deliberately orthogonal to the older tetrio_s2 switch. The
    // incumbent keeps the complete legacy shaping plus H1. Candidate changes
    // only these two coefficients. With a forecast enabled, a line clear cannot
    // raise garbage, so queue shrinkage on that transition is cancellation.
    let raw_attack = tetrio::attack(info).total;
    let (useful_outgoing, cancelled) = useful_attack_delta(
        state.forecast.enabled,
        info.lines_cleared,
        raw_attack,
        incoming_before,
        state.forecast.remaining(),
        sent_before,
        state.forecast.sent,
    );
    reward += weights.useful_attack_reward * useful_outgoing as f32;
    reward += weights.cancellation_reward * cancelled as f32;

    if info.placement.location.piece == Piece::T'''
)

replace(
    'src/bot/freestyle.rs',
    '''fn well_known_tslot_left(board: &Board) -> Option<PieceLocation> {''',
    '''fn useful_attack_delta(
    forecast_enabled: bool,
    lines_cleared: u32,
    raw_attack: u32,
    incoming_before: u32,
    incoming_after: u32,
    sent_before: u32,
    sent_after: u32,
) -> (u32, u32) {
    if lines_cleared == 0 {
        return (0, 0);
    }
    if forecast_enabled {
        (
            sent_after.saturating_sub(sent_before),
            incoming_before.saturating_sub(incoming_after),
        )
    } else {
        (raw_attack, 0)
    }
}

#[cfg(test)]
mod h2_tests {
    use super::useful_attack_delta;

    #[test]
    fn h2_separates_outgoing_from_cancelled_lines() {
        assert_eq!(useful_attack_delta(true, 2, 7, 5, 0, 10, 12), (2, 5));
        assert_eq!(useful_attack_delta(true, 2, 3, 8, 5, 4, 4), (0, 3));
    }

    #[test]
    fn h2_does_not_mistake_garbage_rise_for_cancellation() {
        assert_eq!(useful_attack_delta(true, 0, 0, 8, 0, 0, 0), (0, 0));
    }

    #[test]
    fn raw_attack_is_used_when_no_forecast_is_attached() {
        assert_eq!(useful_attack_delta(false, 4, 6, 0, 0, 0, 0), (6, 0));
    }
}

fn well_known_tslot_left(board: &Board) -> Option<PieceLocation> {'''
)

H2 = r'''//! KO-only H2 A/B: H1 incumbent versus useful outgoing/cancellation rewards.
use std::{collections::VecDeque, env, sync::Arc, time::Instant};
use cold_clear_2::{bot::{Bot, BotConfig, Statistics}, data::{Board, Piece, Placement},
    forecast::Forecast, ko_support::{cancel_plan, with_search_seed}, tbp::{Start, Randomizer},
    tetrio::{self, garbage::GarbagePacket}, try_create_bot};
use enumset::EnumSet;
use rand::{rngs::StdRng, Rng, SeedableRng, seq::SliceRandom};
use serde_json::{json, Value};

const PREVIEW: usize = 6;

#[derive(Clone, Copy, Debug, Default)]
struct Strategy { attack: f32, cancel: f32 }

#[derive(Clone, Copy)]
struct Settings { seeds: u64, start: u64, nodes: u64, strategy: Strategy }

fn settings(args: &[String]) -> Result<Settings, String> {
    let mut s = Settings {
        seeds: 10,
        start: 3000,
        nodes: 10000,
        strategy: Strategy::default(),
    };
    if args.len() % 2 != 0 { return Err("expected flag/value pairs".into()); }
    for p in args.chunks_exact(2) {
        match p[0].as_str() {
            "--seeds" => s.seeds = p[1].parse().map_err(|_| "invalid seeds")?,
            "--start" => s.start = p[1].parse().map_err(|_| "invalid start")?,
            "--nodes" => s.nodes = p[1].parse().map_err(|_| "invalid nodes")?,
            "--attack" => s.strategy.attack = p[1].parse().map_err(|_| "invalid attack reward")?,
            "--cancel" => s.strategy.cancel = p[1].parse().map_err(|_| "invalid cancellation reward")?,
            _ => return Err(format!("unknown flag {}; no piece cap or tiebreak", p[0])),
        }
    }
    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.attack.is_finite() || s.strategy.attack < 0.0
        || !s.strategy.cancel.is_finite() || s.strategy.cancel < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H2 rewards required".into());
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
    // H1 is the frozen incumbent for BOTH sides in H2.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = strategy.attack;
    config.freestyle_weights.cancellation_reward = strategy.cancel;
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
    let incumbent = Strategy::default();
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
        "attack_reward": s.strategy.attack,
        "cancellation_reward": s.strategy.cancel,
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
        "experiment": "H2 useful outgoing attack and cancellation",
        "incumbent": "H1 pending_safety=1.0; legacy evaluator otherwise unchanged",
        "candidate_attack_reward": s.strategy.attack,
        "candidate_cancellation_reward": s.strategy.cancel,
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
            if s.strategy.attack == 0.0 && s.strategy.cancel == 0.0 {
                if let Some(t) = &control_trace {
                    if t != &game["trace_hash"] { return Err("H1/H1 paired game not repeatable".into()); }
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
            vec!["--attack", "NaN"],
            vec!["--cancel", "-0.1"],
        ] {
            assert!(settings(&args.iter().map(|x| x.to_string()).collect::<Vec<_>>()).is_err());
        }
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
'''

Path('src/bin/strategy_h2.rs').write_text(H2)
