from pathlib import Path


def replace(text, old, new, count=1):
    actual = text.count(old)
    assert actual == count, (old[:140], actual)
    return text.replace(old, new)

h = Path('src/bin/strategy_h3.rs').read_text()

h = replace(
    h,
    '//! KO-only H3 A/B: frozen H2 incumbent versus B2B/Surge inventory values.',
    '//! KO-only H4 A/B: frozen H2 incumbent versus legacy structured-attack setup bias scales.'
)
h = replace(
    h,
    '#[derive(Clone, Copy, Debug, Default)]\nstruct Strategy { charge: f32, surge_bank: f32 }',
    '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { tslot_scale: f32, well_scale: f32, wasted_t_scale: f32 }
impl Default for Strategy {
    fn default() -> Self {
        Self { tslot_scale: 1.0, well_scale: 1.0, wasted_t_scale: 1.0 }
    }
}'''
)
h = replace(
    h,
    '''            "--charge" => s.strategy.charge = p[1].parse().map_err(|_| "invalid B2B charge value")?,
            "--surge-bank" => s.strategy.surge_bank = p[1].parse().map_err(|_| "invalid Surge bank value")?,
            "--incumbent-charge" => s.incumbent.charge = p[1].parse().map_err(|_| "invalid incumbent B2B charge value")?,
            "--incumbent-surge-bank" => s.incumbent.surge_bank = p[1].parse().map_err(|_| "invalid incumbent Surge bank value")?,''',
    '''            "--tslot-scale" => s.strategy.tslot_scale = p[1].parse().map_err(|_| "invalid T-slot scale")?,
            "--well-scale" => s.strategy.well_scale = p[1].parse().map_err(|_| "invalid Quad-well scale")?,
            "--wasted-t-scale" => s.strategy.wasted_t_scale = p[1].parse().map_err(|_| "invalid wasted-T scale")?,
            "--incumbent-tslot-scale" => s.incumbent.tslot_scale = p[1].parse().map_err(|_| "invalid incumbent T-slot scale")?,
            "--incumbent-well-scale" => s.incumbent.well_scale = p[1].parse().map_err(|_| "invalid incumbent Quad-well scale")?,
            "--incumbent-wasted-t-scale" => s.incumbent.wasted_t_scale = p[1].parse().map_err(|_| "invalid incumbent wasted-T scale")?,'''
)
h = replace(
    h,
    '''        || !s.strategy.charge.is_finite() || s.strategy.charge < 0.0
        || !s.strategy.surge_bank.is_finite() || s.strategy.surge_bank < 0.0
        || !s.incumbent.charge.is_finite() || s.incumbent.charge < 0.0
        || !s.incumbent.surge_bank.is_finite() || s.incumbent.surge_bank < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H3 values required".into());''',
    '''        || !s.strategy.tslot_scale.is_finite() || s.strategy.tslot_scale < 0.0
        || !s.strategy.well_scale.is_finite() || s.strategy.well_scale < 0.0
        || !s.strategy.wasted_t_scale.is_finite() || s.strategy.wasted_t_scale < 0.0
        || !s.incumbent.tslot_scale.is_finite() || s.incumbent.tslot_scale < 0.0
        || !s.incumbent.well_scale.is_finite() || s.incumbent.well_scale < 0.0
        || !s.incumbent.wasted_t_scale.is_finite() || s.incumbent.wasted_t_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H4 scales required".into());'''
)
h = replace(
    h,
    '''    // H1 + H2 winner are frozen for BOTH sides in H3.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = strategy.charge;
    config.freestyle_weights.h3_surge_bank_value = strategy.surge_bank;''',
    '''    // H1 + H2 winner are frozen for BOTH sides in H4; rejected H3 values stay off.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    for value in &mut config.freestyle_weights.tslot { *value *= strategy.tslot_scale; }
    config.freestyle_weights.tetris_well_depth *= strategy.well_scale;
    config.freestyle_weights.wasted_t *= strategy.wasted_t_scale;'''
)
h = replace(
    h,
    '''        "h3_b2b_charge_value": s.strategy.charge,
        "h3_surge_bank_value": s.strategy.surge_bank,
        "incumbent_h3_b2b_charge_value": s.incumbent.charge,
        "incumbent_h3_surge_bank_value": s.incumbent.surge_bank,''',
    '''        "tslot_scale": s.strategy.tslot_scale,
        "well_scale": s.strategy.well_scale,
        "wasted_t_scale": s.strategy.wasted_t_scale,
        "incumbent_tslot_scale": s.incumbent.tslot_scale,
        "incumbent_well_scale": s.incumbent.well_scale,
        "incumbent_wasted_t_scale": s.incumbent.wasted_t_scale,'''
)
h = replace(
    h,
    '''        "experiment": "H3 B2B charge and Surge bank terminal value",
        "incumbent": "H1 pending_safety=1.0 + H2 useful_attack_reward=1.0,cancellation_reward=0",
        "candidate_h3_b2b_charge_value": s.strategy.charge,
        "candidate_h3_surge_bank_value": s.strategy.surge_bank,
        "incumbent_h3_b2b_charge_value": s.incumbent.charge,
        "incumbent_h3_surge_bank_value": s.incumbent.surge_bank,''',
    '''        "experiment": "H4 T-slot Quad-well and T-preservation shaping scales",
        "incumbent": "H2 winner with legacy setup scales 1.0/1.0/1.0 and rejected H3 disabled",
        "candidate_tslot_scale": s.strategy.tslot_scale,
        "candidate_well_scale": s.strategy.well_scale,
        "candidate_wasted_t_scale": s.strategy.wasted_t_scale,
        "incumbent_tslot_scale": s.incumbent.tslot_scale,
        "incumbent_well_scale": s.incumbent.well_scale,
        "incumbent_wasted_t_scale": s.incumbent.wasted_t_scale,'''
)
h = replace(
    h,
    'if s.strategy.charge == s.incumbent.charge && s.strategy.surge_bank == s.incumbent.surge_bank {',
    'if s.strategy == s.incumbent {'
)
h = replace(h, '"H3 A/A paired game not repeatable"', '"H4 A/A paired game not repeatable"')
h = replace(
    h,
    '''            vec!["--charge", "NaN"],
            vec!["--surge-bank", "-0.1"],
            vec!["--incumbent-charge", "NaN"],
            vec!["--incumbent-surge-bank", "-0.1"],''',
    '''            vec!["--tslot-scale", "NaN"],
            vec!["--well-scale", "-0.1"],
            vec!["--wasted-t-scale", "NaN"],
            vec!["--incumbent-tslot-scale", "-0.1"],
            vec!["--incumbent-well-scale", "NaN"],
            vec!["--incumbent-wasted-t-scale", "-0.1"],'''
)
h = replace(
    h,
    '''    fn accepts_direct_h3_vs_h3_configuration() {
        let args = vec![
            "--charge", "0.5", "--surge-bank", "0.25",
            "--incumbent-charge", "0.1", "--incumbent-surge-bank", "0.05",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!((s.strategy.charge, s.strategy.surge_bank), (0.5, 0.25));
        assert_eq!((s.incumbent.charge, s.incumbent.surge_bank), (0.1, 0.05));
    }''',
    '''    fn accepts_direct_h4_vs_h4_configuration() {
        let args = vec![
            "--tslot-scale", "1.5", "--well-scale", "0.5", "--wasted-t-scale", "2.0",
            "--incumbent-tslot-scale", "1.0", "--incumbent-well-scale", "1.0", "--incumbent-wasted-t-scale", "1.0",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { tslot_scale: 1.5, well_scale: 0.5, wasted_t_scale: 2.0 });
        assert_eq!(s.incumbent, Strategy::default());
    }'''
)

Path('src/bin/strategy_h4.rs').write_text(h)
