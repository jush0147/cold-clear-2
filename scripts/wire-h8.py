from pathlib import Path

src = Path('src/bin/strategy_h6c.rs')
out = Path('src/bin/strategy_h8.rs')
s = src.read_text()

s = s.replace(
    '//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.',
    '//! KO-only H8 A/B: frozen H6C incumbent versus joint useful-attack and row-transition tuning.',
    1,
)

old = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { row_transition_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { row_transition_scale: 1.0 } }
}
'''
new = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy {
    row_transition_scale: f32,
    useful_attack_reward: f32,
}
impl Default for Strategy {
    fn default() -> Self {
        Self { row_transition_scale: 2.5, useful_attack_reward: 1.0 }
    }
}
'''
if old not in s:
    raise SystemExit('H6C Strategy block not found')
s = s.replace(old, new, 1)

old = '''            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
'''
new = '''            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
            "--useful-attack-reward" => s.strategy.useful_attack_reward = p[1].parse().map_err(|_| "invalid useful attack reward")?,
            "--incumbent-useful-attack-reward" => s.incumbent.useful_attack_reward = p[1].parse().map_err(|_| "invalid incumbent useful attack reward")?,
'''
if old not in s:
    raise SystemExit('H6C CLI block not found')
s = s.replace(old, new, 1)

old = '''        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C row-transition scales required".into());
'''
new = '''        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
        || !s.strategy.useful_attack_reward.is_finite() || s.strategy.useful_attack_reward < 0.0
        || !s.incumbent.useful_attack_reward.is_finite() || s.incumbent.useful_attack_reward < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H8 attack/row parameters required".into());
'''
if old not in s:
    raise SystemExit('H6C validation block not found')
s = s.replace(old, new, 1)

s = s.replace(
    '// H1 + H2 winner are frozen for BOTH sides in H6C. Rejected H3/H4/H5/H6/H6B changes stay off.',
    '// H8 jointly retunes the two promoted concepts on top of H1. Rejected H3/H4/H5/H6/H6B/H7 changes stay off.',
    1,
)
s = s.replace(
    '    config.freestyle_weights.useful_attack_reward = 1.0;',
    '    config.freestyle_weights.useful_attack_reward = strategy.useful_attack_reward;',
    1,
)
s = s.replace(
    '// H6B is explicitly reset. Only the always-on row-transition term changes.',
    '// H6B is explicitly reset. Only useful attack and the always-on row-transition term change.',
    1,
)

old = '''        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
        "useful_attack_reward": s.strategy.useful_attack_reward,
        "incumbent_useful_attack_reward": s.incumbent.useful_attack_reward,
'''
if old not in s:
    raise SystemExit('H6C game metadata block not found')
s = s.replace(old, new, 1)

s = s.replace(
    '"experiment": "H6C row-transition shaping scale",',
    '"experiment": "H8 joint useful-attack x row-transition tuning",',
    1,
)
s = s.replace(
    '"incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",',
    '"incumbent": "H6C winner: pending safety 1.0, useful attack 1.0, row-transition scale 2.5; rejected H3/H4/H5/H6/H6B/H7 disabled",',
    1,
)
old = '''        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
        "candidate_useful_attack_reward": s.strategy.useful_attack_reward,
        "incumbent_useful_attack_reward": s.incumbent.useful_attack_reward,
'''
if old not in s:
    raise SystemExit('H6C protocol metadata block not found')
s = s.replace(old, new, 1)
s = s.replace('"H6C A/A paired game not repeatable"', '"H8 A/A paired game not repeatable"')

# Upgrade inherited H6C parser tests to the two-dimensional H8 strategy.
s = s.replace(
    '            vec!["--incumbent-row-transition-scale", "-0.1"],\n',
    '            vec!["--incumbent-row-transition-scale", "-0.1"],\n'
    '            vec!["--useful-attack-reward", "NaN"],\n'
    '            vec!["--useful-attack-reward", "-0.1"],\n'
    '            vec!["--incumbent-useful-attack-reward", "NaN"],\n'
    '            vec!["--incumbent-useful-attack-reward", "-0.1"],\n',
    1,
)
s = s.replace(
    'fn accepts_direct_h6c_vs_h6c_configuration()',
    'fn accepts_direct_h8_vs_h8_configuration()',
    1,
)
s = s.replace(
    '        assert_eq!(s.strategy, Strategy { row_transition_scale: 0.5 });',
    '        assert_eq!(s.strategy, Strategy { row_transition_scale: 0.5, useful_attack_reward: 1.0 });',
    1,
)
s = s.replace(
    '        assert_eq!(s.incumbent, Strategy { row_transition_scale: 1.5 });',
    '        assert_eq!(s.incumbent, Strategy { row_transition_scale: 1.5, useful_attack_reward: 1.0 });',
    1,
)

# Hard guards: H8 must be exactly H1 + joint H2/H6C retuning, with rejected ideas off.
required = [
    'config.freestyle_weights.softdrop = 0.0;',
    'config.freestyle_weights.pending_safety = 1.0;',
    'config.freestyle_weights.useful_attack_reward = strategy.useful_attack_reward;',
    'config.freestyle_weights.cancellation_reward = 0.0;',
    'config.freestyle_weights.h3_b2b_charge_value = 0.0;',
    'config.freestyle_weights.h3_surge_bank_value = 0.0;',
    'config.freestyle_weights.h6_base_holes_scale = 1.0;',
    'config.freestyle_weights.h6_base_coveredness_scale = 1.0;',
    'config.freestyle_weights.row_transitions *= strategy.row_transition_scale;',
    'Strategy { row_transition_scale: 0.5, useful_attack_reward: 1.0 }',
    'Strategy { row_transition_scale: 1.5, useful_attack_reward: 1.0 }',
]
for needle in required:
    if needle not in s:
        raise SystemExit(f'missing H8 invariant: {needle}')
if 'combo_attack *=' in s:
    raise SystemExit('H8 must keep legacy combo scale 1.0')

out.write_text(s)
