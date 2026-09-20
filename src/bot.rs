use std::collections::VecDeque;
use std::sync::Arc;

use enum_dispatch::enum_dispatch;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::data::{GameState, Piece, Placement, PlacementInfo};
use crate::movegen::find_moves_with_clutch;

mod freestyle;
use self::freestyle::Freestyle;

pub struct Bot {
    options: BotOptions,
    current: GameState,
    queue: VecDeque<Piece>,
    mode: ModeEnum,
    // CC2 normalizes empty hold as reserve=current, queue=NEXT. This is a
    // search representation, NOT an actual hold action. Preserve that fact
    // at the boundary so the first real hold consumes/reveals two pieces.
    hold_is_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerPieces {
    pub current: Option<Piece>,
    pub hold: Option<Piece>,
    pub next: Vec<Piece>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BotConfig {
    pub freestyle_weights: freestyle::Weights,
    pub freestyle_exploitation: f64,
    /// H11: allow a different child-selection exploitation value after the
    /// visible preview ends and search enters speculative SevenBag layers.
    #[serde(default = "default_freestyle_speculated_exploitation")]
    pub freestyle_speculated_exploitation: f64,
    /// H12: propagate when the previously-best child is demoted below another child.
    /// False preserves legacy CC2 behavior.
    #[serde(default)]
    pub dag_backprop_best_demotion: bool,
    /// H13: when a new preview piece turns the boundary layer from speculative
    /// to known, replace bag-average values with that piece's known value and
    /// propagate the change toward the root. False preserves pre-H13 behavior.
    #[serde(default)]
    pub dag_backprop_despeculated_values: bool,
}
fn default_freestyle_speculated_exploitation() -> f64 {
    std::f64::consts::LN_2
}

impl Default for BotConfig {
    fn default() -> Self {
        static DEFAULT: Lazy<BotConfig> = Lazy::new(|| serde_json::from_str(include_str!("default.json")).unwrap());
        DEFAULT.clone()
    }
}
impl BotConfig {
    /// Legacy evaluator on the modified core, not an untouched upstream bot.
    pub fn legacy() -> Self {
        let mut config = Self::default();
        config.freestyle_weights.tetrio_s2 = false;
        config
    }
    pub fn tetrio_s2(charge_value: f32, surge_value: f32, shape_value: f32) -> Self {
        let mut config = Self::default();
        config.freestyle_weights.tetrio_s2 = true;
        config.freestyle_weights.b2b_charge_value = charge_value;
        config.freestyle_weights.surge_value = surge_value;
        config.freestyle_weights.legacy_shape_value = shape_value;
        config
    }

    /// Corrected neutral starting point for the post-migration strategy reset.
    /// This keeps legacy evaluator weights, zeroes experimental H1/H2/H3/H6B/H9
    /// terms, removes softdrop placement cost, enables H12, and leaves H13 off.
    pub fn corrected_legacy_h12() -> Self {
        let mut config = Self::legacy();
        config.freestyle_weights.softdrop = 0.0;
        config.freestyle_weights.pending_safety = 0.0;
        config.freestyle_weights.useful_attack_reward = 0.0;
        config.freestyle_weights.cancellation_reward = 0.0;
        config.freestyle_weights.h3_b2b_charge_value = 0.0;
        config.freestyle_weights.h3_surge_bank_value = 0.0;
        config.freestyle_weights.h6_base_holes_scale = 1.0;
        config.freestyle_weights.h6_base_coveredness_scale = 1.0;
        config.freestyle_weights.h9_cavity_excavation = 0.0;
        config.freestyle_speculated_exploitation = std::f64::consts::LN_2;
        config.dag_backprop_best_demotion = true;
        config.dag_backprop_despeculated_values = false;
        config
    }

    /// Canonical scored H14 configuration. Keep H13 off here so historical
    /// compute-strength evidence remains comparable.
    pub fn review_h9_h12() -> Self {
        let mut config = Self::legacy();
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
        config.freestyle_speculated_exploitation = std::f64::consts::LN_2;
        config.dag_backprop_best_demotion = true;
        config
    }

    /// Interactive replay-review profile: scored H9+H12 plus the H13
    /// persistent-DAG despeculation correctness fix.
    pub fn interactive_review() -> Self {
        let mut config = Self::review_h9_h12();
        config.dag_backprop_despeculated_values = true;
        config
    }
}

#[derive(Debug)]
pub struct BotOptions {
    pub speculate: bool,
    pub config: Arc<BotConfig>,
    /// Root-only Hold lock from the visible player state. Descendants may Hold
    /// normally because a lock/spawn resets Tetrp's one-Hold-per-piece gate.
    pub root_hold_locked: bool,
}
#[enum_dispatch]
enum ModeEnum { Freestyle }
#[enum_dispatch(ModeEnum)]
trait Mode {
    fn advance(&mut self, options: &BotOptions, mv: Placement) -> Option<ModeSwitch>;
    fn new_piece(&mut self, options: &BotOptions, piece: Piece);
    fn suggest(&self, options: &BotOptions) -> Vec<Placement>;
    fn ranked(&self) -> Vec<(Placement, f32)>;
    fn do_work(&self, options: &BotOptions, budget: u64) -> Statistics;
}
enum ModeSwitch { Freestyle }

impl Bot {
    pub fn new(options: BotOptions, root: GameState, queue: &[Piece]) -> Self {
        Bot {
            current: root,
            queue: queue.iter().copied().collect(),
            mode: Freestyle::new(&options, root, queue).into(),
            options,
            hold_is_empty: false,
        }
    }
    pub(crate) fn set_initial_empty_hold(&mut self, empty: bool) { self.hold_is_empty = empty; }

    /// Return real player-facing state, not CC2's normalized reserve/queue.
    pub fn player_pieces(&self) -> PlayerPieces {
        if self.hold_is_empty {
            PlayerPieces { current: Some(self.current.reserve), hold: None, next: self.queue.iter().copied().collect() }
        } else {
            PlayerPieces { current: self.queue.front().copied(), hold: Some(self.current.reserve), next: self.queue.iter().skip(1).copied().collect() }
        }
    }

    /// Newly revealed pieces needed to restore current + NEXT x5. Usually one,
    /// but two after the first actual hold. Do not reveal them before the move.
    pub fn preview_refill_needed(&self) -> usize {
        let target = if self.hold_is_empty { 5usize } else { 6usize };
        target.saturating_sub(self.queue.len())
    }

    fn inferred_hold(&self, mv: Placement) -> bool {
        self.player_pieces().current != Some(mv.location.piece)
    }

    /// Trusted core entrypoint used for the bot's own suggestions. For replay
    /// moves use try_play(), including the explicit hold decision.
    pub fn advance(&mut self, mv: Placement) -> PlacementInfo {
        let use_hold = self.inferred_hold(mv);
        self.advance_inner(mv, use_hold)
    }
    fn advance_inner(&mut self, mv: Placement, use_hold: bool) -> PlacementInfo {
        puffin::profile_function!();
        let info = self.current.advance(self.queue.pop_front().expect("cannot advance an exhausted search queue"), mv);
        if self.hold_is_empty && use_hold { self.hold_is_empty = false; }
        self.options.root_hold_locked = false;
        if let Some(to) = self.mode.advance(&self.options, mv) { self.switch(to); }
        info
    }

    pub fn try_advance(&mut self, mv: Placement) -> Result<PlacementInfo, String> {
        let use_hold = self.inferred_hold(mv);
        self.try_play(mv, use_hold)
    }

    /// Invalid, unreachable, wrong-piece or wrong-spin moves leave state intact.
    /// Explicit hold is necessary when current and NEXT[0] have the same type.
    pub fn try_play(&mut self, mv: Placement, use_hold: bool) -> Result<PlacementInfo, String> {
        if self.queue.is_empty() { return Err("search queue exhausted; supply the newly visible pieces first".into()); }
        if use_hold && self.options.root_hold_locked {
            return Err("Hold is already locked for this root position".into());
        }
        let pieces = self.player_pieces();
        let expected = if use_hold { pieces.hold.or_else(|| pieces.next.first().copied()) } else { pieces.current };
        if expected != Some(mv.location.piece) { return Err("placement does not match the current/hold decision".into()); }
        // Check membership before doing arithmetic on untrusted coordinates.
        // This also validates reachability and the spin flag using this core's
        // supported movement rules; it does NOT certify full SRS+ parity.
        let legal = find_moves_with_clutch(&self.current.board, mv.location.piece, self.current.rules.clutch && self.current.combo > 0);
        if !legal.iter().any(|&(candidate, _)| candidate == mv) {
            return Err("placement is not reachable with the supplied spin under the supported move generator".into());
        }
        Ok(self.advance_inner(mv, use_hold))
    }

    pub fn has_legal_move(&self) -> bool {
        if self.queue.is_empty() { return false; }
        let pieces = self.player_pieces();
        let current = match pieces.current { Some(p) => p, None => return false };
        let allow_clutch = self.current.rules.clutch && self.current.combo > 0;
        if !find_moves_with_clutch(&self.current.board, current, allow_clutch).is_empty() { return true; }
        if self.options.root_hold_locked { return false; }
        let held = pieces.hold.or_else(|| pieces.next.first().copied());
        held.map(|p| !find_moves_with_clutch(&self.current.board, p, allow_clutch).is_empty()).unwrap_or(false)
    }

    pub fn new_piece(&mut self, piece: Piece) {
        puffin::profile_function!();
        self.queue.push_back(piece);
        self.mode.new_piece(&self.options, piece);
    }
    pub fn suggest(&self) -> Vec<Placement> {
        puffin::profile_function!();
        self.mode.suggest(&self.options)
    }
    pub fn set_forecast(&mut self, forecast: crate::forecast::Forecast) {
        self.current.forecast = forecast;
        self.mode = Freestyle::new(&self.options, self.current, self.queue.make_contiguous()).into();
    }
    pub fn ranked_suggestions(&self) -> Vec<(Placement,f32)> { self.mode.ranked() }
    pub fn do_work(&self) -> Statistics {
        puffin::profile_function!();
        self.mode.do_work(&self.options, u64::MAX)
    }
    /// Hard evaluator-node allocation, shared by both sides of a KO experiment.
    pub fn do_work_limited(&self, budget: u64) -> Statistics {
        self.mode.do_work(&self.options, budget)
    }
    /// Search state. Use player_pieces() to interpret current/hold/NEXT.
    pub fn state(&self) -> GameState { self.current }

    pub fn add_garbage_line(&mut self, hole: usize) -> bool {
        assert!(hole < 10);
        let overflow = self.current.board.cols.iter().any(|&c| c >> 39 != 0);
        self.current.board.garbage_rows = (self.current.board.garbage_rows << 1 | 1) & ((1u64 << 40) - 1);
        for (x, col) in self.current.board.cols.iter_mut().enumerate() {
            *col <<= 1;
            if x != hole { *col |= 1; }
        }
        self.mode = Freestyle::new(&self.options, self.current, self.queue.make_contiguous()).into();
        overflow
    }
    fn switch(&mut self, to: ModeSwitch) {
        puffin::profile_function!();
        match to {
            ModeSwitch::Freestyle => self.mode = Freestyle::new(&self.options, self.current, self.queue.make_contiguous()).into(),
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Statistics { pub nodes: u64, pub selections: u64, pub expansions: u64, pub max_depth: usize, pub speculative_expansions: u64, pub budget_exhausted: bool }
impl Statistics {
    pub fn accumulate(&mut self, other: Self) {
        self.nodes += other.nodes;
        self.selections += other.selections;
        self.expansions += other.expansions;
        self.max_depth = self.max_depth.max(other.max_depth);
        self.speculative_expansions += other.speculative_expansions;
        self.budget_exhausted |= other.budget_exhausted;
    }
}

#[cfg(test)]
mod config_tests {
    use super::BotConfig;

    #[test]
    fn review_h9_h12_matches_scored_h14_configuration() {
        let legacy = BotConfig::legacy();
        let review = BotConfig::review_h9_h12();
        assert_eq!(review.freestyle_weights.pending_safety, 1.0);
        assert_eq!(review.freestyle_weights.useful_attack_reward, 1.0);
        assert_eq!(review.freestyle_weights.cancellation_reward, 0.0);
        assert_eq!(review.freestyle_weights.row_transitions, legacy.freestyle_weights.row_transitions * 2.5);
        assert_eq!(review.freestyle_weights.h9_cavity_excavation, -0.5);
        assert_eq!(review.freestyle_exploitation, std::f64::consts::LN_2);
        assert_eq!(review.freestyle_speculated_exploitation, std::f64::consts::LN_2);
        assert!(review.dag_backprop_best_demotion);
        assert!(!review.dag_backprop_despeculated_values);
        assert!(!review.freestyle_weights.tetrio_s2);
    }

    #[test]
    fn corrected_legacy_h12_is_neutral_reset_baseline() {
        let legacy = BotConfig::legacy();
        let reset = BotConfig::corrected_legacy_h12();
        assert_eq!(reset.freestyle_weights.row_transitions, legacy.freestyle_weights.row_transitions);
        assert_eq!(reset.freestyle_weights.height, legacy.freestyle_weights.height);
        assert_eq!(reset.freestyle_weights.combo_attack, legacy.freestyle_weights.combo_attack);
        assert_eq!(reset.freestyle_weights.pending_safety, 0.0);
        assert_eq!(reset.freestyle_weights.useful_attack_reward, 0.0);
        assert_eq!(reset.freestyle_weights.h3_b2b_charge_value, 0.0);
        assert_eq!(reset.freestyle_weights.h3_surge_bank_value, 0.0);
        assert_eq!(reset.freestyle_weights.h9_cavity_excavation, 0.0);
        assert_eq!(reset.freestyle_exploitation, std::f64::consts::LN_2);
        assert_eq!(reset.freestyle_speculated_exploitation, std::f64::consts::LN_2);
        assert!(reset.dag_backprop_best_demotion);
        assert!(!reset.dag_backprop_despeculated_values);
    }

    #[test]
    fn interactive_review_adds_h13_without_changing_scored_h14_profile() {
        let scored = BotConfig::review_h9_h12();
        let interactive = BotConfig::interactive_review();
        assert!(!scored.dag_backprop_despeculated_values);
        assert!(interactive.dag_backprop_despeculated_values);
        assert_eq!(
            interactive.freestyle_weights.row_transitions,
            scored.freestyle_weights.row_transitions
        );
        assert_eq!(
            interactive.freestyle_weights.h9_cavity_excavation,
            scored.freestyle_weights.h9_cavity_excavation
        );
        assert_eq!(
            interactive.dag_backprop_best_demotion,
            scored.dag_backprop_best_demotion
        );
    }
}
