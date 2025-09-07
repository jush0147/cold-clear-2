use std::convert::Infallible;
use std::sync::Arc;

use bot::{BotConfig, BotOptions};
use enumset::EnumSet;
use futures::prelude::*;
use tbp::Randomizer;

use crate::bot::Bot;
use crate::data::GameState;
use crate::sync::BotSyncronizer;
use crate::tbp::{BotMessage, FrontendMessage};

mod bot;
mod dag;
mod tbp;
#[macro_use]
pub mod data;
mod map;
pub mod movegen;
mod sync;

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

#[cfg(target_arch = "wasm32")]
fn spawn_workers(_bot: &Arc<BotSyncronizer>) {
    // no-op on wasm
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use console_error_panic_hook;
    use futures::channel::mpsc;
    use js_sys::Function;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::spawn_local;
    use futures::SinkExt;

    #[wasm_bindgen]
    pub struct ColdClearBot {
        incoming_tx: mpsc::Sender<FrontendMessage>,
    }

    #[wasm_bindgen]
    impl ColdClearBot {
        #[wasm_bindgen(constructor)]
        pub fn new(on_message: Function, config_json: &str) -> ColdClearBot {
            console_error_panic_hook::set_once();

            let config = if config_json.is_empty() {
                Arc::new(BotConfig::default())
            } else {
                Arc::new(serde_json::from_str(config_json).unwrap())
            };

            let (incoming_tx, incoming_rx) = mpsc::channel::<FrontendMessage>(8);
            let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<BotMessage>(8);

            // Spawn the main bot loop
            let outgoing_sink = futures::sink::unfold(outgoing_tx, |mut tx, msg: BotMessage| async move {
                tx.send(msg).await.expect("Bot runner panicked");
                Ok::<_, Infallible>(tx)
            });
            let pinned_sink = Box::pin(outgoing_sink);
            spawn_local(run(incoming_rx, pinned_sink, config));

            // Spawn a task to listen for outgoing messages and forward them to JS
            spawn_local(async move {
                while let Some(msg) = outgoing_rx.next().await {
                    let s = serde_json::to_string(&msg).unwrap();
                    let this = JsValue::null();
                    let s = JsValue::from_str(&s);
                    on_message.call1(&this, &s).unwrap();
                }
            });

            ColdClearBot { incoming_tx }
        }

        pub fn send_message(&mut self, message: &str) {
            let msg: FrontendMessage = serde_json::from_str(message).unwrap();
            self.incoming_tx.try_send(msg).unwrap();
        }
    }
}
