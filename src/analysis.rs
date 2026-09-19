//! Snapshot-only pending-aware analysis. This is a scenario approximation,
//! not a certified real-time TETR.IO emulator or a live-game automation API.
use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    bot::{BotConfig, Statistics},
    data::Placement,
    forecast::Forecast,
    ko_support::with_search_seed,
    tbp::{Randomizer, Start},
    try_create_bot,
};

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(deny_unknown_fields)]
pub struct IncomingPacket {
    pub lines: u32,
    /// Backward-compatible snapshot transport. New authority adapters should
    /// prefer ready_in_frames so packet age is not flattened.
    #[serde(default)]
    pub active: Option<bool>,
    /// Remaining frames until this already-observable packet can become active.
    /// Zero means active now. This is timing derivable from own observed history,
    /// not a hidden future arrival.
    #[serde(default)]
    pub ready_in_frames: Option<u32>,
}

impl IncomingPacket {
    fn timing(self, fallback_delay: u32) -> Result<(u32,u32), String> {
        if self.lines == 0 || self.lines > 1000 {
            return Err("packet line count must be 1..1000".into());
        }
        match (self.active, self.ready_in_frames) {
            (Some(true), Some(0)) => Ok((self.lines, 0)),
            (Some(true), Some(_)) => Err("active packet cannot have a positive ready_in_frames".into()),
            (Some(false), Some(0)) => Err("inactive packet cannot have zero ready_in_frames".into()),
            (_, Some(delay)) if delay <= 600 => Ok((self.lines, delay)),
            (_, Some(_)) => Err("packet activation delay exceeds analysis bound".into()),
            (Some(true), None) => Ok((self.lines, 0)),
            (Some(false), None) if fallback_delay <= 600 => Ok((self.lines, fallback_delay)),
            (Some(false), None) => Err("pending_delay_frames exceeds analysis bound".into()),
            (None, None) => Err("incoming packet needs active or ready_in_frames".into()),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub start: Start,
    pub incoming: Vec<IncomingPacket>,
    pub pieces_placed: u32,
    pub garbage_sent: u32,
    /// Explicit pace assumption, not actual future replay timing.
    pub frames_per_piece: u32,
    /// Legacy fallback for callers that provide only active=false. New Tetrp
    /// authority adapters provide ready_in_frames on every packet.
    pub pending_delay_frames: u32,
    /// Total evaluator-node budget across the modeled hole scenarios.
    pub node_budget: u32,
}

#[derive(Serialize)]
pub struct Candidate {
    pub placement: Placement,
    pub mean_score: f64,
    pub worst_score: f32,
    pub scenarios: u32,
}

#[derive(Serialize)]
pub struct Report {
    pub candidates: Vec<Candidate>,
    pub nodes: u64,
    pub node_budget: u32,
    pub scenarios: u32,
    pub config_profile: &'static str,
    pub pending_garbage_in_search: bool,
    pub rules_parity_verified: bool,
    pub frames_per_piece: u32,
    pub pending_delay_frames: u32,
    pub per_packet_ready_timing: bool,
    pub assumptions: Vec<&'static str>,
}

fn copy_randomizer(randomizer: &Randomizer) -> Randomizer {
    match randomizer {
        Randomizer::SevenBag { bag_state } => Randomizer::SevenBag {
            bag_state: bag_state.clone(),
        },
        Randomizer::Unknown => Randomizer::Unknown,
    }
}

fn placement_key(m: Placement) -> (u8, i8, i8, u8, u8) {
    (
        m.location.piece as u8,
        m.location.x,
        m.location.y,
        m.location.rotation as u8,
        m.spin as u8,
    )
}

pub fn analyze(request: Request) -> Result<Report, String> {
    analyze_with_profile(request, "review_h9_h12")
}

pub fn analyze_with_profile(request: Request, profile: &str) -> Result<Report, String> {
    request.start.validate()?;
    if request.start.queue.len() != 6 {
        return Err("exactly current + five NEXT pieces are required".into());
    }
    if !(1000..=2_000_000).contains(&request.node_budget) {
        return Err("node_budget must be 1000..2000000 evaluator nodes".into());
    }
    let per_packet_ready_timing = request.incoming.iter().all(|p| p.ready_in_frames.is_some());
    let timed_incoming: Vec<(u32,u32)> = request.incoming.iter().copied()
        .map(|p| p.timing(request.pending_delay_frames))
        .collect::<Result<_,_>>()?;

    let scenarios: u32 = if request.incoming.is_empty() { 1 } else { 10 };
    let mut scores: HashMap<Placement, (f64, f32, u32)> = HashMap::new();
    let mut nodes = 0u64;
    let (config, profile_label): (Arc<BotConfig>, &'static str) = match profile {
        "review_h9_h12" => (Arc::new(BotConfig::review_h9_h12()), "h9+h12-review"),
        "corrected_legacy_h12" => {
            let mut c = BotConfig::legacy();
            c.freestyle_weights.softdrop = 0.0;
            c.freestyle_weights.pending_safety = 0.0;
            c.freestyle_weights.useful_attack_reward = 0.0;
            c.freestyle_weights.cancellation_reward = 0.0;
            c.freestyle_weights.h3_b2b_charge_value = 0.0;
            c.freestyle_weights.h3_surge_bank_value = 0.0;
            c.freestyle_weights.h6_base_holes_scale = 1.0;
            c.freestyle_weights.h6_base_coveredness_scale = 1.0;
            c.freestyle_weights.h9_cavity_excavation = 0.0;
            c.dag_backprop_best_demotion = true;
            c.dag_backprop_despeculated_values = false;
            (Arc::new(c), "corrected-legacy+h12")
        }
        _ => return Err("profile must be review_h9_h12 or corrected_legacy_h12".into()),
    };

    for scenario in 0..scenarios {
        let start = Start {
            board: request.start.board,
            queue: request.start.queue.clone(),
            hold: request.start.hold,
            combo: request.start.combo,
            back_to_back: request.start.back_to_back,
            b2b_count: request.start.b2b_count,
            randomizer: copy_randomizer(&request.start.randomizer),
        };
        let forecast = Forecast::new_timed(
            &timed_incoming,
            request.pieces_placed,
            request.garbage_sent,
            request.frames_per_piece,
            scenario,
        )?;
        let mut bot = try_create_bot(start, config.clone())?;
        bot.set_forecast(forecast);

        let allocation = request.node_budget as u64 / scenarios as u64
            + u64::from(scenario < request.node_budget % scenarios);
        let mut stats = Statistics::default();
        let mut stalled = 0u32;
        with_search_seed(
            0xC01D_C1EAu64
                ^ (request.pieces_placed as u64).wrapping_mul(0x9E3779B97F4A7C15)
                ^ scenario as u64,
            || {
                while stats.nodes < allocation && stalled < 1024 {
                    let step = bot.do_work_limited(allocation - stats.nodes);
                    let stop = step.budget_exhausted;
                    stalled = if step.nodes == 0 { stalled + 1 } else { 0 };
                    stats.accumulate(step);
                    if stop {
                        break;
                    }
                }
            },
        );
        if stats.nodes > allocation {
            return Err("node allocation exceeded".into());
        }
        nodes += stats.nodes;

        for (placement, score) in bot.ranked_suggestions() {
            let entry = scores
                .entry(placement)
                .or_insert((0.0, f32::INFINITY, 0));
            entry.0 += score as f64;
            entry.1 = entry.1.min(score);
            entry.2 += 1;
        }
    }

    let mut candidates: Vec<_> = scores
        .into_iter()
        .filter(|(_, v)| v.2 == scenarios)
        .map(|(placement, (sum, worst, count))| Candidate {
            placement,
            mean_score: sum / count as f64,
            worst_score: worst,
            scenarios: count,
        })
        .collect();
    candidates.sort_by(|a, b| {
        b.mean_score.total_cmp(&a.mean_score)
            .then_with(|| placement_key(a.placement).cmp(&placement_key(b.placement)))
    });

    Ok(Report {
        candidates,
        nodes,
        node_budget: request.node_budget,
        scenarios,
        config_profile: profile_label,
        pending_garbage_in_search: true,
        rules_parity_verified: false,
        frames_per_piece: request.frames_per_piece,
        pending_delay_frames: request.pending_delay_frames,
        per_packet_ready_timing,
        assumptions: vec![
            "Only already observable incoming packets are modeled; no opponent board or future attacks.",
            "Unknown holes use ten equally weighted clean-hole scenarios when incoming garbage exists.",
            "Per-packet ready_in_frames is used when supplied; active-only callers fall back to pending_delay_frames.",
            "The caller must derive SevenBag bag_state only from information already visible in replay history.",
            "The hard evaluator-node budget is divided across modeled hole scenarios.",
            "Per-scenario future search can be optimistic about information revealed later; scores are heuristic, not win probabilities.",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Board, Piece};
    use enumset::EnumSet;

    fn empty_request() -> Request {
        let mut bag_state = EnumSet::empty();
        bag_state.insert(Piece::Z);
        Request {
            start: Start {
                board: Board::default(),
                queue: vec![Piece::I, Piece::O, Piece::T, Piece::L, Piece::J, Piece::S],
                hold: None,
                combo: 0,
                back_to_back: false,
                b2b_count: 0,
                randomizer: Randomizer::SevenBag { bag_state },
            },
            incoming: vec![],
            pieces_placed: 0,
            garbage_sent: 0,
            frames_per_piece: 30,
            pending_delay_frames: 20,
            node_budget: 5000,
        }
    }

    #[test]
    fn repeated_snapshot_analysis_has_identical_rank_order() {
        let a = analyze_with_profile(empty_request(), "review_h9_h12").unwrap();
        let b = analyze_with_profile(empty_request(), "review_h9_h12").unwrap();
        assert_eq!(a.candidates.len(), b.candidates.len());
        for (x, y) in a.candidates.iter().zip(&b.candidates) {
            assert_eq!(x.placement, y.placement);
            assert_eq!(x.mean_score.to_bits(), y.mean_score.to_bits());
            assert_eq!(x.worst_score.to_bits(), y.worst_score.to_bits());
            assert_eq!(x.scenarios, y.scenarios);
        }
    }

    #[test]
    fn exact_packet_timing_beats_legacy_fallback_when_present() {
        let p = IncomingPacket { lines: 4, active: Some(false), ready_in_frames: Some(7) };
        assert_eq!(p.timing(20).unwrap(), (4, 7));
    }
}

