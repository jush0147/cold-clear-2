use std::sync::Arc;

use wasm_bindgen::prelude::*;

use crate::bot::{Bot, BotConfig, Statistics};
use crate::data::{Piece, Placement};
use crate::tbp::Start;
use crate::create_bot;

#[wasm_bindgen]
pub struct WasmBot {
    bot: Option<Bot>,
    config: Arc<BotConfig>,
    stats: Statistics,
}

#[wasm_bindgen]
impl WasmBot {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmBot {
        WasmBot {
            bot: None,
            config: Arc::new(BotConfig::default()),
            stats: Statistics::default(),
        }
    }

    pub fn start(&mut self, start_json: &str) -> Result<(), JsValue> {
        let start: Start = serde_json::from_str(start_json).map_err(js_error)?;
        if start.hold.is_none() && start.queue.is_empty() {
            return Err(JsValue::from_str(
                "start requires either a hold piece or at least one queued piece",
            ));
        }

        self.bot = Some(create_bot(start, self.config.clone()));
        self.stats = Statistics::default();
        Ok(())
    }

    pub fn think(&mut self, iterations: u32) -> Result<u64, JsValue> {
        let bot = self
            .bot
            .as_ref()
            .ok_or_else(|| JsValue::from_str("bot has not been started"))?;

        let mut nodes = 0;
        for _ in 0..iterations {
            let new_stats = bot.do_work();
            nodes += new_stats.nodes;
            self.stats.accumulate(new_stats);
        }
        Ok(nodes)
    }

    pub fn suggest_json(&self) -> Result<String, JsValue> {
        let bot = self
            .bot
            .as_ref()
            .ok_or_else(|| JsValue::from_str("bot has not been started"))?;
        serde_json::to_string(&bot.suggest()).map_err(js_error)
    }

    pub fn play_json(&mut self, placement_json: &str) -> Result<(), JsValue> {
        let placement: Placement = serde_json::from_str(placement_json).map_err(js_error)?;
        let bot = self
            .bot
            .as_mut()
            .ok_or_else(|| JsValue::from_str("bot has not been started"))?;
        bot.advance(placement);
        self.stats = Statistics::default();
        Ok(())
    }

    pub fn new_piece(&mut self, piece: &str) -> Result<(), JsValue> {
        let piece = parse_piece(piece)?;
        let bot = self
            .bot
            .as_mut()
            .ok_or_else(|| JsValue::from_str("bot has not been started"))?;
        bot.new_piece(piece);
        Ok(())
    }

    pub fn stats_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&serde_json::json!({
            "nodes": self.stats.nodes,
            "selections": self.stats.selections,
            "expansions": self.stats.expansions,
        }))
        .map_err(js_error)
    }

    pub fn reset_stats(&mut self) {
        self.stats = Statistics::default();
    }
}

fn parse_piece(piece: &str) -> Result<Piece, JsValue> {
    match piece.trim().to_ascii_uppercase().as_str() {
        "I" => Ok(Piece::I),
        "O" => Ok(Piece::O),
        "T" => Ok(Piece::T),
        "L" => Ok(Piece::L),
        "J" => Ok(Piece::J),
        "S" => Ok(Piece::S),
        "Z" => Ok(Piece::Z),
        _ => Err(JsValue::from_str("piece must be one of I, O, T, L, J, S, Z")),
    }
}

fn js_error(err: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&err.to_string())
}
