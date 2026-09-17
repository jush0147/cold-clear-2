from pathlib import Path

src = Path('src/bin/strategy_h9.rs')
out = Path('src/bin/strategy_h10.rs')
s = src.read_text()

s = s.replace(
    '//! KO-only H9 A/B: frozen H6C incumbent versus sealed-cavity excavation cost.',
    '//! KO-only H10 A/B: promoted H9 incumbent versus search exploitation/exploration.',
    1,
)

old = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { cavity_excavation: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { cavity_excavation: 0.0 } }
}
'''
new = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { exploitation: f64 }
impl Default for Strategy {
    fn default() -> Self { Self { exploitation: std::f64::consts::LN_2 } }
}
'''
if old not in s:
    raise SystemExit('H9 Strategy block not found')
s = s.replace(old, new, 1)

old = '''            "--cavity-excavation" => s.strategy.cavity_excavation = p[1].parse().map_err(|_| "invalid cavity excavation weight")?,
            "--incumbent-cavity-excavation" => s.incumbent.cavity_excavation = p[1].parse().map_err(|_| "invalid incumbent cavity excavation weight")?,
'''
new = '''            "--exploitation" => s.strategy.exploitation = p[1].parse().map_err(|_| "invalid exploitation")?,
            "--incumbent-exploitation" => s.incumbent.exploitation = p[1].parse().map_err(|_| "invalid incumbent exploitation")?,
'''
if old not in s:
    raise SystemExit('H9 CLI block not found')
s = s.replace(old, new, 1)

old = '''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.cavity_excavation.is_finite()
        || !s.incumbent.cavity_excavation.is_finite()
    {
        return Err("positive seeds, >=1000 nodes and finite H9 cavity weights required".into());
'''
new = '''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.exploitation.is_finite() || s.strategy.exploitation <= 0.0
        || !s.incumbent.exploitation.is_finite() || s.incumbent.exploitation <= 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite positive H10 exploitation values required".into());
'''
if old not in s:
    raise SystemExit('H9 settings validation block not found')
s = s.replace(old, new, 1)

old = '''    // Freeze the promoted H6C incumbent on both sides. H9 changes only the
    // sealed-cavity excavation term; all rejected H3/H4/H5/H6/H6B/H7/H8 ideas stay off.
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
'''
new = '''    // Freeze the promoted H9 evaluator on both sides. H10 changes only the
    // DAG child-selection exploitation parameter.
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
    config.freestyle_exploitation = strategy.exploitation;
'''
if old not in s:
    raise SystemExit('H9 make_player block not found')
s = s.replace(old, new, 1)

old = '''        "cavity_excavation": s.strategy.cavity_excavation,
        "incumbent_cavity_excavation": s.incumbent.cavity_excavation,
        "fixed_row_transition_scale": 2.5,
'''
new = '''        "exploitation": s.strategy.exploitation,
        "incumbent_exploitation": s.incumbent.exploitation,
        "fixed_row_transition_scale": 2.5,
        "fixed_h9_cavity_excavation": -0.5,
'''
if old not in s:
    raise SystemExit('H9 game metadata block not found')
s = s.replace(old, new, 1)

s = s.replace(
    '"experiment": "H9 sealed-cavity excavation cost",',
    '"experiment": "H10 search exploitation/exploration",',
    1,
)
s = s.replace(
    '"incumbent": "H6C: pending safety 1.0, useful attack 1.0, row-transition scale 2.5, combo 1.0; H9 cavity weight 0",',
    '"incumbent": "H9 promoted evaluator: pending safety 1.0, useful attack 1.0, row-transition scale 2.5, combo 1.0, cavity excavation -0.5; default exploitation ln(2)",',
    1,
)
old = '''        "candidate_cavity_excavation": s.strategy.cavity_excavation,
        "incumbent_cavity_excavation": s.incumbent.cavity_excavation,
        "fixed_row_transition_scale": 2.5,
        "fixed_useful_attack_reward": 1.0,
'''
new = '''        "candidate_exploitation": s.strategy.exploitation,
        "incumbent_exploitation": s.incumbent.exploitation,
        "fixed_row_transition_scale": 2.5,
        "fixed_useful_attack_reward": 1.0,
        "fixed_h9_cavity_excavation": -0.5,
'''
if old not in s:
    raise SystemExit('H9 protocol metadata block not found')
s = s.replace(old, new, 1)
s = s.replace('"H9 A/A paired game not repeatable"', '"H10 A/A paired game not repeatable"')

old = '''            vec!["--cavity-excavation", "NaN"],
            vec!["--incumbent-cavity-excavation", "NaN"],
'''
new = '''            vec!["--exploitation", "NaN"],
            vec!["--exploitation", "0"],
            vec!["--exploitation", "-0.1"],
            vec!["--incumbent-exploitation", "NaN"],
            vec!["--incumbent-exploitation", "0"],
'''
if old not in s:
    raise SystemExit('H9 parser rejection tests not found')
s = s.replace(old, new, 1)

old = '''    fn accepts_direct_h9_vs_h6c_configuration() {
        let args = vec![
            "--cavity-excavation", "-0.75",
            "--incumbent-cavity-excavation", "0",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { cavity_excavation: -0.75 });
        assert_eq!(s.incumbent, Strategy { cavity_excavation: 0.0 });
    }
'''
new = '''    fn accepts_direct_h10_configuration() {
        let args = vec![
            "--exploitation", "0.5",
            "--incumbent-exploitation", "0.6931471805599453",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { exploitation: 0.5 });
        assert_eq!(s.incumbent, Strategy { exploitation: std::f64::consts::LN_2 });
    }
'''
if old not in s:
    raise SystemExit('H9 direct configuration test not found')
s = s.replace(old, new, 1)

required = [
    'config.freestyle_weights.pending_safety = 1.0;',
    'config.freestyle_weights.useful_attack_reward = 1.0;',
    'config.freestyle_weights.row_transitions *= 2.5;',
    'config.freestyle_weights.h9_cavity_excavation = -0.5;',
    'config.freestyle_exploitation = strategy.exploitation;',
    'struct Strategy { exploitation: f64 }',
]
for needle in required:
    if needle not in s:
        raise SystemExit(f'missing H10 invariant: {needle}')
if 'strategy.cavity_excavation' in s or 'combo_attack *=' in s:
    raise SystemExit('H10 must freeze H9 evaluator and legacy combo scale')

out.write_text(s)
