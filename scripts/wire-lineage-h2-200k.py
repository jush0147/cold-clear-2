from pathlib import Path

src = Path("src/bin/strategy_h2.rs")
out = Path("src/bin/strategy_lineage_h2.rs")
s = src.read_text()

s = s.replace(
    "//! KO-only H2 A/B: H1 incumbent versus useful outgoing/cancellation rewards.",
    "//! 200k lineage revalidation: H2 versus H1 on corrected H12 core.",
    1,
)

old = """    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = strategy.attack;
    config.freestyle_weights.cancellation_reward = strategy.cancel;
"""
new = """    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = strategy.attack;
    config.freestyle_weights.cancellation_reward = strategy.cancel;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    config.freestyle_weights.h6_base_holes_scale = 1.0;
    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.h9_cavity_excavation = 0.0;
    config.dag_backprop_best_demotion = true;
    config.dag_backprop_despeculated_values = false;
"""
if old not in s:
    raise SystemExit("H2 lineage config anchor not found")
s = s.replace(old, new, 1)

s = s.replace(
    '"experiment": "H2 useful outgoing attack and cancellation",',
    '"experiment": "200k lineage revalidation H2 vs H1",',
    1,
)
s = s.replace(
    '"incumbent": "H1 pending_safety=1.0 plus configured H2 rewards",',
    '"incumbent": "H1 + H12: pending safety 1, useful attack 0, cancellation 0",',
    1,
)
old = """        "incumbent_cancellation_reward": s.incumbent.cancel,
        "speculate": true,
"""
new = """        "incumbent_cancellation_reward": s.incumbent.cancel,
        "fixed_row_transition_scale": 1.0,
        "fixed_h9_cavity_excavation": 0.0,
        "fixed_h12_best_child_demotion": true,
        "speculate": true,
"""
if old not in s:
    raise SystemExit("H2 protocol metadata anchor not found")
s = s.replace(old, new, 1)
s = s.replace('"H1/H1 paired game not repeatable"', '"200k H2 lineage A/A paired game not repeatable"')

for needle in [
    "config.freestyle_weights.pending_safety = 1.0;",
    "config.freestyle_weights.useful_attack_reward = strategy.attack;",
    "config.freestyle_weights.cancellation_reward = strategy.cancel;",
    "config.freestyle_weights.h9_cavity_excavation = 0.0;",
    "config.dag_backprop_best_demotion = true;",
    "config.dag_backprop_despeculated_values = false;",
]:
    if needle not in s:
        raise SystemExit(f"missing H2 lineage invariant: {needle}")

out.write_text(s)
print(out)
