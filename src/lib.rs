use std::sync::Arc;

use bot::{BotConfig, BotOptions};
use enumset::EnumSet;
use tbp::Randomizer;

use crate::bot::Bot;
use crate::data::{GameState, Placement, TetrioRules};

#[cfg(not(target_arch = "wasm32"))]
use std::convert::Infallible;
#[cfg(not(target_arch = "wasm32"))]
use futures::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::sync::BotSyncronizer;
#[cfg(not(target_arch = "wasm32"))]
use crate::tbp::{BotMessage, FrontendMessage};

pub mod bot;
mod dag;
pub mod tbp;
pub mod tetrio;
pub mod replay_check;
pub mod forecast;
pub mod ko_support;
pub mod analysis;
pub mod snapshot;
#[macro_use]
pub mod data;
mod map;
pub mod movegen;
mod srs_plus;
#[cfg(not(target_arch = "wasm32"))]
mod sync;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub async fn run(
    mut incoming: impl Stream<Item = FrontendMessage> + Unpin,
    mut outgoing: impl Sink<BotMessage, Error = Infallible> + Unpin,
    config: Arc<BotConfig>,
) {
    outgoing.send(BotMessage::Info {
        name: "Cold Clear 2",
        version: concat!(env!("CARGO_PKG_VERSION"), " ", env!("GIT_HASH")),
        author: "MinusKelvin",
        features: &[],
    }).await.unwrap();
    let bot = Arc::new(BotSyncronizer::new());
    spawn_workers(&bot);
    let mut waiting_on_first_piece = None;
    while let Some(msg) = incoming.next().await {
        match msg {
            FrontendMessage::Start(start) => {
                if start.hold.is_none() && start.queue.is_empty() {
                    waiting_on_first_piece = Some(start);
                } else { bot.start(create_bot(start, config.clone())); }
            }
            FrontendMessage::Stop => { bot.stop(); waiting_on_first_piece = None; }
            FrontendMessage::Suggest => {
                if let Some((moves, move_info)) = bot.suggest() {
                    outgoing.send(BotMessage::Suggestion { moves, move_info }).await.unwrap();
                }
            }
            FrontendMessage::Play { mv } => {
                bot.advance(mv);
                puffin::GlobalProfiler::lock().new_frame();
            }
            FrontendMessage::NewPiece { piece } => {
                if let Some(mut start) = waiting_on_first_piece.take() {
                    if let Randomizer::SevenBag { bag_state } = &mut start.randomizer {
                        if bag_state.is_empty() { *bag_state = EnumSet::all(); }
                        bag_state.remove(piece);
                    }
                    start.queue.push(piece);
                    bot.start(create_bot(start, config.clone()));
                } else { bot.new_piece(piece); }
            }
            FrontendMessage::Rules => { outgoing.send(BotMessage::Ready).await.unwrap(); }
            FrontendMessage::Quit => break,
            FrontendMessage::Unknown => {}
        }
    }
}

/// Player-facing checked constructor. The generic TBP constructor below is
/// retained for native protocol clients that stream the initial queue.
pub fn try_create_bot(start: tbp::Start, config: Arc<BotConfig>) -> Result<Bot, String> {
    start.validate()?;
    Ok(create_bot(start, config))
}

pub fn try_create_bot_with_rules(start: tbp::Start, config: Arc<BotConfig>, rules: TetrioRules) -> Result<Bot, String> {
    try_create_bot_with_context(start, config, rules, false)
}

pub fn try_create_bot_with_context(start: tbp::Start, config: Arc<BotConfig>, rules: TetrioRules, hold_locked: bool) -> Result<Bot, String> {
    start.validate()?;
    rules.validate()?;
    if hold_locked && start.hold.is_none() {
        return Err("hold_locked=true requires an occupied Hold slot".into());
    }
    Ok(create_bot_with_search_context(start, config, rules, hold_locked, false, None))
}

/// Snapshot-product constructor. The root action gate and authority-derived
/// placement allowlist are search constraints, not claims about Hold occupancy.
pub(crate) fn try_create_bot_with_search_context(
    start: tbp::Start,
    config: Arc<BotConfig>,
    rules: TetrioRules,
    root_hold_locked: bool,
    root_no_hold_only: bool,
    root_legal_placements: Option<Vec<Placement>>,
) -> Result<Bot, String> {
    start.validate()?;
    rules.validate()?;
    Ok(create_bot_with_search_context(
        start, config, rules, root_hold_locked, root_no_hold_only, root_legal_placements,
    ))
}

pub fn create_bot(start: tbp::Start, config: Arc<BotConfig>) -> Bot {
    create_bot_with_search_context(start, config, TetrioRules::default(), false, false, None)
}

fn create_bot_with_search_context(
    mut start: tbp::Start,
    config: Arc<BotConfig>,
    rules: TetrioRules,
    root_hold_locked: bool,
    root_no_hold_only: bool,
    root_legal_placements: Option<Vec<Placement>>,
) -> Bot {
    let hold_is_empty = start.hold.is_none();
    // Empty hold normalization is only internal: reserve represents current,
    // queue represents NEXT. Bot::player_pieces reverses this representation.
    let reserve = start.hold.unwrap_or_else(|| start.queue.remove(0));
    let speculate = matches!(start.randomizer, Randomizer::SevenBag { .. });
    let bag = match start.randomizer {
        Randomizer::Unknown => EnumSet::all(),
        Randomizer::SevenBag { mut bag_state } => {
            for &p in start.queue.iter().rev() {
                if bag_state == EnumSet::all() { bag_state = EnumSet::empty(); }
                bag_state.insert(p);
            }
            bag_state
        }
    };
    let state = GameState {
        reserve,
        back_to_back: start.back_to_back,
        b2b_count: start.b2b_count.try_into().unwrap_or(u16::MAX),
        combo: start.combo.try_into().unwrap_or(255),
        bag,
        board: start.board,
        rules,
        forecast: crate::forecast::Forecast::default(),
    };
    let mut bot = Bot::new(BotOptions {
        speculate,
        config,
        root_hold_locked,
        root_no_hold_only,
        root_hold_is_empty: hold_is_empty,
        root_legal_placements: root_legal_placements.map(Arc::new),
    }, state, &start.queue);
    bot.set_initial_empty_hold(hold_is_empty);
    bot
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_workers(bot: &Arc<BotSyncronizer>) {
    for _ in 0..1 {
        let bot = bot.clone();
        std::thread::spawn(move || bot.work_loop());
    }
}
