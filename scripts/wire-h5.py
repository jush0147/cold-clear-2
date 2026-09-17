from pathlib import Path


def replace(text, old, new, count=1):
    actual = text.count(old)
    assert actual == count, (old[:160], actual)
    return text.replace(old, new)

h = Path('src/bin/strategy_h4.rs').read_text()

h = replace(
    h,
    '//! KO-only H4 A/B: frozen H2 incumbent versus legacy structured-attack setup bias scales.',
    '//! KO-only H5 A/B: frozen H2 incumbent versus legacy combo-continuation shaping scale.'
)
h = replace(
    h,
    '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { tslot_scale: f32, well_scale: f32, wasted_t_scale: f32 }
impl Default for Strategy {
    fn default() -> Self {
        Self { tslot_scale: 1.0, well_scale: 1.0, wasted_t_scale: 1.0 }
    }
}''',
    '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { combo_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { combo_scale: 1.0 } }
}'''
)
h = replace(
    h,
    '''            "--tslot-scale" => s.strategy.tslot_scale = p[1].parse().map_err(|_| "invalid T-slot scale")?,
            "--well-scale" => s.strategy.well_scale = p[1].parse().map_err(|_| "invalid Quad-well scale")?,
            "--wasted-t-scale" => s.strategy.wasted_t_scale = p[1].parse().map_err(|_| "invalid wasted-T scale")?,
            "--incumbent-tslot-scale" => s.incumbent.tslot_scale = p[1].parse().map_err(|_| "invalid incumbent T-slot scale")?,
            "--incumbent-well-scale" => s.incumbent.well_scale = p[1].parse().map_err(|_| "invalid incumbent Quad-well scale")?,
            "--incumbent-wasted-t-scale" => s.incumbent.wasted_t_scale = p[1].parse().map_err(|_| "invalid incumbent wasted-T scale")?,''',
    '''            "--combo-scale" => s.strategy.combo_scale = p[1].parse().map_err(|_| "invalid combo scale")?,
            "--incumbent-combo-scale" => s.incumbent.combo_scale = p[1].parse().map_err(|_| "invalid incumbent combo scale")?,'''
)
h = replace(
    h,
    '''        || !s.strategy.tslot_scale.is_finite() || s.strategy.tslot_scale < 0.0
        || !s.strategy.well_scale.is_finite() || s.strategy.well_scale < 0.0
        || !s.strategy.wasted_t_scale.is_finite() || s.strategy.wasted_t_scale < 0.0
        || !s.incumbent.tslot_scale.is_finite() || s.incumbent.tslot_scale < 0.0
        || !s.incumbent.well_scale.is_finite() || s.incumbent.well_scale < 0.0
        || !s.incumbent.wasted_t_scale.is_finite() || s.incumbent.wasted_t_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H4 scales required".into());''',
    '''        || !s.strategy.combo_scale.is_finite() || s.strategy.combo_scale < 0.0
        || !s.incumbent.combo_scale.is_finite() || s.incumbent.combo_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H5 combo scales required".into());'''
)
h = replace(
    h,
    '''    // H1 + H2 winner are frozen for BOTH sides in H4; rejected H3 values stay off.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    for value in &mut config.freestyle_weights.tslot { *value *= strategy.tslot_scale; }
    config.freestyle_weights.tetris_well_depth *= strategy.well_scale;
    config.freestyle_weights.wasted_t *= strategy.wasted_t_scale;''',
    '''    // H1 + H2 winner are frozen for BOTH sides in H5. Rejected H3/H4 changes stay off.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    config.freestyle_weights.combo_attack *= strategy.combo_scale;'''
)
h = replace(
    h,
    '''        "tslot_scale": s.strategy.tslot_scale,
        "well_scale": s.strategy.well_scale,
        "wasted_t_scale": s.strategy.wasted_t_scale,
        "incumbent_tslot_scale": s.incumbent.tslot_scale,
        "incumbent_well_scale": s.incumbent.well_scale,
        "incumbent_wasted_t_scale": s.incumbent.wasted_t_scale,''',
    '''        "combo_scale": s.strategy.combo_scale,
        "incumbent_combo_scale": s.incumbent.combo_scale,'''
)
h = replace(
    h,
    '''        "experiment": "H4 T-slot Quad-well and T-preservation shaping scales",
        "incumbent": "H2 winner with legacy setup scales 1.0/1.0/1.0 and rejected H3 disabled",
        "candidate_tslot_scale": s.strategy.tslot_scale,
        "candidate_well_scale": s.strategy.well_scale,
        "candidate_wasted_t_scale": s.strategy.wasted_t_scale,
        "incumbent_tslot_scale": s.incumbent.tslot_scale,
        "incumbent_well_scale": s.incumbent.well_scale,
        "incumbent_wasted_t_scale": s.incumbent.wasted_t_scale,''',
    '''        "experiment": "H5 legacy combo-continuation shaping scale",
        "incumbent": "H2 winner with legacy combo shaping scale 1.0; rejected H3/H4 changes disabled",
        "candidate_combo_scale": s.strategy.combo_scale,
        "incumbent_combo_scale": s.incumbent.combo_scale,'''
)
h = replace(h, '"H4 A/A paired game not repeatable"', '"H5 A/A paired game not repeatable"')
h = replace(
    h,
    '''            vec!["--tslot-scale", "NaN"],
            vec!["--well-scale", "-0.1"],
            vec!["--wasted-t-scale", "NaN"],
            vec!["--incumbent-tslot-scale", "-0.1"],
            vec!["--incumbent-well-scale", "NaN"],
            vec!["--incumbent-wasted-t-scale", "-0.1"],''',
    '''            vec!["--combo-scale", "NaN"],
            vec!["--combo-scale", "-0.1"],
            vec!["--incumbent-combo-scale", "NaN"],
            vec!["--incumbent-combo-scale", "-0.1"],'''
)
h = replace(
    h,
    '''    fn accepts_direct_h4_vs_h4_configuration() {
        let args = vec![
            "--tslot-scale", "1.5", "--well-scale", "0.5", "--wasted-t-scale", "2.0",
            "--incumbent-tslot-scale", "1.0", "--incumbent-well-scale", "1.0", "--incumbent-wasted-t-scale", "1.0",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { tslot_scale: 1.5, well_scale: 0.5, wasted_t_scale: 2.0 });
        assert_eq!(s.incumbent, Strategy::default());
    }''',
    '''    fn accepts_direct_h5_vs_h5_configuration() {
        let args = vec![
            "--combo-scale", "0.5", "--incumbent-combo-scale", "1.5",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { combo_scale: 0.5 });
        assert_eq!(s.incumbent, Strategy { combo_scale: 1.5 });
    }'''
)

Path('src/bin/strategy_h5.rs').write_text(h)
