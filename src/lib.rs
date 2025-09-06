use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use serde::{Serialize, Deserialize};

use std::cell::RefCell;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;

use bot::{BotConfig, BotOptions, Statistics};
use enumset::EnumSet;
use futures::prelude::*;
use tbp::{Randomizer, MoveInfo};
use crate::data::{Placement, Piece};

use crate::bot::Bot;
use crate::data::GameState;
use crate::sync::BotSyncronizer;
use crate::tbp::{BotMessage, FrontendMessage};

// Keep original modules public for now, might be needed
pub mod bot;
pub mod dag;
pub mod tbp;
#[macro_use]
pub mod data;
pub mod map;
pub mod movegen;
pub mod sync;

// Global state for the bot, managed by thread_local! for Wasm's single-threaded environment.
thread_local! {
    static BOT_STATE: RefCell<Option<Bot>> = RefCell::new(None);
}

// The original `run` function is kept for native execution, but won't be used by Wasm
pub async fn run(
    mut incoming: impl Stream<Item = FrontendMessage> + Unpin,
    mut outgoing: impl Sink<BotMessage, Error = Infallible> + Unpin,
    config: Arc<BotConfig>,
) {
    outgoing
        .send(BotMessage::Info {
            name: "Cold Clear 2",
            version: concat!(env!("CARGO_PKG_VERSION"), " ", env!("GIT_HASH")),
            author: "MinusKelvin",
            features: &[],
        })
        .await
        .unwrap();

    let bot = Arc::new(BotSyncronizer::new());

    #[cfg(not(target_arch = "wasm32"))]
    spawn_workers(&bot);

    let mut waiting_on_first_piece = None;

    while let Some(msg) = incoming.next().await {
        match msg {
            FrontendMessage::Start(start) => {
                if start.hold.is_none() && start.queue.is_empty() {
                    waiting_on_first_piece = Some(start);
                } else {
                    bot.start(create_bot(start, config.clone()));
                }
            }
            FrontendMessage::Stop => {
                bot.stop();
                waiting_on_first_piece = None;
            }
            FrontendMessage::Suggest => {
                if let Some((moves, move_info)) = bot.suggest() {
                    outgoing
                        .send(BotMessage::Suggestion { moves, move_info })
                        .await
                        .unwrap();
                }
            }
            FrontendMessage::Play { mv } => {
                bot.advance(mv);
                puffin::GlobalProfiler::lock().new_frame();
            }
            FrontendMessage::NewPiece { piece } => {
                if let Some(mut start) = waiting_on_first_piece.take() {
                    if let Randomizer::SevenBag { bag_state } = &mut start.randomizer {
                        if bag_state.is_empty() {
                            *bag_state = EnumSet::all();
                        }
                        bag_state.remove(piece);
                    }
                    start.queue.push(piece);
                    bot.start(create_bot(start, config.clone()));
                } else {
                    bot.new_piece(piece);
                }
            }
            FrontendMessage::Rules => {
                outgoing.send(BotMessage::Ready).await.unwrap();
            }
            FrontendMessage::Quit => break,
            FrontendMessage::Unknown => {}
        }
    }
}

// This helper is needed by our WasmBot
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

#[cfg(not(target_arch = "wasm32"))]
fn spawn_workers(bot: &Arc<BotSyncronizer>) {
    for _ in 0..1 {
        let bot = bot.clone();
        std::thread::spawn(move || bot.work_loop());
    }
}

// Wrapper struct for serialization to match TBP suggestion format
#[derive(Serialize)]
struct Suggestion {
    moves: Vec<Placement>,
    info: MoveInfo,
}

#[wasm_bindgen]
pub fn bot_io(input: JsValue) -> Result<JsValue, JsValue> {
    // Set the panic hook for better debugging in the browser console.
    console_error_panic_hook::set_once();

    #[derive(Deserialize)]
    #[serde(tag = "type")]
    #[serde(rename_all = "lowercase")]
    enum Message {
        Start(tbp::Start),
        Suggest,
        Play { mv: Placement },
        NewPiece { piece: Piece },
    }

    let message: Message = serde_wasm_bindgen::from_value(input)?;

    BOT_STATE.with(|bot_state| {
        match message {
            Message::Start(start_info) => {
                let config = Arc::new(Default::default());
                let bot = create_bot(start_info, config);
                *bot_state.borrow_mut() = Some(bot);
                Ok(JsValue::NULL)
            }
            Message::Suggest => {
                if let Some(bot) = &mut *bot_state.borrow_mut() {
                    let start_time = Instant::now();
                    let mut total_stats = Statistics::default();
                    // The number of iterations could be made configurable in the future.
                    for _ in 0..1000 {
                        total_stats.accumulate(bot.do_work());
                    }
                    let elapsed = start_time.elapsed().as_secs_f64();

                    let moves = bot.suggest();
                    if !moves.is_empty() {
                        let nodes = total_stats.nodes;
                        let nps = if elapsed > 0.0 { nodes as f64 / elapsed } else { 0.0 };

                        let move_info = MoveInfo {
                            nodes,
                            nps,
                            extra: String::new(),
                        };

                        let suggestion = Suggestion { moves, info: move_info };
                        serde_wasm_bindgen::to_value(&suggestion).map_err(|e| e.into())
                    } else {
                        Ok(JsValue::NULL)
                    }
                } else {
                    // Bot not initialized
                    Ok(JsValue::NULL)
                }
            }
            Message::Play { mv } => {
                if let Some(bot) = &mut *bot_state.borrow_mut() {
                    bot.advance(mv);
                }
                Ok(JsValue::NULL)
            }
            Message::NewPiece { piece } => {
                if let Some(bot) = &mut *bot_state.borrow_mut() {
                    bot.new_piece(piece);
                }
                Ok(JsValue::NULL)
            }
        }
    })
}
