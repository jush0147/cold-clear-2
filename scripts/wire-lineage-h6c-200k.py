from pathlib import Path

src = Path("src/bin/strategy_h6c.rs")
out = Path("src/bin/strategy_lineage_h6c.rs")
s = src.read_text()

s = s.replace(
    "//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.",
    "//! 200k lineage revalidation: H6C versus H2 on corrected H12 core.",
    1,
)

old = """    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.row_transitions *= strategy.row_transition_scale;
"""
new = """    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.row_transitions *= strategy.row_transition_scale;
    config.freestyle_weights.h9_cavity_excavation = 0.0;
    config.dag_backprop_best_demotion = true;
    config.dag_backprop_despeculated_values = false;
"""
if old not in s:
    raise SystemExit("H6C lineage config anchor not found")
s = s.replace(old, new, 1)

s = s.replace(
    '"experiment": "H6C row-transition shaping scale",',
    '"experiment": "200k lineage revalidation H6C vs H2",',
    1,
)
s = s.replace(
    '"incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",',
    '"incumbent": "H2 + H12: pending safety 1, useful attack 1, row-transition scale 1.0, cavity 0",',
    1,
)
old = """        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
        "speculate": true,
"""
new = """        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
        "fixed_h9_cavity_excavation": 0.0,
        "fixed_h12_best_child_demotion": true,
        "speculate": true,
"""
if old not in s:
    raise SystemExit("H6C protocol metadata anchor not found")
s = s.replace(old, new, 1)
s = s.replace('"H6C A/A paired game not repeatable"', '"200k H6C lineage A/A paired game not repeatable"')

for needle in [
    "config.freestyle_weights.pending_safety = 1.0;",
    "config.freestyle_weights.useful_attack_reward = 1.0;",
    "config.freestyle_weights.row_transitions *= strategy.row_transition_scale;",
    "config.freestyle_weights.h9_cavity_excavation = 0.0;",
    "config.dag_backprop_best_demotion = true;",
    "config.dag_backprop_despeculated_values = false;",
]:
    if needle not in s:
        raise SystemExit(f"missing H6C lineage invariant: {needle}")

out.write_text(s)
print(out)
