from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)

freestyle_path = Path("src/bot/freestyle.rs")
freestyle = freestyle_path.read_text()
freestyle = replace_once(
    freestyle,
    "    /// H3: terminal value per line of currently banked Surge once charged.\n    #[serde(default)]\n    pub h3_surge_bank_value: f32,\n    pub cell_coveredness: f32,",
    "    /// H3: terminal value per line of currently banked Surge once charged.\n    #[serde(default)]\n    pub h3_surge_bank_value: f32,\n    /// H6B: scale only the always-on base hole penalty; H1 pressure safety is untouched.\n    #[serde(default = \"one\")]\n    pub h6_base_holes_scale: f32,\n    /// H6B: scale only the always-on base coveredness penalty; H1 pressure safety is untouched.\n    #[serde(default = \"one\")]\n    pub h6_base_coveredness_scale: f32,\n    pub cell_coveredness: f32,",
    "insert H6B weights",
)
freestyle = replace_once(
    freestyle,
    "    eval += weights.holes\n        * state",
    "    eval += weights.h6_base_holes_scale * weights.holes\n        * state",
    "scale base holes only",
)
freestyle = replace_once(
    freestyle,
    "    eval += weights.cell_coveredness * coveredness as f32;",
    "    eval += weights.h6_base_coveredness_scale * weights.cell_coveredness * coveredness as f32;",
    "scale base coveredness only",
)
freestyle_path.write_text(freestyle)

src = Path("src/bin/strategy_h6.rs").read_text()
src = replace_once(
    src,
    "//! KO-only H6 A/B: frozen H2 incumbent versus legacy base-height aversion scale.",
    "//! KO-only H6B A/B: frozen H2 incumbent versus base-only hole/coveredness shaping.",
    "module comment",
)
src = replace_once(
    src,
    "struct Strategy { height_scale: f32 }\nimpl Default for Strategy {\n    fn default() -> Self { Self { height_scale: 1.0 } }\n}",
    "struct Strategy { holes_scale: f32, covered_scale: f32 }\nimpl Default for Strategy {\n    fn default() -> Self { Self { holes_scale: 1.0, covered_scale: 1.0 } }\n}",
    "strategy fields",
)
src = replace_once(
    src,
    "            \"--height-scale\" => s.strategy.height_scale = p[1].parse().map_err(|_| \"invalid height scale\")?,\n            \"--incumbent-height-scale\" => s.incumbent.height_scale = p[1].parse().map_err(|_| \"invalid incumbent height scale\")?,",
    "            \"--holes-scale\" => s.strategy.holes_scale = p[1].parse().map_err(|_| \"invalid holes scale\")?,\n            \"--covered-scale\" => s.strategy.covered_scale = p[1].parse().map_err(|_| \"invalid coveredness scale\")?,\n            \"--incumbent-holes-scale\" => s.incumbent.holes_scale = p[1].parse().map_err(|_| \"invalid incumbent holes scale\")?,\n            \"--incumbent-covered-scale\" => s.incumbent.covered_scale = p[1].parse().map_err(|_| \"invalid incumbent coveredness scale\")?,",
    "CLI flags",
)
src = replace_once(
    src,
    "        || !s.strategy.height_scale.is_finite() || s.strategy.height_scale < 0.0\n        || !s.incumbent.height_scale.is_finite() || s.incumbent.height_scale < 0.0\n    {\n        return Err(\"positive seeds, >=1000 nodes and finite nonnegative H6 height scales required\".into());",
    "        || !s.strategy.holes_scale.is_finite() || s.strategy.holes_scale < 0.0\n        || !s.strategy.covered_scale.is_finite() || s.strategy.covered_scale < 0.0\n        || !s.incumbent.holes_scale.is_finite() || s.incumbent.holes_scale < 0.0\n        || !s.incumbent.covered_scale.is_finite() || s.incumbent.covered_scale < 0.0\n    {\n        return Err(\"positive seeds, >=1000 nodes and finite nonnegative H6B cavity scales required\".into());",
    "CLI validation",
)
src = replace_once(
    src,
    "    // H1 + H2 winner are frozen for BOTH sides in H6. Rejected H3/H4/H5 changes stay off.",
    "    // H1 + H2 winner are frozen for BOTH sides in H6B. Rejected H3/H4/H5/H6-height changes stay off.",
    "frozen comment",
)
src = replace_once(
    src,
    "    // Only the always-on base height term changes. H1 upper-height pressure terms stay frozen.\n    config.freestyle_weights.height *= strategy.height_scale;",
    "    // Only always-on cavity shaping changes. H1 pending-pressure terms keep the original hole/coveredness weights.\n    config.freestyle_weights.h6_base_holes_scale = strategy.holes_scale;\n    config.freestyle_weights.h6_base_coveredness_scale = strategy.covered_scale;",
    "make_player strategy",
)
src = replace_once(
    src,
    "        \"height_scale\": s.strategy.height_scale,\n        \"incumbent_height_scale\": s.incumbent.height_scale,",
    "        \"holes_scale\": s.strategy.holes_scale,\n        \"covered_scale\": s.strategy.covered_scale,\n        \"incumbent_holes_scale\": s.incumbent.holes_scale,\n        \"incumbent_covered_scale\": s.incumbent.covered_scale,",
    "game JSON scales",
)
src = replace_once(
    src,
    "        \"experiment\": \"H6 legacy base-height aversion scale\",\n        \"incumbent\": \"H2 winner with base height scale 1.0; H1 upper-height pressure terms frozen; rejected H3/H4/H5 disabled\",\n        \"candidate_height_scale\": s.strategy.height_scale,\n        \"incumbent_height_scale\": s.incumbent.height_scale,",
    "        \"experiment\": \"H6B base-only holes and coveredness shaping\",\n        \"incumbent\": \"H2 winner with base cavity scales 1.0/1.0; H1 pending-pressure safety frozen; rejected H3/H4/H5/H6-height disabled\",\n        \"candidate_holes_scale\": s.strategy.holes_scale,\n        \"candidate_covered_scale\": s.strategy.covered_scale,\n        \"incumbent_holes_scale\": s.incumbent.holes_scale,\n        \"incumbent_covered_scale\": s.incumbent.covered_scale,",
    "protocol JSON",
)
src = src.replace("H6 A/A paired game not repeatable", "H6B A/A paired game not repeatable")
src = replace_once(
    src,
    "            vec![\"--height-scale\", \"NaN\"],\n            vec![\"--height-scale\", \"-0.1\"],\n            vec![\"--incumbent-height-scale\", \"NaN\"],\n            vec![\"--incumbent-height-scale\", \"-0.1\"],",
    "            vec![\"--holes-scale\", \"NaN\"],\n            vec![\"--holes-scale\", \"-0.1\"],\n            vec![\"--covered-scale\", \"NaN\"],\n            vec![\"--covered-scale\", \"-0.1\"],\n            vec![\"--incumbent-holes-scale\", \"NaN\"],\n            vec![\"--incumbent-covered-scale\", \"-0.1\"],",
    "negative tests",
)
src = replace_once(
    src,
    "    fn accepts_direct_h6_vs_h6_configuration() {\n        let args = vec![\n            \"--height-scale\", \"0.5\", \"--incumbent-height-scale\", \"1.5\",\n        ].into_iter().map(str::to_string).collect::<Vec<_>>();\n        let s = settings(&args).unwrap();\n        assert_eq!(s.strategy, Strategy { height_scale: 0.5 });\n        assert_eq!(s.incumbent, Strategy { height_scale: 1.5 });\n    }",
    "    fn accepts_direct_h6b_vs_h6b_configuration() {\n        let args = vec![\n            \"--holes-scale\", \"0.5\", \"--covered-scale\", \"2.0\",\n            \"--incumbent-holes-scale\", \"1.5\", \"--incumbent-covered-scale\", \"0.75\",\n        ].into_iter().map(str::to_string).collect::<Vec<_>>();\n        let s = settings(&args).unwrap();\n        assert_eq!(s.strategy, Strategy { holes_scale: 0.5, covered_scale: 2.0 });\n        assert_eq!(s.incumbent, Strategy { holes_scale: 1.5, covered_scale: 0.75 });\n    }",
    "direct config test",
)
Path("src/bin/strategy_h6b.rs").write_text(src)
