from pathlib import Path

bot_path = Path('src/bot.rs')
freestyle_path = Path('src/bot/freestyle.rs')
dag_path = Path('src/dag.rs')
known_path = Path('src/dag/known.rs')
map_path = Path('src/map.rs')
harness_src = Path('src/bin/strategy_h6c.rs')
harness_out = Path('src/bin/strategy_h13.rs')

# Bot config: H12 stays on for both sides; H13 gets an explicit A/B flag.
bot = bot_path.read_text()
old = '''    #[serde(default)]
    pub dag_backprop_best_demotion: bool,
}
'''
new = '''    #[serde(default)]
    pub dag_backprop_best_demotion: bool,
    /// H13: when a new preview piece turns the boundary layer from speculative
    /// to known, replace bag-average values with that piece's known value and
    /// propagate the change toward the root. False preserves pre-H13 behavior.
    #[serde(default)]
    pub dag_backprop_despeculated_values: bool,
}
'''
if old not in bot:
    raise SystemExit('BotConfig H12 block not found')
bot = bot.replace(old, new, 1)
bot_path.write_text(bot)

# StateMap needs FnMut so despeculation can collect direct backprop updates while
# consuming the speculative state map.
m = map_path.read_text()
old = '    pub fn map_values<T>(self, f: impl Fn(V) -> T) -> StateMap<T, S> {'
new = '    pub fn map_values<T>(self, mut f: impl FnMut(V) -> T) -> StateMap<T, S> {'
if old not in m:
    raise SystemExit('StateMap::map_values signature not found')
m = m.replace(old, new, 1)
map_path.write_text(m)

# Pass H13 through the already-existing new_piece(options, piece) boundary.
freestyle = freestyle_path.read_text()
old = '''    fn new_piece(&mut self, _options: &BotOptions, piece: Piece) {
        puffin::profile_function!();
        self.dag.add_piece(piece);
    }
'''
new = '''    fn new_piece(&mut self, options: &BotOptions, piece: Piece) {
        puffin::profile_function!();
        self.dag.add_piece(
            piece,
            options.config.dag_backprop_despeculated_values,
            options.config.dag_backprop_best_demotion,
        );
    }
'''
if old not in freestyle:
    raise SystemExit('Freestyle::new_piece block not found')
freestyle = freestyle.replace(old, new, 1)
freestyle_path.write_text(freestyle)

dag = dag_path.read_text()

old = '''struct BackpropUpdate {
    parent: u64,
    speculation_piece: Piece,
    mv: Placement,
    child: u64,
}
'''
new = '''struct BackpropUpdate {
    parent: u64,
    speculation_piece: Piece,
    mv: Placement,
    child: u64,
}

struct DirectBackpropUpdate<E: Evaluation> {
    parent: u64,
    speculation_piece: Piece,
    mv: Placement,
    child_eval: E,
}
'''
if old not in dag:
    raise SystemExit('BackpropUpdate block not found')
dag = dag.replace(old, new, 1)

old = '''    pub fn add_piece(&mut self, piece: Piece) {
        puffin::profile_function!();
        let mut layer = &mut self.top_layer;
        loop {
            if layer.kind.despeculate(piece) {
                // TODO: backprop despeculated values
                return;
            }
            layer = &mut layer.next_layer;
        }
    }
'''
new = '''    pub fn add_piece(
        &mut self,
        piece: Piece,
        backprop_despeculated_values: bool,
        backprop_best_demotion: bool,
    ) {
        puffin::profile_function!();

        let (known_depth, mut updates) = {
            let mut known_depth = 0usize;
            let mut layer = &mut self.top_layer;
            loop {
                if let Some(updates) =
                    layer.kind.despeculate(piece, backprop_despeculated_values)
                {
                    break (known_depth, updates);
                }
                known_depth += 1;
                layer = &mut layer.next_layer;
            }
        };

        if !backprop_despeculated_values || updates.is_empty() {
            return;
        }

        // Every layer before the first speculative layer is known. Walk that
        // prefix again from the root, then propagate the revealed-piece value
        // changes from the boundary back toward the root.
        let mut layers = Vec::with_capacity(known_depth);
        let mut layer = &*self.top_layer;
        for _ in 0..known_depth {
            layers.push(layer);
            layer = &layer.next_layer;
        }

        for layer in layers.into_iter().rev() {
            updates = layer
                .kind
                .backprop_direct(updates, backprop_best_demotion);
            if updates.is_empty() {
                break;
            }
        }
    }
'''
if old not in dag:
    raise SystemExit('Dag::add_piece block not found')
dag = dag.replace(old, new, 1)

old = '''fn update_child<E: Evaluation>(
    list: &mut [Child<E>],
'''
new = '''fn despeculated_node_eval<E: Evaluation>(
    old_eval: E,
    children: Option<&[Child<E>]>,
) -> E {
    match children {
        Some(children) => {
            E::average(std::iter::once(children.first().map(|c| c.cached_eval)))
        }
        None => old_eval,
    }
}

fn update_child<E: Evaluation>(
    list: &mut [Child<E>],
'''
if old not in dag:
    raise SystemExit('update_child insertion point not found')
dag = dag.replace(old, new, 1)

old = '''    fn backprop(
        &self,
        to_update: Vec<BackpropUpdate>,
        next_layer: &LayerCommon<E>,
        backprop_best_demotion: bool,
    ) -> Vec<BackpropUpdate> {
'''
new = '''    fn backprop(
        &self,
        to_update: Vec<BackpropUpdate>,
        next_layer: &LayerCommon<E>,
        backprop_best_demotion: bool,
    ) -> Vec<BackpropUpdate> {
'''
# Signature is intentionally unchanged; use it as an anchor for the new method.
if old not in dag:
    raise SystemExit('WithBump::backprop anchor not found')
anchor_end = '''        })
    }

    fn piece(&self) -> Option<Piece> {
'''
insert = '''        })
    }

    fn backprop_direct(
        &self,
        to_update: Vec<DirectBackpropUpdate<E>>,
        backprop_best_demotion: bool,
    ) -> Vec<DirectBackpropUpdate<E>> {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => {
                l.backprop_direct(to_update, backprop_best_demotion)
            }
            LayerKind::Speculated(_) => {
                unreachable!("layers before the first speculative layer must be known")
            }
        })
    }

    fn piece(&self) -> Option<Piece> {
'''
if anchor_end not in dag:
    raise SystemExit('WithBump backprop_direct insertion anchor not found')
dag = dag.replace(anchor_end, insert, 1)

old = '''    fn despeculate(&mut self, piece: Piece) -> bool {
        puffin::profile_function!();
        self.with_mut(|this| {
            let old = match this.data {
                LayerKind::Known(_) => return false,
                LayerKind::Speculated(l) => std::mem::take(l),
            };

            let layer = known::Layer {
                states: old.states.map_values(|node| known::Node {
                    parents: node.parents,
                    eval: node.eval,
                    children: node.children.map(|v| v.into_children(piece)),
                    expanding: node.expanding,
                }),
                piece,
            };

            *this.data = LayerKind::Known(layer);

            true
        })
    }
'''
new = '''    fn despeculate(
        &mut self,
        piece: Piece,
        backprop_despeculated_values: bool,
    ) -> Option<Vec<DirectBackpropUpdate<E>>> {
        puffin::profile_function!();
        self.with_mut(|this| {
            let old = match this.data {
                LayerKind::Known(_) => return None,
                LayerKind::Speculated(l) => std::mem::take(l),
            };

            let mut updates = vec![];
            let states = old.states.map_values(|node| {
                let children = node.children.map(|v| v.into_children(piece));
                let eval = if backprop_despeculated_values {
                    despeculated_node_eval(node.eval, children.as_deref())
                } else {
                    node.eval
                };

                if backprop_despeculated_values && eval != node.eval {
                    for &(parent, mv, speculation_piece) in node.parents {
                        updates.push(DirectBackpropUpdate {
                            parent,
                            mv,
                            speculation_piece,
                            child_eval: eval,
                        });
                    }
                }

                known::Node {
                    parents: node.parents,
                    eval,
                    children,
                    expanding: node.expanding,
                }
            });

            *this.data = LayerKind::Known(known::Layer { states, piece });
            Some(updates)
        })
    }
'''
if old not in dag:
    raise SystemExit('WithBump::despeculate block not found')
dag = dag.replace(old, new, 1)

# Add focused H13 unit tests next to the existing H12 DAG tests.
tests = r'''
#[cfg(test)]
mod h13_tests {
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
            let sum: i32 = values
                .iter()
                .map(|v| v.unwrap_or(TestEval(-1_000_000)).0)
                .sum();
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
    fn h13_revealed_piece_replaces_speculative_average_with_known_best() {
        let children = [
            Child {
                mv: placement(0),
                reward: TestReward(0),
                cached_eval: TestEval(9),
            },
            Child {
                mv: placement(1),
                reward: TestReward(0),
                cached_eval: TestEval(7),
            },
        ];
        assert_eq!(
            despeculated_node_eval(TestEval(50), Some(&children)),
            TestEval(9)
        );
    }

    #[test]
    fn h13_unexpanded_despeculated_node_keeps_existing_eval() {
        assert_eq!(
            despeculated_node_eval::<TestEval>(TestEval(13), None),
            TestEval(13)
        );
    }
}
'''
if 'mod h13_tests' in dag:
    raise SystemExit('H13 tests already present')
dag += tests
dag_path.write_text(dag)

# Known layers need a direct-value backprop path because despeculation changes
# a child layer's value without a fresh expansion event carrying a child index.
known = known_path.read_text()
old = '''use super::{
    update_child, BackpropUpdate, Child, ChildData, Evaluation, LayerCommon, SelectResult,
};
'''
new = '''use super::{
    update_child, BackpropUpdate, Child, ChildData, DirectBackpropUpdate, Evaluation,
    LayerCommon, SelectResult,
};
'''
if old not in known:
    raise SystemExit('known.rs super import block not found')
known = known.replace(old, new, 1)

anchor = '''    pub fn backprop(
        &self,
        to_update: Vec<BackpropUpdate>,
        next_layer: &LayerCommon<E>,
        backprop_best_demotion: bool,
    ) -> Vec<BackpropUpdate> {
'''
i = known.find(anchor)
if i < 0:
    raise SystemExit('known.rs backprop anchor not found')
# Insert the direct method immediately before the existing indexed backprop.
direct = '''    pub fn backprop_direct(
        &self,
        to_update: Vec<DirectBackpropUpdate<E>>,
        backprop_best_demotion: bool,
    ) -> Vec<DirectBackpropUpdate<E>> {
        puffin::profile_function!();
        let mut new_updates = vec![];

        for update in to_update {
            if update.speculation_piece != self.piece {
                continue;
            }

            let mut parent = self.states.get_raw_mut(update.parent).unwrap();
            let children = parent.children.as_mut().unwrap();

            let affects_parent = update_child(
                children,
                update.mv,
                update.child_eval,
                backprop_best_demotion,
            );

            if affects_parent {
                let eval = children[0].cached_eval;

                if parent.eval != eval {
                    parent.eval = eval;

                    for &(parent, mv, speculation_piece) in parent.parents {
                        new_updates.push(DirectBackpropUpdate {
                            parent,
                            mv,
                            speculation_piece,
                            child_eval: eval,
                        });
                    }
                }
            }
        }

        new_updates
    }

'''
known = known[:i] + direct + known[i:]
known_path.write_text(known)

# Harness: corrected H12 is frozen true on both sides; H13 is the only A/B.
h = harness_src.read_text()
h = h.replace(
    '//! KO-only H6C A/B: frozen H2 incumbent versus row-transition shaping scale.',
    '//! KO-only H13 A/B: corrected H12/H9 core versus revealed-piece despeculation backprop.',
    1,
)

old = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { row_transition_scale: f32 }
impl Default for Strategy {
    fn default() -> Self { Self { row_transition_scale: 1.0 } }
}
'''
new = '''#[derive(Clone, Copy, Debug, PartialEq)]
struct Strategy { backprop_despeculated_values: bool }
impl Default for Strategy {
    fn default() -> Self { Self { backprop_despeculated_values: false } }
}
'''
if old not in h:
    raise SystemExit('H6C Strategy block not found')
h = h.replace(old, new, 1)

old = '''            "--row-transition-scale" => s.strategy.row_transition_scale = p[1].parse().map_err(|_| "invalid row transition scale")?,
            "--incumbent-row-transition-scale" => s.incumbent.row_transition_scale = p[1].parse().map_err(|_| "invalid incumbent row transition scale")?,
'''
new = '''            "--backprop-despeculated-values" => s.strategy.backprop_despeculated_values = p[1].parse().map_err(|_| "invalid despeculation flag")?,
            "--incumbent-backprop-despeculated-values" => s.incumbent.backprop_despeculated_values = p[1].parse().map_err(|_| "invalid incumbent despeculation flag")?,
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
new = '''    // Freeze promoted H9 plus corrected H12 on both sides. H13 changes only
    // whether newly revealed preview values are propagated from the boundary.
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
    config.dag_backprop_best_demotion = true;
    config.dag_backprop_despeculated_values = strategy.backprop_despeculated_values;
'''
if old not in h:
    raise SystemExit('H6C make_player block not found')
h = h.replace(old, new, 1)

old = '''        "row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "backprop_despeculated_values": s.strategy.backprop_despeculated_values,
        "incumbent_backprop_despeculated_values": s.incumbent.backprop_despeculated_values,
        "fixed_backprop_best_demotion": true,
        "fixed_row_transition_scale": 2.5,
        "fixed_h9_cavity_excavation": -0.5,
'''
if old not in h:
    raise SystemExit('H6C game metadata block not found')
h = h.replace(old, new, 1)

h = h.replace(
    '"experiment": "H6C row-transition shaping scale",',
    '"experiment": "H13 revealed-piece despeculation backprop correctness",',
    1,
)
h = h.replace(
    '"incumbent": "H2 winner with legacy row-transition scale 1.0; H1/H2 frozen; rejected H3/H4/H5/H6/H6B disabled",',
    '"incumbent": "H9 promoted evaluator + H12 corrected backprop, with legacy despeculation values",',
    1,
)

old = '''        "candidate_row_transition_scale": s.strategy.row_transition_scale,
        "incumbent_row_transition_scale": s.incumbent.row_transition_scale,
'''
new = '''        "candidate_backprop_despeculated_values": s.strategy.backprop_despeculated_values,
        "incumbent_backprop_despeculated_values": s.incumbent.backprop_despeculated_values,
        "fixed_backprop_best_demotion": true,
        "fixed_row_transition_scale": 2.5,
        "fixed_useful_attack_reward": 1.0,
        "fixed_h9_cavity_excavation": -0.5,
'''
if old not in h:
    raise SystemExit('H6C protocol metadata block not found')
h = h.replace(old, new, 1)
h = h.replace('"H6C A/A paired game not repeatable"', '"H13 A/A paired game not repeatable"')

old = '''            vec!["--row-transition-scale", "NaN"],
            vec!["--row-transition-scale", "-0.1"],
            vec!["--incumbent-row-transition-scale", "NaN"],
            vec!["--incumbent-row-transition-scale", "-0.1"],
'''
new = '''            vec!["--backprop-despeculated-values", "maybe"],
            vec!["--incumbent-backprop-despeculated-values", "maybe"],
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
new = '''    fn accepts_direct_h13_configuration() {
        let args = vec![
            "--backprop-despeculated-values", "true",
            "--incumbent-backprop-despeculated-values", "false",
        ].into_iter().map(str::to_string).collect::<Vec<_>>();
        let s = settings(&args).unwrap();
        assert_eq!(s.strategy, Strategy { backprop_despeculated_values: true });
        assert_eq!(s.incumbent, Strategy { backprop_despeculated_values: false });
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
    'config.dag_backprop_best_demotion = true;',
    'config.dag_backprop_despeculated_values = strategy.backprop_despeculated_values;',
]
for needle in required:
    if needle not in h:
        raise SystemExit(f'missing H13 harness invariant: {needle}')
if 'strategy.row_transition_scale' in h or 'combo_attack *=' in h:
    raise SystemExit('H13 must freeze H9/H12 and legacy combo scale')

harness_out.write_text(h)

for path, needles in {
    bot_path: ['pub dag_backprop_despeculated_values: bool'],
    freestyle_path: ['options.config.dag_backprop_despeculated_values'],
    dag_path: [
        'struct DirectBackpropUpdate<E: Evaluation>',
        'fn despeculated_node_eval<E: Evaluation>',
        'backprop_direct(updates, backprop_best_demotion)',
        'h13_revealed_piece_replaces_speculative_average_with_known_best',
    ],
    known_path: ['pub fn backprop_direct('],
    map_path: ['mut f: impl FnMut(V) -> T'],
}.items():
    text = path.read_text()
    for needle in needles:
        if needle not in text:
            raise SystemExit(f'missing H13 invariant in {path}: {needle}')
