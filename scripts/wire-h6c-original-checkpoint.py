from pathlib import Path

src_path = Path("src/bin/strategy_h6c.rs")
out_path = Path("src/bin/strategy_checkpoint.rs")
s = src_path.read_text()

def once(old: str, new: str) -> None:
    global s
    n = s.count(old)
    if n != 1:
        raise SystemExit(f"expected exactly one occurrence, found {n}: {old[:100]!r}")
    s = s.replace(old, new)

once(
    "//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.",
    "//! KO-only checkpoint: promoted H6C profile versus original CC2 evaluator profile on shared corrected mechanics.",
)
once(
    "struct Strategy { row_transition_scale: f32 }",
    "struct Strategy { row_transition_scale: f32, pending_safety: f32, useful_attack_reward: f32 }",
)
once(
    "fn default() -> Self { Self { row_transition_scale: 1.0 } }",
    "fn default() -> Self { Self { row_transition_scale: 1.0, pending_safety: 1.0, useful_attack_reward: 1.0 } }",
)
once(
    '            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,\n            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,',
    '            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,\n            "--pending-safety" => s.strategy.pending_safety = p[1].parse().map_err(|_| "invalid pending safety")?,\n            "--useful-attack-reward" => s.strategy.useful_attack_reward = p[1].parse().map_err(|_| "invalid useful attack reward")?,\n            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,\n            "--incumbent-pending-safety" => s.incumbent.pending_safety = p[1].parse().map_err(|_| "invalid incumbent pending safety")?,\n            "--incumbent-useful-attack-reward" => s.incumbent.useful_attack_reward = p[1].parse().map_err(|_| "invalid incumbent useful attack reward")?,',
)
once(
    '''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C row-transition scales required".into());
    }''',
    '''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.strategy.pending_safety.is_finite() || s.strategy.pending_safety < 0.0
        || !s.strategy.useful_attack_reward.is_finite() || s.strategy.useful_attack_reward < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
        || !s.incumbent.pending_safety.is_finite() || s.incumbent.pending_safety < 0.0
        || !s.incumbent.useful_attack_reward.is_finite() || s.incumbent.useful_attack_reward < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative checkpoint profile weights required".into());
    }''',
)
once(
    "    // H1 + H2 winner are frozen for BOTH sides in H6C. Rejected H3/H4/H5/H6/H6B changes stay off.\n    config.freestyle_weights.softdrop = 0.0;\n    config.freestyle_weights.pending_safety = 1.0;\n    config.freestyle_weights.useful_attack_reward = 1.0;",
    "    // Strategy profile selects H1/H2/H6C values. Mechanics, zero gravity and rejected changes remain shared.\n    config.freestyle_weights.softdrop = 0.0;\n    config.freestyle_weights.pending_safety = strategy.pending_safety;\n    config.freestyle_weights.useful_attack_reward = strategy.useful_attack_reward;",
)
once(
    '''        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,''',
    '''        "row_transition_scale": s.strategy.row_transition_scale,
        "pending_safety": s.strategy.pending_safety,
        "useful_attack_reward": s.strategy.useful_attack_reward,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
        "incumbent_pending_safety": s.incumbent.pending_safety,
        "incumbent_useful_attack_reward": s.incumbent.useful_attack_reward,''',
)
once(
    '        "experiment": "H6C row-transition shaping scale",\n        "incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",\n        "candidate_row_transition_scale": s.strategy.row_transition_scale,\n        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,',
    '        "experiment": "H6C versus original CC2 evaluator checkpoint",\n        "baseline_definition": "legacy CC2 strategy/evaluator values under the same corrected S2 mechanics, information boundary, zero-gravity policy and arena; not byte-for-byte upstream binary",\n        "candidate_row_transition_scale": s.strategy.row_transition_scale,\n        "candidate_pending_safety": s.strategy.pending_safety,\n        "candidate_useful_attack_reward": s.strategy.useful_attack_reward,\n        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,\n        "incumbent_pending_safety": s.incumbent.pending_safety,\n        "incumbent_useful_attack_reward": s.incumbent.useful_attack_reward,',
)
once(
    '        assert_eq!(s.strategy, Strategy { row_transition_scale: 0.5 });\n        assert_eq!(s.incumbent, Strategy { row_transition_scale: 1.5 });',
    '        assert_eq!(s.strategy, Strategy { row_transition_scale: 0.5, pending_safety: 1.0, useful_attack_reward: 1.0 });\n        assert_eq!(s.incumbent, Strategy { row_transition_scale: 1.5, pending_safety: 1.0, useful_attack_reward: 1.0 });',
)

# Make the checkpoint-specific flags visibly covered by the argument-validation test.
once(
    '            vec!["--incumbent-row-transition-scale", "-0.1"],',
    '            vec!["--incumbent-row-transition-scale", "-0.1"],\n            vec!["--pending-safety", "NaN"],\n            vec!["--useful-attack-reward", "-0.1"],\n            vec!["--incumbent-pending-safety", "NaN"],\n            vec!["--incumbent-useful-attack-reward", "-0.1"],',
)

if "strategy.pending_safety" not in s or "strategy.useful_attack_reward" not in s:
    raise SystemExit("checkpoint strategy profile wiring missing")
if "pending_safety = 1.0;" in s or "useful_attack_reward = 1.0;" in s:
    raise SystemExit("stale hard-coded H1/H2 values remain in generated checkpoint harness")

out_path.write_text(s)
print(out_path)
