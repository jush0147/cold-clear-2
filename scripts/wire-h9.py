from pathlib import Path

core_path = Path('src/bot/freestyle.rs')
harness_src = Path('src/bin/strategy_h6c.rs')
harness_out = Path('src/bin/strategy_h9.rs')

core = core_path.read_text()

old = '''    /// H6B: scale only the always-on base coveredness penalty; H1 pressure safety is untouched.
    #[serde(default = "one")]
    pub h6_base_coveredness_scale: f32,
    pub cell_coveredness: f32,
'''
new = '''    /// H6B: scale only the always-on base coveredness penalty; H1 pressure safety is untouched.
    #[serde(default = "one")]
    pub h6_base_coveredness_scale: f32,
    /// H9: penalize the minimum blocker excavation needed to expose sealed empty components.
    /// Zero preserves H6C exactly.
    #[serde(default)]
    pub h9_cavity_excavation: f32,
    pub cell_coveredness: f32,
'''
if old not in core:
    raise SystemExit('H9 weight insertion point not found')
core = core.replace(old, new, 1)

old = '''    if state.back_to_back {
        eval += weights.has_back_to_back;
    }
    reward += weights.softdrop * softdrop as f32;

    let cutout_count = state.bag.contains(Piece::T) as usize
'''
new = '''    if state.back_to_back {
        eval += weights.has_back_to_back;
    }
    reward += weights.softdrop * softdrop as f32;

    // H9 uses the real post-placement board before optimistic T-slot cutouts.
    // Existing holes/coveredness count vertical emptiness; this adds only the
    // lower-bound excavation needed to expose sealed empty components. A cave
    // connected through empty cells to any sky-exposed column costs zero.
    if weights.h9_cavity_excavation != 0.0 {
        eval += weights.h9_cavity_excavation * cavity_excavation_cost(&state.board) as f32;
    }

    let cutout_count = state.bag.contains(Piece::T) as usize
'''
if old not in core:
    raise SystemExit('H9 evaluation insertion point not found')
core = core.replace(old, new, 1)

helper = r'''fn cavity_excavation_cost(board: &Board) -> u32 {
    let top = board
        .cols
        .iter()
        .map(|&c| 64 - c.leading_zeros())
        .max()
        .unwrap_or(0)
        .min(40) as usize;
    if top == 0 {
        return 0;
    }

    // Flood empty cells below the global surface. For each connected empty
    // component, find the cheapest vertical entry: the number of occupied
    // cells above any cell in that component. If the component can reach a
    // sky-exposed cell, that minimum is zero and no H9 penalty is added.
    let mut seen = [false; 400];
    let mut total = 0u32;
    for start_y in 0..top {
        for start_x in 0..10usize {
            let start = start_y * 10 + start_x;
            if seen[start] || (board.cols[start_x] & (1u64 << start_y)) != 0 {
                continue;
            }

            seen[start] = true;
            let mut stack = vec![start];
            let mut min_blockers = u32::MAX;
            while let Some(index) = stack.pop() {
                let x = index % 10;
                let y = index / 10;
                let blockers = (board.cols[x] >> (y + 1)).count_ones();
                min_blockers = min_blockers.min(blockers);

                let neighbors = [
                    if x > 0 { Some(index - 1) } else { None },
                    if x < 9 { Some(index + 1) } else { None },
                    if y > 0 { Some(index - 10) } else { None },
                    if y + 1 < top { Some(index + 10) } else { None },
                ];
                for next in neighbors.into_iter().flatten() {
                    let nx = next % 10;
                    let ny = next / 10;
                    if !seen[next] && (board.cols[nx] & (1u64 << ny)) == 0 {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
            total = total.saturating_add(min_blockers);
        }
    }
    total
}

'''
marker = 'fn h3_inventory(back_to_back: bool, b2b_count: u16) -> (u32, u32) {'
if marker not in core:
    raise SystemExit('H9 helper insertion point not found')
core = core.replace(marker, helper + marker, 1)

old = '    use super::{h3_inventory, useful_attack_delta};'
new = '    use super::{cavity_excavation_cost, h3_inventory, useful_attack_delta};'
if old not in core:
    raise SystemExit('H9 test import not found')
core = core.replace(old, new, 1)

tests = r'''    #[test]
    fn h9_side_open_cave_has_zero_excavation_cost() {
        let mut board = crate::data::Board::default();
        board.cols[1] = 0b10;
        assert_eq!(cavity_excavation_cost(&board), 0);
    }

    #[test]
    fn h9_sealed_component_counts_minimum_roof_blockers_once() {
        let mut board = crate::data::Board::default();
        board.cols[4] = 0b11;
        board.cols[5] = 0b100;
        board.cols[6] = 0b11;
        assert_eq!(cavity_excavation_cost(&board), 1);

        board.cols[5] = 0b1100;
        assert_eq!(cavity_excavation_cost(&board), 2);
    }

'''
marker = '    #[test]\n    fn h3_values_only_live_charge_and_banked_surge() {'
if marker not in core:
    raise SystemExit('H9 test insertion point not found')
core = core.replace(marker, tests + marker, 1)

for needle in [
    'pub h9_cavity_excavation: f32,',
    'weights.h9_cavity_excavation * cavity_excavation_cost(&state.board) as f32',
    'fn cavity_excavation_cost(board: &Board) -> u32',
    'h9_side_open_cave_has_zero_excavation_cost',
    'h9_sealed_component_counts_minimum_roof_blockers_once',
]:
    if needle not in core:
        raise SystemExit(f'missing H9 core invariant: {needle}')
core_path.write_text(core)

s = harness_src.read_text()
s = s.replace(
    '//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.',
    '//! KO-only H9 A/B: frozen H6C incumbent versus sealed-cavity excavation cost.',
    1,
)

old = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { row_transition_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { row_transition_scale: 1.0 } }
}
'''
new = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { cavity_excavation: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { cavity_excavation: 0.0 } }
}
'''
if old not in s:
    raise SystemExit('H6C Strategy block not found')
s = s.replace(old, new, 1)

old = '''            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
'''
new = '''            "--cavity-excavation" => s.strategy.cavity_excavation = p[1].parse().map_err(|_| "invalid cavity excavation weight")?,
            "--incumbent-cavity-excavation" => s.incumbent.cavity_excavation = p[1].parse().map_err(|_| "invalid incumbent cavity excavation weight")?,
'''
if old not in s:
    raise SystemExit('H6C CLI block not found')
s = s.replace(old, new, 1)

old = '''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C row-transition scales required".into());
'''
new = '''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.cavity_excavation.is_finite()
        || !s.incumbent.cavity_excavation.is_finite()
    {
        return Err("positive seeds, >=1000 nodes and finite H9 cavity weights required".into());
'''
if old not in s:
    raise SystemExit('H6C validation block not found')
s = s.replace(old, new, 1)

old = '''    // H1 + H2 winner are frozen for BOTH sides in H6C. Rejected H3/H4/H5/H6/H6B changes stay off.
    config.freestyle_weights.softdrop = 0.0;
    config.freestyle_weights.pending_safety = 1.0;
    config.freestyle_weights.useful_attack_reward = 1.0;
    config.freestyle_weights.cancellation_reward = 0.0;
    config.freestyle_weights.h3_b2b_charge_value = 0.0;
    config.freestyle_weights.h3_surge_bank_value = 0.0;
    // H6B is explicitly reset. Only the always-on row-transition term changes.
    config.freestyle_weights.h6_base_holes_scale = 1.0;
    config.freestyle_weights.h6_base_coveredness_scale = 1.0;
    config.freestyle_weights.row_transitions *= strategy.row_transition_scale;
'''
new = '''    // Freeze the promoted H6C incumbent on both sides. H9 changes only the
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
if old not in s:
    raise SystemExit('H6C make_player block not found')
s = s.replace(old, new, 1)

old = '''        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "cavity_excavation": s.strategy.cavity_excavation,
        "incumbent_cavity_excavation": s.incumbent.cavity_excavation,
        "fixed_row_transition_scale": 2.5,
'''
if old not in s:
    raise SystemExit('H6C game metadata block not found')
s = s.replace(old, new, 1)

s = s.replace(
    '"experiment": "H6C row-transition shaping scale",',
    '"experiment": "H9 sealed-cavity excavation cost",',
    1,
)
s = s.replace(
    '"incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",',
    '"incumbent": "H6C: pending safety 1.0, useful attack 1.0, row-transition scale 2.5, combo 1.0; H9 cavity weight 0",',
    1,
)
old = '''        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "candidate_cavity_excavation": s.strategy.cavity_excavation,
        "incumbent_cavity_excavation": s.incumbent.cavity_excavation,
        "fixed_row_transition_scale": 2.5,
        "fixed_useful_attack_reward": 1.0,
'''
if old not in s:
    raise SystemExit('H6C protocol metadata block not found')
s = s.replace(old, new, 1)
s = s.replace('"H6C A/A paired game not repeatable"', '"H9 A/A paired game not repeatable"')

old = '''            vec!["--row-transition-scale", "NaN"],
            vec!["--row-transition-scale", "-0.1"],
            vec!["--incumbent-row-transition-scale", "NaN"],
            vec!["--incumbent-row-transition-scale", "-0.1"],
'''
new = '''            vec!["--cavity-excavation", "NaN"],
            vec!["--incumbent-cavity-excavation", "NaN"],
'''
if old not in s:
    raise SystemExit('H6C parser rejection tests not found')
s = s.replace(old, new, 1)

old = '''    fn accepts_direct_h6c_vs_h6c_configuration() {
        let args = vec![
            "--row-transition-scale", "0.5",
            "--incumbent-row-transition-scale", "1.5",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { row_transition_scale: 0.5 });
        assert_eq!(s.incumbent, Strategy { row_transition_scale: 1.5 });
    }
'''
new = '''    fn accepts_direct_h9_vs_h6c_configuration() {
        let args = vec![
            "--cavity-excavation", "-0.75",
            "--incumbent-cavity-excavation", "0",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { cavity_excavation: -0.75 });
        assert_eq!(s.incumbent, Strategy { cavity_excavation: 0.0 });
    }
'''
if old not in s:
    raise SystemExit('H6C direct configuration test not found')
s = s.replace(old, new, 1)

required = [
    'config.freestyle_weights.pending_safety = 1.0;',
    'config.freestyle_weights.useful_attack_reward = 1.0;',
    'config.freestyle_weights.cancellation_reward = 0.0;',
    'config.freestyle_weights.h3_b2b_charge_value = 0.0;',
    'config.freestyle_weights.h3_surge_bank_value = 0.0;',
    'config.freestyle_weights.h6_base_holes_scale = 1.0;',
    'config.freestyle_weights.h6_base_coveredness_scale = 1.0;',
    'config.freestyle_weights.row_transitions *= 2.5;',
    'config.freestyle_weights.h9_cavity_excavation = strategy.cavity_excavation;',
]
for needle in required:
    if needle not in s:
        raise SystemExit(f'missing H9 harness invariant: {needle}')
if 'strategy.row_transition_scale' in s or 'combo_attack *=' in s:
    raise SystemExit('H9 must freeze H6C row scale and legacy combo scale')

harness_out.write_text(s)
