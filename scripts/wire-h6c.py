from pathlib import Path

p = Path('src/bin/strategy_h6b.rs')
s = p.read_text()
s = s.replace('H6B', 'H6C').replace('h6b', 'h6c')

replacements = [
    ('//! KO-only H6C A/B: frozen H2 incumbent versus base-only hole/coveredness shaping.',
     '//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.'),
    ('struct Strategy { holes_scale: f32, covered_scale: f32 }',
     'struct Strategy { row_transition_scale: f32 }'),
    ('fn default() -> Self { Self { holes_scale: 1.0, covered_scale: 1.0 } }',
     'fn default() -> Self { Self { row_transition_scale: 1.0 } }'),
    ('            "--holes-scale" => s.strategy.holes_scale = p[1].parse().map_err(|_| "invalid holes scale")?,\n'
     '            "--covered-scale" => s.strategy.covered_scale = p[1].parse().map_err(|_| "invalid coveredness scale")?,\n'
     '            "--incumbent-holes-scale" => s.incumbent.holes_scale = p[1].parse().map_err(|_| "invalid incumbent holes scale")?,\n'
     '            "--incumbent-covered-scale" => s.incumbent.covered_scale = p[1].parse().map_err(|_| "invalid incumbent coveredness scale")?,',
     '            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,\n'
     '            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,'),
    ('        || !s.strategy.holes_scale.is_finite() || s.strategy.holes_scale < 0.0\n'
     '        || !s.strategy.covered_scale.is_finite() || s.strategy.covered_scale < 0.0\n'
     '        || !s.incumbent.holes_scale.is_finite() || s.incumbent.holes_scale < 0.0\n'
     '        || !s.incumbent.covered_scale.is_finite() || s.incumbent.covered_scale < 0.0',
     '        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0\n'
     '        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0'),
    ('        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C cavity scales required".into());',
     '        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C row-transition scales required".into());'),
    ('    // H1 + H2 winner are frozen for BOTH sides in H6C. Rejected H3/H4/H5/H6-height changes stay off.',
     '    // H1 + H2 winner are frozen for BOTH sides in H6C. Rejected H3/H4/H5/H6/H6B changes stay off.'),
    ('    // Only always-on cavity shaping changes. H1 pending-pressure terms keep the original hole/coveredness weights.\n'
     '    config.freestyle_weights.h6_base_holes_scale = strategy.holes_scale;\n'
     '    config.freestyle_weights.h6_base_coveredness_scale = strategy.covered_scale;',
     '    // H6B is explicitly reset. Only the always-on row-transition term changes.\n'
     '    config.freestyle_weights.h6_base_holes_scale = 1.0;\n'
     '    config.freestyle_weights.h6_base_coveredness_scale = 1.0;\n'
     '    config.freestyle_weights.row_transitions *= strategy.row_transition_scale;'),
    ('        "holes_scale": s.strategy.holes_scale,\n'
     '        "covered_scale": s.strategy.covered_scale,\n'
     '        "incumbent_holes_scale": s.incumbent.holes_scale,\n'
     '        "incumbent_covered_scale": s.incumbent.covered_scale,',
     '        "row_transition_scale": s.strategy.row_transition_scale,\n'
     '        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,'),
    ('        "experiment": "H6C base-only holes and coveredness shaping",',
     '        "experiment": "H6C row-transition shaping scale",'),
    ('        "incumbent": "H2 winner with base cavity scales 1.0/1.0; H1 pending-pressure safety frozen; rejected H3/H4/H5/H6-height disabled",',
     '        "incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",'),
    ('        "candidate_holes_scale": s.strategy.holes_scale,\n'
     '        "candidate_covered_scale": s.strategy.covered_scale,\n'
     '        "incumbent_holes_scale": s.incumbent.holes_scale,\n'
     '        "incumbent_covered_scale": s.incumbent.covered_scale,',
     '        "candidate_row_transition_scale": s.strategy.row_transition_scale,\n'
     '        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,'),
    ('            vec!["--holes-scale", "NaN"],\n'
     '            vec!["--holes-scale", "-0.1"],\n'
     '            vec!["--covered-scale", "NaN"],\n'
     '            vec!["--covered-scale", "-0.1"],\n'
     '            vec!["--incumbent-holes-scale", "NaN"],\n'
     '            vec!["--incumbent-covered-scale", "-0.1"],',
     '            vec!["--row-transition-scale", "NaN"],\n'
     '            vec!["--row-transition-scale", "-0.1"],\n'
     '            vec!["--incumbent-row-transition-scale", "NaN"],\n'
     '            vec!["--incumbent-row-transition-scale", "-0.1"],'),
    ('    fn accepts_direct_h6c_vs_h6c_configuration() {\n'
     '        let args = vec![\n'
     '            "--holes-scale", "0.5", "--covered-scale", "2.0",\n'
     '            "--incumbent-holes-scale", "1.5", "--incumbent-covered-scale", "0.75",\n'
     '        ].into_iter().map(str::to_string).collect::<Vec<_>>();\n'
     '        let s = settings(&args).unwrap();\n'
     '        assert_eq!(s.strategy, Strategy { holes_scale: 0.5, covered_scale: 2.0 });\n'
     '        assert_eq!(s.incumbent, Strategy { holes_scale: 1.5, covered_scale: 0.75 });\n'
     '    }',
     '    fn accepts_direct_h6c_vs_h6c_configuration() {\n'
     '        let args = vec![\n'
     '            "--row-transition-scale", "0.5",\n'
     '            "--incumbent-row-transition-scale", "1.5",\n'
     '        ].into_iter().map(str::to_string).collect::<Vec<_>>();\n'
     '        let s = settings(&args).unwrap();\n'
     '        assert_eq!(s.strategy, Strategy { row_transition_scale: 0.5 });\n'
     '        assert_eq!(s.incumbent, Strategy { row_transition_scale: 1.5 });\n'
     '    }')
]

for old, new in replacements:
    if old not in s:
        raise SystemExit(f'missing expected H6B source fragment:\n{old}')
    s = s.replace(old, new, 1)

# The generated H6C harness intentionally references the H6B base-only
# config fields to reset them to 1.0. Guard only against stale H6B strategy/CLI
# references, not those reset field names.
stale = [
    'strategy.holes_scale',
    'strategy.covered_scale',
    'incumbent.holes_scale',
    'incumbent.covered_scale',
    '"--holes-scale"',
    '"--covered-scale"',
    '"--incumbent-holes-scale"',
    '"--incumbent-covered-scale"',
]
for needle in stale:
    if needle in s:
        raise SystemExit(f'stale H6B strategy reference remains in generated H6C harness: {needle}')

Path('src/bin/strategy_h6c.rs').write_text(s)
