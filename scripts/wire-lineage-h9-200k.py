from pathlib import Path

src = Path("src/bin/strategy_h6c.rs")
out = Path("src/bin/strategy_lineage_h9.rs")
s = src.read_text()

s = s.replace(
    "//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.",
    "//! 200k lineage revalidation: promoted H9 versus H6C on corrected H12 core.",
    1,
)

old = """#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { row_transition_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { row_transition_scale: 1.0 } }
}
"""
new = """#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { cavity_excavation: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { cavity_excavation: 0.0 } }
}
"""
if old not in s:
    raise SystemExit("strategy block not found")
s = s.replace(old, new, 1)

old = """            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
"""
new = """            "--cavity-excavation" => s.strategy.cavity_excavation = p[1].parse().map_err(|_| "invalid cavity excavation weight")?,
            "--incumbent-cavity-excavation" => s.incumbent.cavity_excavation = p[1].parse().map_err(|_| "invalid incumbent cavity excavation weight")?,
"""
if old not in s:
    raise SystemExit("cli block not found")
s = s.replace(old, new, 1)

old = """    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C row-transition scales required".into());
"""
new = """    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.cavity_excavation.is_finite()
        || !s.incumbent.cavity_excavation.is_finite()
    {
        return Err("positive seeds, >=1000 nodes and finite H9 cavity weights required".into());
"""
if old not in s:
    raise SystemExit("validation block not found")
s = s.replace(old, new, 1)

old = """    // H1 + H2 winner are frozen for BOTH sides in H6C. Rejected H3/H4/H5/H6/H6B changes stay off.
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
"""
new = """    // High-compute lineage revalidation freezes H1 + H2 + H6C and the
    // corrected H12 DAG on BOTH sides. H9 cavity weight is the only variable.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    config.freestyle_weights.h6_base_holes_scale = 1.0;
    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.row_transitions *= 2.5;
    config.freestyle_weights.h9_cavity_excavation = strategy.cavity_excavation;
    config.dag_backprop_best_demotion = true;
    config.dag_backprop_despeculated_values = false;
"""
if old not in s:
    raise SystemExit("make_player block not found")
s = s.replace(old, new, 1)

old = """        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
"""
new = """        "cavity_excavation": s.strategy.cavity_excavation,
        "incumbent_cavity_excavation": s.incumbent.cavity_excavation,
        "fixed_row_transition_scale": 2.5,
        "fixed_h12_best_child_demotion": true,
"""
if old not in s:
    raise SystemExit("game metadata block not found")
s = s.replace(old, new, 1)

s = s.replace(
    '"experiment": "H6C row-transition shaping scale",',
    '"experiment": "200k lineage revalidation H9 vs H6C",',
    1,
)
s = s.replace(
    '"incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",',
    '"incumbent": "H6C + H12: pending safety 1, useful attack 1, row-transition scale 2.5, cavity 0",',
    1,
)

old = """        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
"""
new = """        "candidate_cavity_excavation": s.strategy.cavity_excavation,
        "incumbent_cavity_excavation": s.incumbent.cavity_excavation,
        "fixed_row_transition_scale": 2.5,
        "fixed_useful_attack_reward": 1.0,
        "fixed_h12_best_child_demotion": true,
"""
if old not in s:
    raise SystemExit("protocol metadata block not found")
s = s.replace(old, new, 1)

s = s.replace('"H6C A/A paired game not repeatable"', '"200k H9 lineage A/A paired game not repeatable"')

old = """            vec!["--row-transition-scale", "NaN"],
            vec!["--row-transition-scale", "-0.1"],
            vec!["--incumbent-row-transition-scale", "NaN"],
            vec!["--incumbent-row-transition-scale", "-0.1"],
"""
new = """            vec!["--cavity-excavation", "NaN"],
            vec!["--incumbent-cavity-excavation", "NaN"],
"""
if old not in s:
    raise SystemExit("parser rejection block not found")
s = s.replace(old, new, 1)

old = """    fn accepts_direct_h6c_vs_h6c_configuration() {
        let args = vec![
            "--row-transition-scale", "0.5",
            "--incumbent-row-transition-scale", "1.5",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { row_transition_scale: 0.5 });
        assert_eq!(s.incumbent, Strategy { row_transition_scale: 1.5 });
    }
"""
new = """    fn accepts_direct_h9_vs_h6c_configuration() {
        let args = vec![
            "--cavity-excavation", "-0.5",
            "--incumbent-cavity-excavation", "0",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { cavity_excavation: -0.5 });
        assert_eq!(s.incumbent, Strategy { cavity_excavation: 0.0 });
    }
"""
if old not in s:
    raise SystemExit("direct config test block not found")
s = s.replace(old, new, 1)

required = [
    "config.freestyle_weights.pending_safety = 1.0;",
    "config.freestyle_weights.useful_attack_reward = 1.0;",
    "config.freestyle_weights.row_transitions *= 2.5;",
    "config.freestyle_weights.h9_cavity_excavation = strategy.cavity_excavation;",
    "config.dag_backprop_best_demotion = true;",
    "config.dag_backprop_despeculated_values = false;",
    '"experiment": "200k lineage revalidation H9 vs H6C"',
]
for needle in required:
    if needle not in s:
        raise SystemExit(f"missing lineage invariant: {needle}")
if "strategy.row_transition_scale" in s:
    raise SystemExit("H6C row scale must be frozen, not variable")

out.write_text(s)
print(out)
