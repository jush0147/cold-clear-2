use enumset::{EnumSet, EnumSetType};
use serde::{Deserialize, Serialize};

use crate::data::{Board, Piece, Placement};

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum FrontendMessage {
    Rules,
    Start(Start),
    Play { #[serde(rename = "move")] mv: Placement },
    NewPiece { piece: Piece },
    Suggest,
    Stop,
    Quit,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum BotMessage {
    Info { name: &'static str, version: &'static str, author: &'static str, features: &'static [&'static str] },
    Ready,
    Suggestion { moves: Vec<Placement>, move_info: MoveInfo },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    pub board: Board,
    /// Current piece followed by visible NEXT pieces. Board rows are bottom-up.
    pub queue: Vec<Piece>,
    pub hold: Option<Piece>,
    /// Consecutive clears before the next placement, NOT the previous displayed combo.
    pub combo: u32,
    pub back_to_back: bool,
    #[serde(default)]
    pub b2b_count: u32,
    #[serde(default)]
    pub randomizer: Randomizer,
}

impl Start {
    /// Checked boundary for browser/replay callers. Core state transitions remain
    /// trusted APIs; callers must not silently clamp unsupported counters.
    pub fn validate(&self) -> Result<(), String> {
        if self.queue.is_empty() {
            return Err("a current piece is required".into());
        }
        if self.hold.is_none() && self.queue.len() < 2 {
            return Err("empty-hold search currently needs at least one visible NEXT piece".into());
        }
        if self.queue.len() > 6 {
            return Err("player-visible input is limited to current + five NEXT pieces".into());
        }
        if self.combo > u8::MAX as u32 || self.b2b_count > u16::MAX as u32 {
            return Err("combo or B2B counter exceeds the core's supported range".into());
        }
        if !self.back_to_back && self.b2b_count != 0 {
            return Err("nonzero B2B count requires an active B2B chain".into());
        }
        if self.board.cols.iter().any(|&c| c >> 40 != 0) {
            return Err("board cells above row 39 are unsupported".into());
        }
        if self.board.line_clears() != 0 {
            return Err("snapshot contains uncleared full rows; provide a post-clear decision state".into());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Randomizer {
    SevenBag { #[serde(deserialize_with = "collect_enumset")] bag_state: EnumSet<Piece> },
    #[serde(other)]
    Unknown,
}
impl Default for Randomizer { fn default() -> Self { Self::Unknown } }

#[derive(Serialize)]
pub struct MoveInfo { pub nodes: u64, pub nps: f64, pub extra: String }

impl TryFrom<Vec<[Option<char>; 10]>> for Board {
    type Error = String;
    fn try_from(rows: Vec<[Option<char>; 10]>) -> Result<Self, Self::Error> {
        if rows.len() > 40 {
            return Err(format!("board has {} rows; maximum supported is 40", rows.len()));
        }
        let mut cols = [0u64; 10];
        let mut garbage_rows = 0u64;
        // Short bottom-up boards are explicitly padded with empty rows above.
        // Never index a 20-row UI board as if it already contained 40 rows.
        for (y, row) in rows.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if cell.is_some() { cols[x] |= 1u64 << y; }
                if matches!(cell, Some('G') | Some('g')) { garbage_rows |= 1u64 << y; }
            }
        }
        Ok(Board { cols, garbage_rows })
    }
}

fn collect_enumset<'de, D, T>(de: D) -> Result<EnumSet<T>, D::Error>
where D: serde::Deserializer<'de>, T: EnumSetType + Deserialize<'de> {
    Ok(Vec::<T>::deserialize(de)?.into_iter().collect())
}
