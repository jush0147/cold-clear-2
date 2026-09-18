use std::sync::Arc;
use wasm_bindgen::prelude::*;
use crate::bot::{Bot, BotConfig, Statistics};
use crate::data::{Piece, Placement};
use crate::tbp::Start;
use crate::tetrio::garbage::GarbageQueue;
use crate::try_create_bot;

#[wasm_bindgen]
pub struct WasmBot { bot: Option<Bot>, config: Arc<BotConfig>, stats: Statistics }

#[wasm_bindgen]
impl WasmBot {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmBot {
        WasmBot { bot: None, config: Arc::new(BotConfig::interactive_review()), stats: Statistics::default() }
    }

    pub fn start(&mut self, start_json: &str) -> Result<(), JsValue> {
        let value = parse_json(start_json)?;
        let object = value.as_object().ok_or_else(|| js_error("start must be an object"))?;
        const FIELDS: &[&str] = &["board", "queue", "hold", "combo", "back_to_back", "b2b_count", "randomizer"];
        for key in object.keys() {
            if !FIELDS.contains(&key.as_str()) {
                return Err(js_error(format!("unsupported start field '{key}'; pending garbage is NOT yet integrated in DAG search and must not be silently ignored")));
            }
        }
        let start: Start = serde_json::from_value(value).map_err(js_error)?;
        if start.queue.len() != 6 { return Err(js_error("browser start requires current + exactly five visible NEXT pieces")); }
        let bot = try_create_bot(start, self.config.clone()).map_err(js_error)?;
        self.bot = Some(bot);
        self.stats = Statistics::default();
        Ok(())
    }

    /// Compatibility API using CC2 work iterations. Product review code should
    /// use think_nodes() so native and WASM use the same evaluator-node budget.
    pub fn think(&mut self, iterations: u32) -> Result<u64, JsValue> {
        let bot = self.bot.as_ref().ok_or_else(|| js_error("bot has not been started"))?;
        if bot.preview_refill_needed() != 0 { return Err(js_error("supply newly visible NEXT pieces before searching")); }
        let mut nodes = 0;
        for _ in 0..iterations {
            let stats = bot.do_work();
            nodes += stats.nodes;
            self.stats.accumulate(stats);
        }
        Ok(nodes)
    }

    /// Hard evaluator-node budget, matching H14's compute unit. Search may
    /// return fewer nodes only if the current graph has no further work.
    pub fn think_nodes(&mut self, node_budget: u32) -> Result<u64, JsValue> {
        if node_budget == 0 { return Err(js_error("node budget must be positive")); }
        let bot = self.bot.as_ref().ok_or_else(|| js_error("bot has not been started"))?;
        if bot.preview_refill_needed() != 0 { return Err(js_error("supply newly visible NEXT pieces before searching")); }

        let budget = node_budget as u64;
        let mut searched = Statistics::default();
        let mut stalled = 0u32;
        while searched.nodes < budget && stalled < 1024 {
            let step = bot.do_work_limited(budget - searched.nodes);
            let stop = step.budget_exhausted;
            stalled = if step.nodes == 0 { stalled + 1 } else { 0 };
            searched.accumulate(step);
            if stop { break; }
        }
        if searched.nodes > budget { return Err(js_error("hard node budget exceeded")); }
        let nodes = searched.nodes;
        self.stats.accumulate(searched);
        Ok(nodes)
    }
    pub fn suggest_json(&self) -> Result<String, JsValue> {
        let bot = self.bot.as_ref().ok_or_else(|| js_error("bot has not been started"))?;
        if bot.preview_refill_needed() != 0 { return Err(js_error("supply newly visible NEXT pieces before requesting suggestions")); }
        serde_json::to_string(&bot.suggest()).map_err(js_error)
    }
    /// Compatibility API: infer hold from piece type. Replays should use the
    /// explicit API below to distinguish holding identical current/NEXT pieces.
    pub fn play_json(&mut self, placement_json: &str) -> Result<(), JsValue> {
        let placement: Placement = serde_json::from_value(parse_json(placement_json)?).map_err(js_error)?;
        let bot = self.bot.as_mut().ok_or_else(|| js_error("bot has not been started"))?;
        if bot.preview_refill_needed() != 0 { return Err(js_error("restore visible previews before advancing again")); }
        bot.try_advance(placement).map_err(js_error)?;
        self.stats = Statistics::default();
        Ok(())
    }
    pub fn play_with_hold_json(&mut self, placement_json: &str, use_hold: bool) -> Result<(), JsValue> {
        let placement: Placement = serde_json::from_value(parse_json(placement_json)?).map_err(js_error)?;
        let bot = self.bot.as_mut().ok_or_else(|| js_error("bot has not been started"))?;
        if bot.preview_refill_needed() != 0 { return Err(js_error("restore visible previews before advancing again")); }
        bot.try_play(placement, use_hold).map_err(js_error)?;
        self.stats = Statistics::default();
        Ok(())
    }
    pub fn preview_refill_needed(&self) -> Result<u32, JsValue> {
        let bot = self.bot.as_ref().ok_or_else(|| js_error("bot has not been started"))?;
        Ok(bot.preview_refill_needed() as u32)
    }
    pub fn new_piece(&mut self, piece: &str) -> Result<(), JsValue> {
        let piece = parse_piece(piece)?;
        let bot = self.bot.as_mut().ok_or_else(|| js_error("bot has not been started"))?;
        if bot.preview_refill_needed() == 0 { return Err(js_error("visible queue is full; refusing a hidden future piece")); }
        bot.new_piece(piece);
        Ok(())
    }
    pub fn player_state_json(&self) -> Result<String, JsValue> {
        let bot = self.bot.as_ref().ok_or_else(|| js_error("bot has not been started"))?;
        let state = bot.state();
        let rows: Vec<Vec<Option<char>>> = (0..40).map(|y| (0..10).map(|x| if state.board.cols[x] & (1u64 << y) != 0 { Some('X') } else { None }).collect()).collect();
        serde_json::to_string(&serde_json::json!({
            "board": rows,
            "pieces": bot.player_pieces(),
            "consecutive_clears": state.combo,
            "back_to_back": state.back_to_back,
            "b2b_count": state.b2b_count,
            "preview_refill_needed": bot.preview_refill_needed(),
        })).map_err(js_error)
    }
    pub fn capabilities_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&serde_json::json!({
            "rules_parity_verified": crate::tetrio::PARITY_VERIFIED,
            "next_limit": 5,
            "explicit_hold_playback": true,
            "pending_garbage_in_search": false,
            "normal_garbage_queue_primitives": true,
            "movement": "SRS+ CW/CCW/180 with Clutch spawn rescue; replay-fixture checked",
            "pending_forecast_api": "analyze_pending_json",
            "config_profile": "h9+h12+h13-interactive",
            "despeculation_backprop": true,
            "hard_node_budget": true,
            "persistent_dag": true,
            "clutch_clears": false,
            "garbage_special_bonus": true,
            "opening_double_cancel": false,
        })).map_err(js_error)
    }
    pub fn stats_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&serde_json::json!({
            "nodes": self.stats.nodes,
            "selections": self.stats.selections,
            "expansions": self.stats.expansions,
            "max_depth": self.stats.max_depth,
            "speculative_expansions": self.stats.speculative_expansions,
            "budget_exhausted": self.stats.budget_exhausted,
        })).map_err(js_error)
    }
    pub fn reset_stats(&mut self) { self.stats = Statistics::default(); }
}

/// Diagnostic normal-cancel transition only. No board prediction, activation
/// forecast or opening double-cancel is implied by this API.
#[wasm_bindgen]
pub fn preview_garbage_one_to_one(queue_json: &str, attack: u32, cleared_lines: u32, cap: u32) -> Result<String, JsValue> {
    let queue: GarbageQueue = serde_json::from_value(parse_json(queue_json)?).map_err(js_error)?;
    serde_json::to_string(&queue.resolve_one_to_one(attack, cleared_lines, cap)).map_err(js_error)
}
fn parse_json(text: &str) -> Result<serde_json::Value, JsValue> {
    if text.len() > 262_144 { return Err(js_error("JSON request exceeds the 256 KiB boundary limit")); }
    serde_json::from_str(text).map_err(js_error)
}
fn parse_piece(piece: &str) -> Result<Piece, JsValue> {
    match piece.trim().to_ascii_uppercase().as_str() {
        "I" => Ok(Piece::I), "O" => Ok(Piece::O), "T" => Ok(Piece::T),
        "L" => Ok(Piece::L), "J" => Ok(Piece::J), "S" => Ok(Piece::S), "Z" => Ok(Piece::Z),
        _ => Err(js_error("piece must be one of I, O, T, L, J, S, Z")),
    }
}
fn js_error(err: impl std::fmt::Display) -> JsValue { JsValue::from_str(&err.to_string()) }

/// Replay diagnostics at a fresh-spawn decision boundary. Counts and attack
/// packets can be compared against independently reconstructed replay locks.
#[wasm_bindgen]
pub fn check_replay_lock_json(input_json: &str) -> Result<String, JsValue> {
    let input=serde_json::from_value(parse_json(input_json)?).map_err(js_error)?;
    serde_json::to_string(&crate::replay_check::check(input).map_err(js_error)?).map_err(js_error)
}

/// Pending-aware, snapshot-only search across ten hypothetical hole scenarios.
/// Returns the timing/hole assumptions alongside suggestions, never a claim of
/// exact future replay simulation.
#[wasm_bindgen]
pub fn analyze_pending_json(input_json:&str)->Result<String,JsValue>{
    let input=serde_json::from_value(parse_json(input_json)?).map_err(js_error)?;
    serde_json::to_string(&crate::analysis::analyze(input).map_err(js_error)?).map_err(js_error)
}
