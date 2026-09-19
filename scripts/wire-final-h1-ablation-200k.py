from pathlib import Path

src = Path("src/bin/strategy_h6c.rs")
out = Path("src/bin/strategy_final_h1_ablation.rs")
s = src.read_text()

s = s.replace(
    "//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.",
    "//! Final-profile H1 ablation at 200k: pending safety on versus off with H2/H6C/H9/H12 fixed.",
    1,
)

old = """#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { row_transition_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { row_transition_scale: 1.0 } }
}
"""
new = """#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { pending_safety: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { pending_safety: 0.0 } }
}
"""
if old not in s:
    raise SystemExit("strategy block not found")
s = s.replace(old, new, 1)

old = """            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
"""
new = """            "--pending-safety" => s.strategy.pending_safety = p[1].parse().map_err(|_| "invalid pending safety")?,
            "--incumbent-pending-safety" => s.incumbent.pending_safety = p[1].parse().map_err(|_| "invalid incumbent pending safety")?,
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
        || !s.strategy.pending_safety.is_finite() || s.strategy.pending_safety < 0.0
        || !s.incumbent.pending_safety.is_finite() || s.incumbent.pending_safety < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative pending-safety weights required".into());
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
new = """    // Final-profile ablation freezes H2 + H6C + H9 + H12 on both sides.
    // pending_safety is the only variable.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = strategy.pending_safety;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    config.freestyle_weights.h6_base_holes_scale = 1.0;
    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.row_transitions *= 2.5;
    config.freestyle_weights.h9_cavity_excavation = -0.5;
    config.dag_backprop_best_demotion = true;
    config.dag_backprop_despeculated_values = false;
"""
if old not in s:
    raise SystemExit("make_player block not found")
s = s.replace(old, new, 1)

old = """        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
"""
new = """        "pending_safety": s.strategy.pending_safety,
        "incumbent_pending_safety": s.incumbent.pending_safety,
        "fixed_useful_attack_reward": 1.0,
        "fixed_row_transition_scale": 2.5,
        "fixed_h9_cavity_excavation": -0.5,
        "fixed_h12_best_child_demotion": true,
"""
if old not in s:
    raise SystemExit("game metadata block not found")
s = s.replace(old, new, 1)

s = s.replace(
    '"experiment": "H6C row-transition shaping scale",',
    '"experiment": "final-profile H1 ablation at 200k",',
    1,
)
s = s.replace(
    '"incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",',
    '"incumbent": "H2+H6C+H9+H12 with pending_safety off",',
    1,
)

old = """        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
"""
new = """        "candidate_pending_safety": s.strategy.pending_safety,
        "incumbent_pending_safety": s.incumbent.pending_safety,
        "fixed_useful_attack_reward": 1.0,
        "fixed_row_transition_scale": 2.5,
        "fixed_h9_cavity_excavation": -0.5,
        "fixed_h12_best_child_demotion": true,
"""
if old not in s:
    raise SystemExit("protocol metadata block not found")
s = s.replace(old, new, 1)

s = s.replace('"H6C A/A paired game not repeatable"', '"final H1 ablation A/A paired game not repeatable"')

old = """            vec!["--row-transition-scale", "NaN"],
            vec!["--row-transition-scale", "-0.1"],
            vec!["--incumbent-row-transition-scale", "NaN"],
            vec!["--incumbent-row-transition-scale", "-0.1"],
"""
new = """            vec!["--pending-safety", "NaN"],
            vec!["--pending-safety", "-0.1"],
            vec!["--incumbent-pending-safety", "NaN"],
            vec!["--incumbent-pending-safety", "-0.1"],
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
new = """    fn accepts_direct_final_h1_ablation_configuration() {
        let args = vec![
            "--pending-safety", "1.0",
            "--incumbent-pending-safety", "0.0",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { pending_safety: 1.0 });
        assert_eq!(s.incumbent, Strategy { pending_safety: 0.0 });
    }
"""
if old not in s:
    raise SystemExit("direct config test block not found")
s = s.replace(old, new, 1)

for needle in [
    "config.freestyle_weights.pending_safety = strategy.pending_safety;",
    "config.freestyle_weights.useful_attack_reward = 1.0;",
    "config.freestyle_weights.row_transitions *= 2.5;",
    "config.freestyle_weights.h9_cavity_excavation = -0.5;",
    "config.dag_backprop_best_demotion = true;",
    "config.dag_backprop_despeculated_values = false;",
]:
    if needle not in s:
        raise SystemExit(f"missing final-profile invariant: {needle}")

out.write_text(s)
print(out)
