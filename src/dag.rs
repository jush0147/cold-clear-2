use bumpalo_herd::Herd;
use enum_map::EnumMap;
use once_cell::sync::Lazy;
use ouroboros::self_referencing;

use crate::data::Placement;
use crate::data::{GameState, Piece};

mod known;
mod speculated;

pub trait Evaluation:
    Ord + Copy + Default + std::ops::Add<Self::Reward, Output = Self> + 'static
{
    type Reward: Copy;
    fn scalar(self) -> f32;

    fn average(of: impl Iterator<Item = Option<Self>>) -> Self;
}

pub struct Dag<E: Evaluation> {
    root: GameState,
    top_layer: Box<LayerCommon<E>>,
}

pub struct Selection<'a, E: Evaluation> {
    layers: Vec<&'a LayerCommon<E>>,
    game_state: GameState,
}

pub struct ChildData<E: Evaluation> {
    pub resulting_state: GameState,
    pub mv: Placement,
    pub eval: E,
    pub reward: E::Reward,
}

#[derive(Default)]
struct LayerCommon<E: Evaluation> {
    next_layer: Lazy<Box<LayerCommon<E>>>,
    kind: WithBump<E>,
}

#[self_referencing]
struct WithBump<E: Evaluation> {
    bump: Herd,
    #[borrows(bump)]
    #[not_covariant]
    data: LayerKind<'this, E>,
}

enum LayerKind<'bump, E: Evaluation> {
    Known(known::Layer<'bump, E>),
    Speculated(speculated::Layer<'bump, E>),
}

#[derive(Clone, Copy, Debug)]
struct Child<E: Evaluation> {
    mv: Placement,
    reward: E::Reward,
    cached_eval: E,
}

enum SelectResult {
    Failed,
    Done,
    Advance(Piece, Placement),
}

struct BackpropUpdate {
    parent: u64,
    speculation_piece: Piece,
    mv: Placement,
    child: u64,
}

impl<E: Evaluation> Dag<E> {
    pub fn new(root: GameState, queue: &[Piece]) -> Self {
        let mut top_layer = LayerCommon::default();
        top_layer.kind.initialize_root(&root);

        let mut layer = &mut top_layer;
        for &piece in queue {
            layer.kind.despeculate(piece);
            layer = &mut layer.next_layer;
        }

        Dag {
            root,
            top_layer: Box::new(top_layer),
        }
    }

    pub fn advance(&mut self, mv: Placement) {
        puffin::profile_function!();
        let top_layer = std::mem::take(&mut *self.top_layer);
        self.root.advance(
            top_layer
                .kind
                .piece()
                .expect("cannot advance without next piece"),
            mv,
        );
        Lazy::force(&top_layer.next_layer);
        self.top_layer = Lazy::into_value(top_layer.next_layer).unwrap();
        self.top_layer.kind.initialize_root(&self.root);
    }

    pub fn add_piece(&mut self, piece: Piece) {
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

    pub fn ranked(&self) -> Vec<(Placement, f32)> {
        self.top_layer.kind.with(|this| match this.data {
            LayerKind::Known(l) => l.ranked(&self.root),
            LayerKind::Speculated(_) => vec![],
        })
    }

    pub fn suggest(&self) -> Vec<Placement> {
        puffin::profile_function!();
        self.top_layer.kind.suggest(&self.root)
    }

    pub fn select(&self, speculate: bool, exploration: f64) -> Option<Selection<E>> {
        puffin::profile_function!();
        let mut layers = vec![&*self.top_layer];
        let mut game_state = self.root;
        loop {
            let &layer = layers.last().unwrap();

            match layer.kind.select(&game_state, speculate, exploration) {
                SelectResult::Failed => return None,
                SelectResult::Done => return Some(Selection { layers, game_state }),
                SelectResult::Advance(next, placement) => {
                    game_state.advance(next, placement);
                    layers.push(&layer.next_layer);
                }
            }
        }
    }
}

impl<E: Evaluation> Selection<'_, E> {
    pub fn depth(&self) -> usize { self.layers.len() }
    pub fn cancel(self) {
        use std::sync::atomic::Ordering;
        self.layers.last().unwrap().kind.with(|this| match this.data {
            LayerKind::Known(l) => l.states.get(&self.game_state).unwrap().expanding.store(false, Ordering::Relaxed),
            LayerKind::Speculated(l) => l.states.get(&self.game_state).unwrap().expanding.store(false, Ordering::Relaxed),
        });
    }

    pub fn state(&self) -> (GameState, Option<Piece>) {
        (self.game_state, self.layers.last().unwrap().kind.piece())
    }

    pub fn expand(
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
        .iter()
        .enumerate()
        .find_map(|(i, c)| (c.mv == placement).then(|| i))
        .unwrap();
    let was_best = index == 0;

    list[index].cached_eval = child_eval + list[index].reward;

    if index > 0 && list[index - 1].cached_eval < list[index].cached_eval {
        // Shift up until the list is in order
        let hole = list[index];
        while index > 0 && list[index - 1].cached_eval < hole.cached_eval {
            list[index] = list[index - 1];
            index -= 1;
        }
        list[index] = hole;
    } else if index < list.len() - 1 && list[index + 1].cached_eval > list[index].cached_eval {
        // Shift down until the list is in order
        let hole = list[index];
        while index < list.len() - 1 && list[index + 1].cached_eval > hole.cached_eval {
            list[index] = list[index + 1];
            index += 1;
        }
        list[index] = hole;
    }

    index == 0 || (backprop_best_demotion && was_best)
}

impl<E: Evaluation> WithBump<E> {
    fn initialize_root(&self, root: &GameState) {
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.initialize_root(root),
            LayerKind::Speculated(l) => l.initialize_root(root),
        });
    }

    fn backprop(
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

    fn piece(&self) -> Option<Piece> {
        self.with(|this| match this.data {
            LayerKind::Known(l) => Some(l.piece),
            LayerKind::Speculated(_) => None,
        })
    }

    fn expand(
        &self,
        next_layer: &LayerCommon<E>,
        parent_state: GameState,
        children: EnumMap<Piece, Vec<ChildData<E>>>,
    ) -> Vec<BackpropUpdate> {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.expand(this.bump, next_layer, parent_state, children),
            LayerKind::Speculated(l) => l.expand(this.bump, next_layer, parent_state, children),
        })
    }

    fn select(&self, game_state: &GameState, speculate: bool, exploration: f64) -> SelectResult {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.select(game_state, exploration),
            LayerKind::Speculated(l) if speculate => l.select(game_state, exploration),
            LayerKind::Speculated(_) => SelectResult::Failed,
        })
    }

    fn suggest(&self, state: &GameState) -> Vec<Placement> {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.suggest(state),
            LayerKind::Speculated(l) => l.suggest(state),
        })
    }

    fn despeculate(&mut self, piece: Piece) -> bool {
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

    fn get_eval(&self, raw: u64) -> E {
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.get_eval(raw),
            LayerKind::Speculated(l) => l.get_eval(raw),
        })
    }

    fn create_nodes(
        &self,
        children: &[ChildData<E>],
        parent: u64,
        speculation_piece: Piece,
    ) -> Vec<E> {
        self.with(|this| match this.data {
            LayerKind::Known(l) => {
                let bump = this.bump.get();
                children
                    .iter()
                    .map(|child| l.create_node(&bump, child, parent, speculation_piece))
                    .collect()
            }
            LayerKind::Speculated(l) => {
                let bump = this.bump.get();
                children
                    .iter()
                    .map(|child| l.create_node(&bump, child, parent, speculation_piece))
                    .collect()
            }
        })
    }
}

impl<E: Evaluation> Default for WithBump<E> {
    fn default() -> Self {
        WithBump::new(Herd::new(), |_| LayerKind::Speculated(Default::default()))
    }
}

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
