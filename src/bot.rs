use std::collections::VecDeque;
use std::sync::Arc;

use enum_dispatch::enum_dispatch;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::data::{GameState, Piece, Placement, PlacementInfo};

mod freestyle;

use self::freestyle::Freestyle;

pub struct Bot {
    options: BotOptions,
    current: GameState,
    queue: VecDeque<Piece>,
    mode: ModeEnum,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BotConfig {
    pub freestyle_weights: freestyle::Weights,
    pub freestyle_exploitation: f64,
}

impl Default for BotConfig {
    fn default() -> Self {
        static DEFAULT: Lazy<BotConfig> =
            Lazy::new(|| serde_json::from_str(include_str!("default.json")).unwrap());
        DEFAULT.clone()
    }
}

impl BotConfig {
    /// Same board heuristics and search parameters, but with the original
    /// Cold Clear 2 clear-type reward model for A/B comparisons.
    pub fn legacy() -> Self {
        let mut config = Self::default();
        config.freestyle_weights.tetrio_s2 = false;
        config
    }

    /// Experimental TL S2 configuration with explicit values for parameter
    /// sweeps. Keeping this constructor here prevents benchmark binaries from
    /// reaching through the private freestyle module.
    pub fn tetrio_s2(charge_value: f32, surge_value: f32) -> Self {
        let mut config = Self::default();
        config.freestyle_weights.tetrio_s2 = true;
        config.freestyle_weights.b2b_charge_value = charge_value;
        config.freestyle_weights.surge_value = surge_value;
        config
    }
}

#[derive(Debug)]
pub struct BotOptions {
    pub speculate: bool,
    pub config: Arc<BotConfig>,
}

#[enum_dispatch]
enum ModeEnum {
    Freestyle,
}

#[enum_dispatch(ModeEnum)]
trait Mode {
    fn advance(&mut self, options: &BotOptions, mv: Placement) -> Option<ModeSwitch>;
    fn new_piece(&mut self, options: &BotOptions, piece: Piece);
    fn suggest(&self, options: &BotOptions) -> Vec<Placement>;
    fn do_work(&self, options: &BotOptions) -> Statistics;
}

enum ModeSwitch {
    Freestyle,
}

impl Bot {
    pub fn new(options: BotOptions, root: GameState, queue: &[Piece]) -> Self {
        Bot {
            current: root,
            queue: queue.iter().copied().collect(),
            mode: Freestyle::new(&options, root, queue).into(),
            options,
        }
    }

    pub fn advance(&mut self, mv: Placement) -> PlacementInfo {
        puffin::profile_function!();
        let info = self.current.advance(self.queue.pop_front().unwrap(), mv);
        if let Some(to) = self.mode.advance(&self.options, mv) {
            self.switch(to);
        };
        info
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

    pub fn do_work(&self) -> Statistics {
        puffin::profile_function!();
        self.mode.do_work(&self.options)
    }

    pub fn state(&self) -> GameState {
        self.current
    }

    /// Inject one garbage line at the bottom of the board and rebuild the
    /// search tree around the changed state. Returns true if blocks overflowed
    /// the 40-row simulation board.
    pub fn add_garbage_line(&mut self, hole: usize) -> bool {
        assert!(hole < 10);
        let overflow = self.current.board.cols.iter().any(|&c| c >> 39 != 0);
        for (x, col) in self.current.board.cols.iter_mut().enumerate() {
            *col <<= 1;
            if x != hole {
                *col |= 1;
            }
        }
        self.mode = Freestyle::new(
            &self.options,
            self.current,
            self.queue.make_contiguous(),
        )
        .into();
        overflow
    }

    fn switch(&mut self, to: ModeSwitch) {
        puffin::profile_function!();
        match to {
            ModeSwitch::Freestyle => {
                self.mode =
                    Freestyle::new(&self.options, self.current, self.queue.make_contiguous()).into()
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Statistics {
    pub nodes: u64,
    pub selections: u64,
    pub expansions: u64,
}

impl Default for Statistics {
    fn default() -> Self {
        Statistics {
            nodes: 0,
            selections: 0,
            expansions: 0,
        }
    }
}

impl Statistics {
    pub fn accumulate(&mut self, other: Self) {
        self.nodes += other.nodes;
        self.selections += other.selections;
        self.expansions += other.expansions;
    }
}
