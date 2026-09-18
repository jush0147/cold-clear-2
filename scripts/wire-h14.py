from pathlib import Path

src = Path("src/bin/strategy_h6c.rs")
out = Path("src/bin/strategy_h14.rs")
s = src.read_text()

def replace(old, new, label):
    global s
    if old not in s:
        raise SystemExit(f"H14 anchor not found: {label}")
    s = s.replace(old, new, 1)

replace(
    "//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.",
    "//! KO-only H14 A/B: fixed H9+H12 strategy with unequal review compute budgets.",
    "header",
)

replace(
'''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { row_transition_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { row_transition_scale: 1.0 } }
}

#[derive(Clone, Copy)]
struct Settings { seeds: u64, start: u64, nodes: u64, strategy: Strategy, incumbent: Strategy }
''',
'''#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Strategy { nodes: u64 }
impl Default for Strategy {
    fn default() -> Self { Self { nodes: 10000 } }
}

#[derive(Clone, Copy)]
struct Settings { seeds: u64, start: u64, strategy: Strategy, incumbent: Strategy }
''',
    "strategy/settings",
)

replace(
'''        seeds: 10,
        start: 3000,
        nodes: 10000,
        strategy: Strategy::default(),
        incumbent: Strategy::default(),
''',
'''        seeds: 10,
        start: 47000,
        strategy: Strategy::default(),
        incumbent: Strategy::default(),
''',
    "settings defaults",
)

replace(
'''            "--seeds" => s.seeds = p[1].parse().map_err(|_| "invalid seeds")?,
            "--start" => s.start = p[1].parse().map_err(|_| "invalid start")?,
            "--nodes" => s.nodes = p[1].parse().map_err(|_| "invalid nodes")?,
            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
''',
'''            "--seeds" => s.seeds = p[1].parse().map_err(|_| "invalid seeds")?,
            "--start" => s.start = p[1].parse().map_err(|_| "invalid start")?,
            "--nodes" => s.strategy.nodes = p[1].parse().map_err(|_| "invalid candidate nodes")?,
            "--incumbent-nodes" => s.incumbent.nodes = p[1].parse().map_err(|_| "invalid incumbent nodes")?,
''',
    "CLI",
)

replace(
'''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C row-transition scales required".into());
''',
'''    if s.seeds == 0 || s.strategy.nodes < 1000 || s.incumbent.nodes < 1000 {
        return Err("positive seeds and >=1000 evaluator nodes per move on both sides required".into());
''',
    "validation",
)

replace(
'''fn make_player(visible: Vec<Piece>, strategy: Strategy) -> Result<Player, String> {
''',
'''fn make_player(visible: Vec<Piece>, _strategy: Strategy) -> Result<Player, String> {
''',
    "make_player signature",
)

replace(
'''    // H1 + H2 winner are frozen for BOTH sides in H6C. Rejected H3/H4/H5/H6/H6B changes stay off.
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
''',
'''    // Freeze the scored H9 evaluator plus the promoted H12 corrected DAG.
    // H14 changes only evaluator-node compute per move.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    config.freestyle_weights.h6_base_holes_scale = 1.0;
    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.row_transitions *= 2.5;
    config.freestyle_weights.h9_cavity_excavation = -0.5;
    config.dag_backprop_best_demotion = true;
''',
    "fixed H9+H12 config",
)

replace(
'''            let (mv, stats) = choose(&mut p.bot, &observed, p.pieces, p.sent, s.nodes)?;
            p.unused += s.nodes - stats.nodes;
''',
'''            let budget = strategies[active].nodes;
            let (mv, stats) = choose(&mut p.bot, &observed, p.pieces, p.sent, budget)?;
            p.unused += budget - stats.nodes;
''',
    "per-player budget",
)

replace(
'''        "node_budget": s.nodes,
        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
''',
'''        "candidate_node_budget": s.strategy.nodes,
        "incumbent_node_budget": s.incumbent.nodes,
        "slot_node_budgets": [strategies[0].nodes, strategies[1].nodes],
        "fixed_row_transition_scale": 2.5,
        "fixed_h9_cavity_excavation": -0.5,
        "fixed_backprop_best_demotion": true,
''',
    "game metadata",
)

replace(
'''        "experiment": "H6C row-transition shaping scale",
        "incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",
        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
''',
'''        "experiment": "H14 review compute scaling",
        "incumbent": "H9 promoted evaluator + H12 corrected DAG at 10000 evaluator nodes per move",
        "candidate_node_budget": s.strategy.nodes,
        "incumbent_node_budget": s.incumbent.nodes,
        "fixed_row_transition_scale": 2.5,
        "fixed_h9_cavity_excavation": -0.5,
        "fixed_backprop_best_demotion": true,
''',
    "protocol metadata",
)

replace(
'''        "nodes_per_move": s.nodes,
''',
'''        "nodes_per_move": {"candidate": s.strategy.nodes, "incumbent": s.incumbent.nodes},
''',
    "protocol node budget",
)

s = s.replace('"H6C A/A paired game not repeatable"', '"H14 A/A paired game not repeatable"')

replace(
'''    fn rejects_caps_negative_and_nan_rewards() {
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
''',
'''    fn rejects_caps_and_too_small_budgets() {
        for args in [
            vec!["--pieces", "120"],
            vec!["--nodes", "0"],
            vec!["--nodes", "999"],
            vec!["--incumbent-nodes", "0"],
            vec!["--incumbent-nodes", "999"],
        ] {
            assert!(settings(&args.iter().map(|x| x.to_string()).collect::<Vec<_>>()).is_err());
        }
    }

    #[test]
    fn accepts_direct_compute_configuration() {
        let args = vec![
            "--nodes", "50000",
            "--incumbent-nodes", "10000",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { nodes: 50000 });
        assert_eq!(s.incumbent, Strategy { nodes: 10000 });
    }
''',
    "tests",
)

required = [
    "config.freestyle_weights.pending_safety = 1.0;",
    "config.freestyle_weights.useful_attack_reward = 1.0;",
    "config.freestyle_weights.row_transitions *= 2.5;",
    "config.freestyle_weights.h9_cavity_excavation = -0.5;",
    "config.dag_backprop_best_demotion = true;",
    "let budget = strategies[active].nodes;",
    '"experiment": "H14 review compute scaling"',
]
for needle in required:
    if needle not in s:
        raise SystemExit(f"missing H14 invariant: {needle}")

if "strategy.row_transition_scale" in s or "s.nodes" in s:
    raise SystemExit("H14 must vary only per-side node budget")

out.write_text(s)
