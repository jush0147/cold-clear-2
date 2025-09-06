use crate::{
    bot::{Bot, BotConfig, BotOptions},
    data::GameState,
    sync::BotSyncronizer,
    tbp::{self, Randomizer},
};
use enumset::EnumSet;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_str(s: &str);
}

macro_rules! log {
    ($($t:tt)*) => (log_str(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub struct WasmBot {
    bot: Arc<BotSyncronizer>,
    config: Arc<BotConfig>,
}

#[wasm_bindgen]
impl WasmBot {
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<WasmBot, JsValue> {
        // Sets up a panic hook to log panics to the browser console
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        log!("Initializing Cold Clear Wasm bot...");

        let config: BotConfig = serde_wasm_bindgen::from_value(config)
            .map_err(|e| e.to_string())?;

        let bot = Arc::new(BotSyncronizer::new());

        Ok(WasmBot {
            bot,
            config: Arc::new(config),
        })
    }

    pub fn start(&self, start_info: JsValue) -> Result<(), JsValue> {
        let start: tbp::Start = serde_wasm_bindgen::from_value(start_info)
            .map_err(|e| e.to_string())?;

        self.bot.start(create_bot(start, self.config.clone()));
        Ok(())
    }

    pub fn stop(&self) {
        self.bot.stop();
    }

    pub fn set_search_depth(&self, depth: u64) {
        self.bot.set_search_depth(depth);
    }

    pub fn new_piece(&self, piece: JsValue) -> Result<(), JsValue> {
        let piece: crate::data::Piece = serde_wasm_bindgen::from_value(piece)
            .map_err(|e| e.to_string())?;
        self.bot.new_piece(piece);
        Ok(())
    }

    pub fn play(&self, mv: JsValue) -> Result<(), JsValue> {
        let mv: crate::data::Placement = serde_wasm_bindgen::from_value(mv)
            .map_err(|e| e.to_string())?;
        self.bot.advance(mv);
        Ok(())
    }

    pub fn suggest(&self) -> Result<JsValue, JsValue> {
        match self.bot.suggest() {
            Some((moves, move_info)) => {
                let suggestion = tbp::BotMessage::Suggestion { moves, move_info };
                serde_wasm_bindgen::to_value(&suggestion).map_err(|e| e.to_string().into())
            }
            None => Ok(JsValue::NULL),
        }
    }
}

// This is a helper function adapted from `lib.rs` to create a new bot instance.
// It's not exposed to Wasm.
fn create_bot(mut start: tbp::Start, config: Arc<BotConfig>) -> Bot {
    let reserve = start.hold.unwrap_or_else(|| start.queue.remove(0));

    let speculate = matches!(start.randomizer, Randomizer::SevenBag { .. });
    let bag = match start.randomizer {
        Randomizer::Unknown => EnumSet::all(),
        Randomizer::SevenBag { mut bag_state } => {
            for &p in start.queue.iter().rev() {
                if bag_state == EnumSet::all() {
                    bag_state = EnumSet::empty();
                }
                bag_state.insert(p);
            }
            bag_state
        }
    };

    let state = GameState {
        reserve,
        back_to_back: start.back_to_back,
        combo: start.combo.try_into().unwrap_or(255),
        bag,
        board: start.board.into(),
    };

    Bot::new(BotOptions { speculate, config }, state, &start.queue)
}
