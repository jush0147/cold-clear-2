from pathlib import Path

src = Path('src/bin/strategy_h5.rs')
out = Path('src/bin/strategy_h7.rs')
s = src.read_text()

s = s.replace(
    '//! KO-only H5 A/B: frozen H2 incumbent versus legacy combo-continuation shaping scale.',
    '//! KO-only H7 A/B: frozen H6C incumbent versus legacy combo-continuation shaping scale.',
    1,
)
s = s.replace('H5 combo scales required', 'H7 combo scales required')
s = s.replace(
    '// H1 + H2 winner are frozen for BOTH sides in H5. Rejected H3/H4 changes stay off.\n',
    '// H1 + H2 + H6C winner are frozen for BOTH sides in H7. Rejected H3/H4/H5/H6/H6B changes stay off.\n',
    1,
)
needle = '''    config.freestyle_weights.h3_b2b_charge_value = 0.0;\n    config.freestyle_weights.h3_surge_bank_value = 0.0;\n    config.freestyle_weights.combo_attack *= strategy.combo_scale;\n'''
replacement = '''    config.freestyle_weights.h3_b2b_charge_value = 0.0;\n    config.freestyle_weights.h3_surge_bank_value = 0.0;\n    config.freestyle_weights.h6_base_holes_scale = 1.0;\n    config.freestyle_weights.h6_base_coveredness_scale = 1.0;\n    config.freestyle_weights.row_transitions *= 2.5;\n    config.freestyle_weights.combo_attack *= strategy.combo_scale;\n'''
if needle not in s:
    raise SystemExit('expected H5 config block not found')
s = s.replace(needle, replacement, 1)
s = s.replace('"experiment": "H5 combo shaping scale"', '"experiment": "H7 combo interaction on H6C"')
s = s.replace(
    '"incumbent": "H2 winner with legacy combo scale 1.0; H1/H2 frozen; rejected H3/H4 disabled"',
    '"incumbent": "H6C winner: H1/H2 frozen, row-transition scale 2.5, legacy combo scale 1.0"',
)
# Keep CLI shape identical to H5 so the experiment changes only the frozen base policy.
# Guard against accidentally forgetting the defining H6C setting.
for required in [
    'config.freestyle_weights.pending_safety = 1.0;',
    'config.freestyle_weights.useful_attack_reward = 1.0;',
    'config.freestyle_weights.row_transitions *= 2.5;',
    'config.freestyle_weights.combo_attack *= strategy.combo_scale;',
]:
    if required not in s:
        raise SystemExit(f'missing H7 invariant: {required}')

out.write_text(s)
