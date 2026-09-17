from pathlib import Path


def replace(text, old, new, count=1):
    actual = text.count(old)
    assert actual == count, (old[:160], actual)
    return text.replace(old, new)

h = Path('src/bin/strategy_h5.rs').read_text()

h = replace(
    h,
    '//! KO-only H5 A/B: frozen H2 incumbent versus legacy combo-continuation shaping scale.',
    '//! KO-only H6 A/B: frozen H2 incumbent versus legacy base-height aversion scale.'
)
h = replace(
    h,
    'struct Strategy { combo_scale: f32 }',
    'struct Strategy { height_scale: f32 }'
)
h = replace(
    h,
    'fn default() -> Self { Self { combo_scale: 1.0 } }',
    'fn default() -> Self { Self { height_scale: 1.0 } }'
)
h = replace(
    h,
    '            "--combo-scale" => s.strategy.combo_scale = p[1].parse().map_err(|_| "invalid combo scale")?,\n            "--incumbent-combo-scale" => s.incumbent.combo_scale = p[1].parse().map_err(|_| "invalid incumbent combo scale")?,',
    '            "--height-scale" => s.strategy.height_scale = p[1].parse().map_err(|_| "invalid height scale")?,\n            "--incumbent-height-scale" => s.incumbent.height_scale = p[1].parse().map_err(|_| "invalid incumbent height scale")?,'
)
h = replace(
    h,
    '        || !s.strategy.combo_scale.is_finite() || s.strategy.combo_scale < 0.0\n        || !s.incumbent.combo_scale.is_finite() || s.incumbent.combo_scale < 0.0',
    '        || !s.strategy.height_scale.is_finite() || s.strategy.height_scale < 0.0\n        || !s.incumbent.height_scale.is_finite() || s.incumbent.height_scale < 0.0'
)
h = replace(
    h,
    'return Err("positive seeds, >=1000 nodes and finite nonnegative H5 combo scales required".into());',
    'return Err("positive seeds, >=1000 nodes and finite nonnegative H6 height scales required".into());'
)
h = replace(
    h,
    '    // H1 + H2 winner are frozen for BOTH sides in H5. Rejected H3/H4 changes stay off.\n',
    '    // H1 + H2 winner are frozen for BOTH sides in H6. Rejected H3/H4/H5 changes stay off.\n'
)
h = replace(
    h,
    '    config.freestyle_weights.combo_attack *= strategy.combo_scale;',
    '    // Only the always-on base height term changes. H1 upper-height pressure terms stay frozen.\n    config.freestyle_weights.height *= strategy.height_scale;'
)
h = replace(
    h,
    '        "combo_scale": s.strategy.combo_scale,\n        "incumbent_combo_scale": s.incumbent.combo_scale,',
    '        "height_scale": s.strategy.height_scale,\n        "incumbent_height_scale": s.incumbent.height_scale,'
)
h = replace(
    h,
    '        "experiment": "H5 legacy combo-continuation shaping scale",\n        "incumbent": "H2 winner with legacy combo shaping scale 1.0; rejected H3/H4 changes disabled",\n        "candidate_combo_scale": s.strategy.combo_scale,\n        "incumbent_combo_scale": s.incumbent.combo_scale,',
    '        "experiment": "H6 legacy base-height aversion scale",\n        "incumbent": "H2 winner with base height scale 1.0; H1 upper-height pressure terms frozen; rejected H3/H4/H5 disabled",\n        "candidate_height_scale": s.strategy.height_scale,\n        "incumbent_height_scale": s.incumbent.height_scale,'
)
h = replace(h, '"H5 A/A paired game not repeatable"', '"H6 A/A paired game not repeatable"')
h = replace(
    h,
    '            vec!["--combo-scale", "NaN"],\n            vec!["--combo-scale", "-0.1"],\n            vec!["--incumbent-combo-scale", "NaN"],\n            vec!["--incumbent-combo-scale", "-0.1"],',
    '            vec!["--height-scale", "NaN"],\n            vec!["--height-scale", "-0.1"],\n            vec!["--incumbent-height-scale", "NaN"],\n            vec!["--incumbent-height-scale", "-0.1"],'
)
h = replace(
    h,
    '''    fn accepts_direct_h5_vs_h5_configuration() {
        let args = vec![
            "--combo-scale", "0.5", "--incumbent-combo-scale", "1.5",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { combo_scale: 0.5 });
        assert_eq!(s.incumbent, Strategy { combo_scale: 1.5 });
    }''',
    '''    fn accepts_direct_h6_vs_h6_configuration() {
        let args = vec![
            "--height-scale", "0.5", "--incumbent-height-scale", "1.5",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { height_scale: 0.5 });
        assert_eq!(s.incumbent, Strategy { height_scale: 1.5 });
    }'''
)

Path('src/bin/strategy_h6.rs').write_text(h)
