//! Snapshot checker. No future events or RNG state are supplied to search.
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{bot::BotConfig, data::Placement, tbp::Start, tetrio, try_create_bot};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckInput { pub start: Start, pub placement: Placement }
#[derive(Serialize)]
pub struct CheckOutput {
    pub reachable: bool, pub lines: u32, pub garbage_cleared: u32,
    pub consecutive_clears: u8, pub back_to_back: bool, pub b2b_count: u16,
    pub perfect_clear: bool, pub attack: tetrio::AttackBreakdown, pub packets: Vec<u32>,
    pub board: Vec<Vec<Option<char>>>,
}
pub fn check(input: CheckInput) -> Result<CheckOutput,String> {
    let mut bot=try_create_bot(input.start,Arc::new(BotConfig::default()))?;
    let info=bot.try_play(input.placement,false)?;
    let state=bot.state();let attack=tetrio::attack_with_rules(&info,state.rules);
    let board=(0..40).map(|y|(0..10).map(|x|if state.board.cols[x] & (1u64<<y)!=0 {Some('X')} else {None}).collect()).collect();
    Ok(CheckOutput {reachable:true,lines:info.lines_cleared,garbage_cleared:info.garbage_cleared,
        consecutive_clears:state.combo,back_to_back:state.back_to_back,b2b_count:state.b2b_count,
        perfect_clear:info.perfect_clear,packets:attack.packets(),attack,board})
}
