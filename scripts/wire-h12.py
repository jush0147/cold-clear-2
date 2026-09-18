from pathlib import Path

bot_path = Path('src/bot.rs')
freestyle_path = Path('src/bot/freestyle.rs')
dag_path = Path('src/dag.rs')
known_path = Path('src/dag/known.rs')
spec_path = Path('src/dag/speculated.rs')
harness_src = Path('src/bin/strategy_h6c.rs')
harness_out = Path('src/bin/strategy_h12.rs')

bot = bot_path.read_text()
old = '''pub struct BotConfig {
    pub freestyle_weights: freestyle::Weights,
    pub freestyle_exploitation: f64,
}
'''
new = '''pub struct BotConfig {
    pub freestyle_weights: freestyle::Weights,
    pub freestyle_exploitation: f64,
    /// H12: propagate when the previously-best child is demoted below another child.
    /// False preserves legacy CC2 behavior.
    #[serde(default)]
    pub dag_backprop_best_demotion: bool,
}
'''
if old not in bot:
    raise SystemExit('BotConfig insertion point not found')
bot = bot.replace(old, new, 1)
bot_path.write_text(bot)

freestyle = freestyle_path.read_text()
old = 'if state.forecast.topped_out { node.expand(EnumMap::default()); return new_stats; }'
new = '''if state.forecast.topped_out {
                node.expand(
                    EnumMap::default(),
                    options.config.dag_backprop_best_demotion,
                );
                return new_stats;
            }'''
if old not in freestyle:
    raise SystemExit('topped-out expansion point not found')
freestyle = freestyle.replace(old, new, 1)

old = '''            new_stats.expansions += 1;
            node.expand(children);
'''
new = '''            new_stats.expansions += 1;
            node.expand(children, options.config.dag_backprop_best_demotion);
'''
if old not in freestyle:
    raise SystemExit('normal expansion point not found')
freestyle = freestyle.replace(old, new, 1)
freestyle_path.write_text(freestyle)

dag = dag_path.read_text()
old = '''    pub fn expand(self, children: EnumMap<Piece, Vec<ChildData<E>>>) {
        puffin::profile_function!();
        let mut layers = self.layers;
        let start_layer = layers.pop().unwrap();
        let mut next = start_layer
            .kind
            .expand(&start_layer.next_layer, self.game_state, children);

        puffin::profile_scope!("backprop");
        let mut next_layer = start_layer;
        while let Some(layer) = layers.pop() {
            next = layer.kind.backprop(next, next_layer);
            next_layer = layer;

            if next.is_empty() {
                break;
            }
        }
    }
}

fn update_child<E: Evaluation>(list: &mut [Child<E>], placement: Placement, child_eval: E) -> bool {
    let mut index = list
'''
new = '''    pub fn expand(
        self,
        children: EnumMap<Piece, Vec<ChildData<E>>>,
        backprop_best_demotion: bool,
    ) {
        puffin::profile_function!();
        let mut layers = self.layers;
        let start_layer = layers.pop().unwrap();
        let mut next = start_layer
            .kind
            .expand(&start_layer.next_layer, self.game_state, children);

        puffin::profile_scope!("backprop");
        let mut next_layer = start_layer;
        while let Some(layer) = layers.pop() {
            next = layer
                .kind
                .backprop(next, next_layer, backprop_best_demotion);
            next_layer = layer;

            if next.is_empty() {
                break;
            }
        }
    }
}

fn update_child<E: Evaluation>(
    list: &mut [Child<E>],
    placement: Placement,
    child_eval: E,
    backprop_best_demotion: bool,
) -> bool {
    let mut index = list
'''
if old not in dag:
    raise SystemExit('Selection::expand/update_child block not found')
dag = dag.replace(old, new, 1)

old = '''        .find_map(|(i, c)| (c.mv == placement).then(|| i))
        .unwrap();

    list[index].cached_eval = child_eval + list[index].reward;
'''
new = '''        .find_map(|(i, c)| (c.mv == placement).then(|| i))
        .unwrap();
    let was_best = index == 0;

    list[index].cached_eval = child_eval + list[index].reward;
'''
if old not in dag:
    raise SystemExit('update_child pre-update block not found')
dag = dag.replace(old, new, 1)

old = '''    index == 0
}
'''
new = '''    index == 0 || (backprop_best_demotion && was_best)
}
'''
if old not in dag:
    raise SystemExit('update_child return block not found')
dag = dag.replace(old, new, 1)

old = '''    fn backprop(
        &self,
        to_update: Vec<BackpropUpdate>,
        next_layer: &LayerCommon<E>,
    ) -> Vec<BackpropUpdate> {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.backprop(to_update, next_layer),
            LayerKind::Speculated(l) => l.backprop(to_update, next_layer),
        })
    }
'''
new = '''    fn backprop(
        &self,
        to_update: Vec<BackpropUpdate>,
        next_layer: &LayerCommon<E>,
        backprop_best_demotion: bool,
    ) -> Vec<BackpropUpdate> {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => {
                l.backprop(to_update, next_layer, backprop_best_demotion)
            }
            LayerKind::Speculated(l) => {
                l.backprop(to_update, next_layer, backprop_best_demotion)
            }
        })
    }
'''
if old not in dag:
    raise SystemExit('LayerCommon backprop block not found')
dag = dag.replace(old, new, 1)

tests = r'''
#[cfg(test)]
mod h12_tests {
    use super::*;
    use crate::data::{PieceLocation, Rotation, Spin};

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    struct TestEval(i32);

    #[derive(Clone, Copy, Debug)]
    struct TestReward(i32);

    impl std::ops::Add<TestReward> for TestEval {
        type Output = Self;
        fn add(self, rhs: TestReward) -> Self { TestEval(self.0 + rhs.0) }
    }

    impl Evaluation for TestEval {
        type Reward = TestReward;
        fn scalar(self) -> f32 { self.0 as f32 }
        fn average(of: impl Iterator<Item = Option<Self>>) -> Self {
            let values: Vec<_> = of.collect();
            let sum: i32 = values.iter().map(|v| v.unwrap_or(TestEval(-1_000_000)).0).sum();
            TestEval(sum / values.len() as i32)
        }
    }

    fn placement(x: i8) -> Placement {
        Placement {
            location: PieceLocation {
                piece: Piece::T,
                rotation: Rotation::North,
                x,
                y: 0,
            },
            spin: Spin::None,
        }
    }

    #[test]
    fn h12_demoted_best_requires_parent_recompute() {
        let original = [
            Child { mv: placement(0), reward: TestReward(0), cached_eval: TestEval(10) },
            Child { mv: placement(1), reward: TestReward(0), cached_eval: TestEval(9) },
        ];

        let mut legacy = original;
        assert!(!update_child(&mut legacy, placement(0), TestEval(8), false));
        assert_eq!(legacy[0].cached_eval, TestEval(9));
        assert_eq!(legacy[1].cached_eval, TestEval(8));

        let mut fixed = original;
        assert!(update_child(&mut fixed, placement(0), TestEval(8), true));
        assert_eq!(fixed[0].cached_eval, TestEval(9));
        assert_eq!(fixed[1].cached_eval, TestEval(8));
    }

    #[test]
    fn h12_nonbest_change_that_stays_nonbest_needs_no_parent_recompute() {
        let mut list = [
            Child { mv: placement(0), reward: TestReward(0), cached_eval: TestEval(10) },
            Child { mv: placement(1), reward: TestReward(0), cached_eval: TestEval(8) },
        ];
        assert!(!update_child(&mut list, placement(1), TestEval(9), true));
        assert_eq!(list[0].cached_eval, TestEval(10));
        assert_eq!(list[1].cached_eval, TestEval(9));
    }
}
'''
if 'mod h12_tests' in dag:
    raise SystemExit('H12 tests already present')
dag += tests
dag_path.write_text(dag)

for path in [known_path, spec_path]:
    s = path.read_text()
    old = '''    pub fn backprop(
        &self,
        to_update: Vec<BackpropUpdate>,
        next_layer: &LayerCommon<E>,
    ) -> Vec<BackpropUpdate> {
'''
    new = '''    pub fn backprop(
        &self,
        to_update: Vec<BackpropUpdate>,
        next_layer: &LayerCommon<E>,
        backprop_best_demotion: bool,
    ) -> Vec<BackpropUpdate> {
'''
    if old not in s:
        raise SystemExit(f'backprop signature not found in {path}')
    s = s.replace(old, new, 1)

    old = 'let is_best = update_child(children, update.mv, child_eval);' if path == known_path else 'let is_best = update_child(list, update.mv, child_eval);'
    new = (
        'let is_best = update_child(\\n                children,\\n                update.mv,\\n                child_eval,\\n                backprop_best_demotion,\\n            );'
        if path == known_path else
        'let is_best = update_child(\\n                list,\\n                update.mv,\\n                child_eval,\\n                backprop_best_demotion,\\n            );'
    )
    # Keep real newlines rather than literal backslash-n.
    new = new.replace('\\\\n', '\n')
    if old not in s:
        raise SystemExit(f'update_child call not found in {path}')
    s = s.replace(old, new, 1)
    path.write_text(s)

h = harness_src.read_text()
h = h.replace(
    '//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.',
    '//! KO-only H12 A/B: promoted H9 incumbent versus best-child demotion backprop fix.',
    1,
)

old = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { row_transition_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { row_transition_scale: 1.0 } }
}
'''
new = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { backprop_best_demotion: bool }
impl Default for Strategy {
    fn default() -> Self { Self { backprop_best_demotion: false } }
}
'''
if old not in h:
    raise SystemExit('H6C Strategy block not found')
h = h.replace(old, new, 1)

old = '''            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
'''
new = '''            "--backprop-best-demotion" => s.strategy.backprop_best_demotion = p[1].parse().map_err(|_| "invalid best-demotion flag")?,
            "--incumbent-backprop-best-demotion" => s.incumbent.backprop_best_demotion = p[1].parse().map_err(|_| "invalid incumbent best-demotion flag")?,
'''
if old not in h:
    raise SystemExit('H6C CLI block not found')
h = h.replace(old, new, 1)

old = '''    if s.seeds == 0 || s.nodes < 1000
        || !s.strategy.row_transition_scale.is_finite() || s.strategy.row_transition_scale < 0.0
        || !s.incumbent.row_transition_scale.is_finite() || s.incumbent.row_transition_scale < 0.0
    {
        return Err("positive seeds, >=1000 nodes and finite nonnegative H6C row-transition scales required".into());
'''
new = '''    if s.seeds == 0 || s.nodes < 1000 {
        return Err("positive seeds and >=1000 nodes required".into());
'''
if old not in h:
    raise SystemExit('H6C validation block not found')
h = h.replace(old, new, 1)

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
new = '''    // Freeze the promoted H9 evaluator on both sides. H12 changes only whether
    // a formerly-best child that gets demoted triggers parent recomputation.
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
    config.dag_backprop_best_demotion = strategy.backprop_best_demotion;
'''
if old not in h:
    raise SystemExit('H6C make_player block not found')
h = h.replace(old, new, 1)

old = '''        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "backprop_best_demotion": s.strategy.backprop_best_demotion,
        "incumbent_backprop_best_demotion": s.incumbent.backprop_best_demotion,
        "fixed_row_transition_scale": 2.5,
        "fixed_h9_cavity_excavation": -0.5,
'''
if old not in h:
    raise SystemExit('H6C game metadata block not found')
h = h.replace(old, new, 1)

h = h.replace(
    '"experiment": "H6C row-transition shaping scale",',
    '"experiment": "H12 best-child demotion backprop correctness",',
    1,
)
h = h.replace(
    '"incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",',
    '"incumbent": "H9 promoted evaluator with legacy best-child demotion backprop behavior",',
    1,
)

old = '''        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "candidate_backprop_best_demotion": s.strategy.backprop_best_demotion,
        "incumbent_backprop_best_demotion": s.incumbent.backprop_best_demotion,
        "fixed_row_transition_scale": 2.5,
        "fixed_useful_attack_reward": 1.0,
        "fixed_h9_cavity_excavation": -0.5,
'''
if old not in h:
    raise SystemExit('H6C protocol metadata block not found')
h = h.replace(old, new, 1)
h = h.replace('"H6C A/A paired game not repeatable"', '"H12 A/A paired game not repeatable"')

old = '''            vec!["--row-transition-scale", "NaN"],
            vec!["--row-transition-scale", "-0.1"],
            vec!["--incumbent-row-transition-scale", "NaN"],
            vec!["--incumbent-row-transition-scale", "-0.1"],
'''
new = '''            vec!["--backprop-best-demotion", "maybe"],
            vec!["--incumbent-backprop-best-demotion", "maybe"],
'''
if old not in h:
    raise SystemExit('H6C parser rejection tests not found')
h = h.replace(old, new, 1)

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
new = '''    fn accepts_direct_h12_configuration() {
        let args = vec![
            "--backprop-best-demotion", "true",
            "--incumbent-backprop-best-demotion", "false",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { backprop_best_demotion: true });
        assert_eq!(s.incumbent, Strategy { backprop_best_demotion: false });
    }
'''
if old not in h:
    raise SystemExit('H6C direct configuration test not found')
h = h.replace(old, new, 1)

required = [
    'config.freestyle_weights.pending_safety = 1.0;',
    'config.freestyle_weights.useful_attack_reward = 1.0;',
    'config.freestyle_weights.row_transitions *= 2.5;',
    'config.freestyle_weights.h9_cavity_excavation = -0.5;',
    'config.dag_backprop_best_demotion = strategy.backprop_best_demotion;',
    'backprop_best_demotion: bool',
]
for needle in required:
    if needle not in h:
        raise SystemExit(f'missing H12 harness invariant: {needle}')
if 'strategy.row_transition_scale' in h or 'combo_attack *=' in h:
    raise SystemExit('H12 must freeze H9 evaluator and legacy combo scale')

harness_out.write_text(h)

for path, needles in {
    bot_path: ['pub dag_backprop_best_demotion: bool'],
    freestyle_path: ['options.config.dag_backprop_best_demotion'],
    dag_path: ['backprop_best_demotion && was_best', 'h12_demoted_best_requires_parent_recompute'],
    known_path: ['backprop_best_demotion: bool'],
    spec_path: ['backprop_best_demotion: bool'],
}.items():
    text = path.read_text()
    for needle in needles:
        if needle not in text:
            raise SystemExit(f'missing H12 invariant in {path}: {needle}')
