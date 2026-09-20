use std::ops::Add;

use enum_map::EnumMap;
use enumset::EnumSet;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use super::{BotOptions, Mode, ModeSwitch, Statistics};
use crate::dag::{ChildData, Dag, Evaluation};
use crate::data::*;
use crate::movegen::find_moves_with_clutch;
use crate::tetrio;

pub struct Freestyle {
    dag: Dag<Eval>,
}

impl Freestyle {
    pub fn new(_options: &BotOptions, root: GameState, queue: &[Piece]) -> Self {
        Freestyle {
            dag: Dag::new(root, queue),
        }
    }
}

impl Mode for Freestyle {
    fn ranked(&self) -> Vec<(Placement,f32)> { self.dag.ranked() }
    fn advance(&mut self, _options: &BotOptions, mv: Placement) -> Option<ModeSwitch> {
        puffin::profile_function!();
        self.dag.advance(mv);
        None
    }

    fn new_piece(&mut self, options: &BotOptions, piece: Piece) {
        puffin::profile_function!();
        self.dag.add_piece(
            piece,
            options.config.dag_backprop_despeculated_values,
            options.config.dag_backprop_best_demotion,
        );
    }

    fn suggest(&self, _options: &BotOptions) -> Vec<Placement> {
        puffin::profile_function!();
        self.dag.suggest()
    }

    fn do_work(&self, options: &BotOptions, budget: u64) -> Statistics {
        puffin::profile_function!();
        let mut new_stats = Statistics::default();
        new_stats.selections += 1;

        if let Some(node) = self.dag.select(
            options.speculate,
            options.config.freestyle_exploitation,
            options.config.freestyle_speculated_exploitation,
        ) {
            new_stats.max_depth = node.depth();
            let (state, next) = node.state();
            new_stats.speculative_expansions = u64::from(next.is_none());
            if state.forecast.topped_out {
                node.expand(
                    EnumMap::default(),
                    options.config.dag_backprop_best_demotion,
                );
                return new_stats;
            }
            let next_possibilities = next.map(EnumSet::only).unwrap_or(state.bag);

            let mut moves = EnumMap::default();
            {
                puffin::profile_scope!("movegen");
                for piece in next_possibilities | state.reserve {
                    moves[piece] = find_moves_with_clutch(&state.board, piece, state.rules.clutch && state.combo > 0);
                }
            }

            let mut children: EnumMap<_, Vec<_>> = EnumMap::default();

            {
                puffin::profile_scope!("eval");
                for next in next_possibilities {
                    let is_root = node.depth() == 1;
                    let next_moves: &[(Placement,u32)] = &moves[next];
                    let reserve_moves: &[(Placement,u32)] = &moves[state.reserve];
                    let empty: &[(Placement,u32)] = &[];
                    let selected: Vec<(Placement,u32)> = if is_root && options.root_no_hold_only {
                        // Empty-Hold normalization stores current in reserve and
                        // NEXT[0] in the known layer. Occupied Hold is the opposite.
                        let source = if options.root_hold_is_empty { reserve_moves } else { next_moves };
                        source.to_vec()
                    } else {
                        let reserve = if next == state.reserve || options.root_hold_locked && is_root {
                            empty
                        } else {
                            reserve_moves
                        };
                        next_moves.iter().chain(reserve.iter()).copied().collect()
                    };
                    for (mv, sd_distance) in selected {
                        if is_root {
                            if let Some(allowed) = &options.root_legal_placements {
                                if !allowed.contains(&mv) { continue; }
                            }
                        }
                        if new_stats.nodes == budget {
                            // Charge evaluated nodes, but never publish a partially
                            // enumerated action set as a completed expansion.
                            node.cancel();
                            new_stats.budget_exhausted = true;
                            return new_stats;
                        }
                        new_stats.nodes += 1;
                        let mut state = state;
                        let incoming_before = state.forecast.remaining();
                        let sent_before = state.forecast.sent;
                        let info = state.advance(next, mv);

                        let (eval, reward) = evaluate(
                            &options.config.freestyle_weights,
                            state,
                            &info,
                            sd_distance,
                            incoming_before,
                            sent_before,
                        );

                        children[next].push(ChildData {
                            resulting_state: state,
                            mv,
                            eval,
                            reward,
                        });
                    }


                }
            }

            new_stats.expansions += 1;
            node.expand(children, options.config.dag_backprop_best_demotion);
        }

        new_stats
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Weights {
    /// Experimental H1 multiplier; zero preserves legacy evaluation.
    #[serde(default)]
    pub pending_safety: f32,
    /// H2: reward garbage that remains after cancellation and is actually sent.
    #[serde(default)]
    pub useful_attack_reward: f32,
    /// H2: independently reward visible incoming garbage actually cancelled.
    #[serde(default)]
    pub cancellation_reward: f32,
    /// H3: terminal value of B2B count progress toward the Surge threshold.
    #[serde(default)]
    pub h3_b2b_charge_value: f32,
    /// H3: terminal value per line of currently banked Surge once charged.
    #[serde(default)]
    pub h3_surge_bank_value: f32,
    /// H6B: scale only the always-on base hole penalty; H1 pressure safety is untouched.
    #[serde(default = "one")]
    pub h6_base_holes_scale: f32,
    /// H6B: scale only the always-on base coveredness penalty; H1 pressure safety is untouched.
    #[serde(default = "one")]
    pub h6_base_coveredness_scale: f32,
    /// H9: penalize the minimum blocker excavation needed to expose sealed empty components.
    /// Zero preserves H6C exactly.
    #[serde(default)]
    pub h9_cavity_excavation: f32,
    pub cell_coveredness: f32,
    pub max_cell_covered_height: u32,
    pub holes: f32,
    pub row_transitions: f32,
    pub height: f32,
    pub height_upper_half: f32,
    pub height_upper_quarter: f32,
    pub tetris_well_depth: f32,
    pub tslot: [f32; 4],

    pub has_back_to_back: f32,
    pub wasted_t: f32,
    pub softdrop: f32,

    pub normal_clears: [f32; 5],
    pub mini_spin_clears: [f32; 3],
    pub spin_clears: [f32; 4],
    pub back_to_back_clear: f32,
    pub combo_attack: f32,
    pub perfect_clear: f32,
    pub perfect_clear_override: bool,

    #[serde(default)]
    pub tetrio_s2: bool,
    #[serde(default = "one")]
    pub attack_reward: f32,
    #[serde(default = "one")]
    pub surge_value: f32,
    #[serde(default = "one")]
    pub b2b_charge_value: f32,
    #[serde(default)]
    pub legacy_shape_value: f32,
}

fn one() -> f32 {
    1.0
}

fn legacy_clear_reward(weights: &Weights, info: &PlacementInfo) -> f32 {
    let mut reward = 0.0;
    if info.perfect_clear {
        reward += weights.perfect_clear;
    }
    if !info.perfect_clear || !weights.perfect_clear_override {
        if info.back_to_back {
            reward += weights.back_to_back_clear;
        }
        let lines = info.lines_cleared.min(4) as usize;
        match info.placement.spin {
            Spin::None => reward += weights.normal_clears[lines],
            Spin::Mini => {
                reward += weights
                    .mini_spin_clears
                    .get(lines)
                    .copied()
                    .unwrap_or(weights.normal_clears[lines]);
            }
            Spin::Full => {
                reward += weights
                    .spin_clears
                    .get(lines)
                    .copied()
                    .unwrap_or(weights.normal_clears[lines]);
            }
        }
        reward += weights.combo_attack * (info.combo.saturating_sub(1) / 2) as f32;
    }
    reward
}

fn evaluate(
    weights: &Weights,
    mut state: GameState,
    info: &PlacementInfo,
    softdrop: u32,
    incoming_before: u32,
    sent_before: u32,
) -> (Eval, Reward) {
    if state.forecast.topped_out {
        return (Eval { value: (-1_000_000.0).into() }, Reward { value: 0.0.into() });
    }
    let mut eval = 0.0;
    let mut reward = 0.0;

    // H1: unsafe structure costs more while observed garbage remains pending.
    // Use the real board, before optimistic T-slot cutouts. Cancellation
    // lowers the remaining pressure automatically.
    if weights.pending_safety != 0.0 {
        let (height, holes, covered) = crate::ko_support::board_danger(&state.board, weights.max_cell_covered_height);
        let pressure = state.forecast.remaining().min(16) as f32 / 8.0;
        eval += weights.pending_safety * pressure * (
            weights.holes * holes as f32 + weights.cell_coveredness * covered as f32
            + weights.height_upper_half * height.saturating_sub(10) as f32
            + weights.height_upper_quarter * height.saturating_sub(15) as f32);
    }
    let legacy_shape = legacy_clear_reward(weights, info);
    if weights.tetrio_s2 {
        // Actual S2 garbage is the objective, while a fraction of the original
        // CC2 reward remains as shaping. The latter encodes useful planning
        // preferences such as avoiding inefficient normal clears that raw
        // immediate attack alone cannot see through a short search horizon.
        let attack = tetrio::attack_with_rules(info, state.rules);
        reward += weights.attack_reward * attack.total as f32;
        reward += weights.legacy_shape_value * legacy_shape;

        if state.back_to_back {
            let progress = (state.b2b_count as u32 + 1).min(4);
            eval += weights.b2b_charge_value * progress as f32;
        }
        eval += weights.surge_value * tetrio::surge_size(state.b2b_count as u32) as f32;
    } else {
        reward += legacy_shape;
    }

    // H2 is deliberately orthogonal to the older tetrio_s2 switch. The
    // incumbent keeps the complete legacy shaping plus H1. Candidate changes
    // only these two coefficients. With a forecast enabled, a line clear cannot
    // raise garbage, so queue shrinkage on that transition is cancellation.
    let raw_attack = tetrio::attack_with_rules(info, state.rules).total;
    let (useful_outgoing, cancelled) = useful_attack_delta(
        state.forecast.enabled,
        info.lines_cleared,
        raw_attack,
        incoming_before,
        state.forecast.remaining(),
        sent_before,
        state.forecast.sent,
    );
    reward += weights.useful_attack_reward * useful_outgoing as f32;
    reward += weights.cancellation_reward * cancelled as f32;

    // H3 values unrealized B2B/Surge inventory only at the search leaf. H2
    // already rewards Surge when it is actually released and sent, so release
    // attack is deliberately not rewarded a second time here. Legacy CC2
    // already rewards merely having B2B, therefore charge starts at count x1.
    let (charge_progress, surge_bank) = h3_inventory(state.back_to_back, state.b2b_count, state.rules);
    eval += weights.h3_b2b_charge_value * charge_progress as f32;
    eval += weights.h3_surge_bank_value * surge_bank as f32;

    if info.placement.location.piece == Piece::T
        && (info.lines_cleared < 2 || !matches!(info.placement.spin, Spin::Full))
    {
        reward += weights.wasted_t;
    }
    if state.back_to_back {
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
        + (state.reserve == Piece::T) as usize
        + (state.bag.len() <= 3) as usize;
    for _ in 0..cutout_count {
        let location =
            well_known_tslot_left(&state.board).or_else(|| well_known_tslot_right(&state.board));
        let location = match location {
            Some(v) => v,
            None => break,
        };
        let mut board = state.board;
        board.place(location);
        eval += weights.tslot[board.line_clears().count_ones() as usize];
        if board.line_clears().count_ones() > 1 {
            board.remove_lines(board.line_clears());
            state.board = board;
        }
    }

    eval += weights.h6_base_holes_scale * weights.holes
        * state
            .board
            .cols
            .iter()
            .map(|&c| {
                let height = 64 - c.leading_zeros();
                let underneath = (1 << height) - 1;
                let holes = !c & underneath;
                holes.count_ones()
            })
            .sum::<u32>() as f32;

    let mut coveredness = 0;
    for &c in &state.board.cols {
        let height = 64 - c.leading_zeros();
        let underneath = (1 << height) - 1;
        let mut holes = !c & underneath;
        while holes != 0 {
            let y = holes.trailing_zeros();
            coveredness += (height - y).min(weights.max_cell_covered_height);
            holes &= !(1 << y);
        }
    }
    eval += weights.h6_base_coveredness_scale * weights.cell_coveredness * coveredness as f32;

    let (tetris_well_column, tetris_well_height) = state
        .board
        .cols
        .iter()
        .enumerate()
        .map(|(i, &c)| (i, 64 - c.leading_zeros()))
        .min_by_key(|&(_, h)| h)
        .unwrap();
    let full_lines_except_well = state
        .board
        .cols
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != tetris_well_column)
        .map(|(_, &c)| c)
        .fold(!0, |a, b| a & b);
    let tetris_well_depth = (full_lines_except_well >> tetris_well_height).trailing_ones();
    eval += tetris_well_depth as f32 * weights.tetris_well_depth;

    let highest_point = state
        .board
        .cols
        .iter()
        .map(|&c| 64 - c.leading_zeros())
        .max()
        .unwrap();
    eval += weights.height * highest_point as f32;
    if highest_point > 10 {
        eval += weights.height_upper_half * (highest_point - 10) as f32;
    }
    if highest_point > 15 {
        eval += weights.height_upper_quarter * (highest_point - 15) as f32;
    }

    let mut row_transitions = 0;
    row_transitions += (!0 ^ state.board.cols[0]).count_ones();
    row_transitions += (!0 ^ state.board.cols[9]).count_ones();
    for cs in state.board.cols.windows(2) {
        row_transitions += (cs[0] ^ cs[1]).count_ones();
    }
    eval += row_transitions as f32 * weights.row_transitions;

    (
        Eval { value: eval.into() },
        Reward {
            value: reward.into(),
        },
    )
}

fn cavity_excavation_cost(board: &Board) -> u32 {
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

fn h3_inventory(back_to_back: bool, b2b_count: u16, rules: TetrioRules) -> (u32, u32) {
    if !back_to_back || !rules.b2b_charging {
        return (0, 0);
    }
    let count = b2b_count as u32;
    (count.min(rules.b2b_charge_at), tetrio::surge_size_with_rules(count, rules))
}

fn useful_attack_delta(
    forecast_enabled: bool,
    lines_cleared: u32,
    raw_attack: u32,
    incoming_before: u32,
    incoming_after: u32,
    sent_before: u32,
    sent_after: u32,
) -> (u32, u32) {
    if lines_cleared == 0 {
        return (0, 0);
    }
    if forecast_enabled {
        (
            sent_after.saturating_sub(sent_before),
            incoming_before.saturating_sub(incoming_after),
        )
    } else {
        (raw_attack, 0)
    }
}

#[cfg(test)]
mod h2_tests {
    use super::{cavity_excavation_cost, h3_inventory, useful_attack_delta};

    #[test]
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

    #[test]
    fn h3_values_only_live_charge_and_banked_surge() {
        assert_eq!(h3_inventory(false, 12, crate::data::TetrioRules::default()), (0, 0));
        assert_eq!(h3_inventory(true, 0, crate::data::TetrioRules::default()), (0, 0));
        assert_eq!(h3_inventory(true, 1, crate::data::TetrioRules::default()), (1, 0));
        assert_eq!(h3_inventory(true, 3, crate::data::TetrioRules::default()), (3, 0));
        assert_eq!(h3_inventory(true, 4, crate::data::TetrioRules::default()), (4, 1));
        assert_eq!(h3_inventory(true, 9, crate::data::TetrioRules::default()), (4, 6));
    }

    #[test]
    fn h2_separates_outgoing_from_cancelled_lines() {
        assert_eq!(useful_attack_delta(true, 2, 7, 5, 0, 10, 12), (2, 5));
        assert_eq!(useful_attack_delta(true, 2, 3, 8, 5, 4, 4), (0, 3));
    }

    #[test]
    fn h2_does_not_mistake_garbage_rise_for_cancellation() {
        assert_eq!(useful_attack_delta(true, 0, 0, 8, 0, 0, 0), (0, 0));
    }

    #[test]
    fn raw_attack_is_used_when_no_forecast_is_attached() {
        assert_eq!(useful_attack_delta(false, 4, 6, 0, 0, 0, 0), (6, 0));
    }
}

fn well_known_tslot_left(board: &Board) -> Option<PieceLocation> {
    for (x, cols) in board.cols.windows(3).enumerate() {
        let y = 64 - cols[0].leading_zeros();
        if 64 - cols[1].leading_zeros() >= y {
            continue;
        }
        if !board.occupied((x as i8 + 2, y as i8 - 1)) {
            continue;
        }
        if board.occupied((x as i8 + 2, y as i8)) {
            continue;
        }
        if !board.occupied((x as i8 + 2, y as i8 + 1)) {
            continue;
        }
        return Some(PieceLocation {
            piece: Piece::T,
            rotation: Rotation::South,
            x: x as i8 + 1,
            y: y as i8,
        });
    }
    None
}

fn well_known_tslot_right(board: &Board) -> Option<PieceLocation> {
    for (x, cols) in board.cols.windows(3).enumerate() {
        let y = 64 - cols[2].leading_zeros();
        if 64 - cols[1].leading_zeros() >= y {
            continue;
        }
        if !board.occupied((x as i8, y as i8 - 1)) {
            continue;
        }
        if board.occupied((x as i8, y as i8)) {
            continue;
        }
        if !board.occupied((x as i8, y as i8 + 1)) {
            continue;
        }
        return Some(PieceLocation {
            piece: Piece::T,
            rotation: Rotation::South,
            x: x as i8 + 1,
            y: y as i8,
        });
    }
    None
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Eval {
    value: OrderedFloat<f32>,
}

#[derive(Copy, Clone, Debug)]
struct Reward {
    value: OrderedFloat<f32>,
}

impl Evaluation for Eval {
    fn scalar(self) -> f32 { self.value.0 }
    type Reward = Reward;

    fn average(of: impl Iterator<Item = Option<Self>>) -> Self {
        let mut count = 0;
        let sum: f32 = of
            .map(|v| {
                count += 1;
                v.map(|e| e.value.0).unwrap_or(-1_000_000.0)
            })
            .sum();
        Eval {
            value: (sum / count as f32).into(),
        }
    }
}

impl Add<Reward> for Eval {
    type Output = Self;

    fn add(self, rhs: Reward) -> Eval {
        Eval {
            value: self.value + rhs.value,
        }
    }
}
