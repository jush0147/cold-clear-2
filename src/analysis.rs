//! Snapshot-only pending-aware analysis. This is a scenario approximation,
//! not a certified real-time TETR.IO emulator or a live-game automation API.
use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    bot::{BotConfig, Statistics},
    data::{Placement, TetrioRules},
    forecast::Forecast,
    ko_support::with_search_seed,
    tbp::{Randomizer, Start},
    try_create_bot_with_rules,
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
    #[serde(default)]
    pub rules: TetrioRules,
    pub incoming: Vec<IncomingPacket>,
    pub pieces_placed: u32,
    pub garbage_sent: u32,
    /// Explicit pace assumption, not actual future replay timing.
    pub frames_per_piece: u32,
    /// Legacy fallback for callers that provide only active=false. New Tetrp
    /// authority adapters provide ready_in_frames on every packet.
    pub pending_delay_frames: u32,
    /// Optional exact authority clock state. All four fields must be present
    /// together for time-based Tetrp attack scaling to be modeled.
    #[serde(default)]
    pub authority_frame: Option<u32>,
    #[serde(default)]
    pub garbage_multiplier: Option<f64>,
    #[serde(default)]
    pub garbage_margin_frames: Option<u32>,
    #[serde(default)]
    pub garbage_increase_per_second: Option<f64>,
    /// Total evaluator-node budget across the modeled hole scenarios.
    pub node_budget: u32,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
struct TunerOverrides {
    #[serde(default)]
    pending_safety: Option<f32>,
    #[serde(default)]
    useful_attack_reward: Option<f32>,
    #[serde(default)]
    cancellation_reward: Option<f32>,
    #[serde(default)]
    h3_b2b_charge_value: Option<f32>,
    #[serde(default)]
    h3_surge_bank_value: Option<f32>,
    #[serde(default)]
    h6_base_holes_scale: Option<f32>,
    #[serde(default)]
    h6_base_coveredness_scale: Option<f32>,
    #[serde(default)]
    h9_cavity_excavation: Option<f32>,
    #[serde(default)]
    row_transitions_scale: Option<f32>,
    #[serde(default)]
    height_scale: Option<f32>,
    #[serde(default)]
    tetris_well_depth_scale: Option<f32>,
    #[serde(default)]
    combo_attack_scale: Option<f32>,
    #[serde(default)]
    freestyle_exploitation: Option<f64>,
    #[serde(default)]
    freestyle_speculated_exploitation: Option<f64>,
}

fn bounded_f32(name: &str, value: f32, min: f32, max: f32) -> Result<f32, String> {
    if !value.is_finite() || value < min || value > max {
        return Err(format!("{name} must be finite and within [{min}, {max}]"));
    }
    Ok(value)
}

fn bounded_f64(name: &str, value: f64, min: f64, max: f64) -> Result<f64, String> {
    if !value.is_finite() || value < min || value > max {
        return Err(format!("{name} must be finite and within [{min}, {max}]"));
    }
    Ok(value)
}

/// Dynamic evaluator overrides used only by the external black-box tuner.
/// The base is corrected_legacy_h12, so H12 remains a correctness invariant
/// while H13 stays off for scored snapshot comparisons.
fn tuner_config_from_profile(profile: &str) -> Result<Option<BotConfig>, String> {
    let Some(json) = profile.strip_prefix("tuner:") else { return Ok(None); };
    if json.len() > 4096 {
        return Err("tuner profile exceeds 4096 bytes".into());
    }
    let o: TunerOverrides = serde_json::from_str(json)
        .map_err(|e| format!("invalid tuner profile JSON: {e}"))?;
    if [
        o.pending_safety.is_some(),
        o.useful_attack_reward.is_some(),
        o.cancellation_reward.is_some(),
        o.h3_b2b_charge_value.is_some(),
        o.h3_surge_bank_value.is_some(),
        o.h6_base_holes_scale.is_some(),
        o.h6_base_coveredness_scale.is_some(),
        o.h9_cavity_excavation.is_some(),
        o.row_transitions_scale.is_some(),
        o.height_scale.is_some(),
        o.tetris_well_depth_scale.is_some(),
        o.combo_attack_scale.is_some(),
        o.freestyle_exploitation.is_some(),
        o.freestyle_speculated_exploitation.is_some(),
    ].into_iter().all(|v| !v) {
        return Err("tuner profile must contain at least one override".into());
    }

    let mut c = BotConfig::corrected_legacy_h12();
    if let Some(v) = o.pending_safety {
        c.freestyle_weights.pending_safety = bounded_f32("pending_safety", v, -100.0, 100.0)?;
    }
    if let Some(v) = o.useful_attack_reward {
        c.freestyle_weights.useful_attack_reward = bounded_f32("useful_attack_reward", v, -100.0, 100.0)?;
    }
    if let Some(v) = o.cancellation_reward {
        c.freestyle_weights.cancellation_reward = bounded_f32("cancellation_reward", v, -100.0, 100.0)?;
    }
    if let Some(v) = o.h3_b2b_charge_value {
        c.freestyle_weights.h3_b2b_charge_value = bounded_f32("h3_b2b_charge_value", v, -100.0, 100.0)?;
    }
    if let Some(v) = o.h3_surge_bank_value {
        c.freestyle_weights.h3_surge_bank_value = bounded_f32("h3_surge_bank_value", v, -100.0, 100.0)?;
    }
    if let Some(v) = o.h6_base_holes_scale {
        c.freestyle_weights.h6_base_holes_scale = bounded_f32("h6_base_holes_scale", v, 0.0, 20.0)?;
    }
    if let Some(v) = o.h6_base_coveredness_scale {
        c.freestyle_weights.h6_base_coveredness_scale = bounded_f32("h6_base_coveredness_scale", v, 0.0, 20.0)?;
    }
    if let Some(v) = o.h9_cavity_excavation {
        c.freestyle_weights.h9_cavity_excavation = bounded_f32("h9_cavity_excavation", v, -100.0, 100.0)?;
    }
    if let Some(v) = o.row_transitions_scale {
        c.freestyle_weights.row_transitions *= bounded_f32("row_transitions_scale", v, 0.0, 20.0)?;
    }
    if let Some(v) = o.height_scale {
        c.freestyle_weights.height *= bounded_f32("height_scale", v, 0.0, 20.0)?;
    }
    if let Some(v) = o.tetris_well_depth_scale {
        c.freestyle_weights.tetris_well_depth *= bounded_f32("tetris_well_depth_scale", v, 0.0, 20.0)?;
    }
    if let Some(v) = o.combo_attack_scale {
        c.freestyle_weights.combo_attack *= bounded_f32("combo_attack_scale", v, 0.0, 20.0)?;
    }
    if let Some(v) = o.freestyle_exploitation {
        c.freestyle_exploitation = bounded_f64("freestyle_exploitation", v, 0.01, 5.0)?;
    }
    if let Some(v) = o.freestyle_speculated_exploitation {
        c.freestyle_speculated_exploitation =
            bounded_f64("freestyle_speculated_exploitation", v, 0.01, 5.0)?;
    }
    Ok(Some(c))
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
    pub authority_attack_clock: bool,
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
    request.rules.validate()?;
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
    let clock_fields = (
        request.authority_frame,
        request.garbage_multiplier,
        request.garbage_margin_frames,
        request.garbage_increase_per_second,
    );
    let authority_attack_clock = matches!(clock_fields, (Some(_),Some(_),Some(_),Some(_)));
    if !authority_attack_clock
        && [request.authority_frame.is_some(),request.garbage_multiplier.is_some(),
            request.garbage_margin_frames.is_some(),request.garbage_increase_per_second.is_some()]
            .into_iter().any(|x|x)
    {
        return Err("authority attack clock fields must be supplied together".into());
    }

    let scenarios: u32 = if request.incoming.is_empty() { 1 } else { 10 };
    let mut scores: HashMap<Placement, (f64, f32, u32)> = HashMap::new();
    let mut nodes = 0u64;
    let tuner_config = tuner_config_from_profile(profile)?;
    let (config, profile_label): (Arc<BotConfig>, &'static str) = if let Some(c) = tuner_config {
        (Arc::new(c), "tuner-overrides+h12")
    } else {
        match profile {
        "review_h9_h12" => (Arc::new(BotConfig::review_h9_h12()), "h9+h12-review"),
        "review_minus_h1_h12" => {
            let mut c = BotConfig::review_h9_h12();
            c.freestyle_weights.pending_safety = 0.0;
            (Arc::new(c), "review-minus-h1+h12")
        }
        "review_minus_h2_h12" => {
            let mut c = BotConfig::review_h9_h12();
            c.freestyle_weights.useful_attack_reward = 0.0;
            c.freestyle_weights.cancellation_reward = 0.0;
            (Arc::new(c), "review-minus-h2+h12")
        }
        "review_minus_h6c_h12" => {
            let mut c = BotConfig::review_h9_h12();
            c.freestyle_weights.row_transitions = BotConfig::legacy().freestyle_weights.row_transitions;
            (Arc::new(c), "review-minus-h6c+h12")
        }
        "review_minus_h9_h12" => {
            let mut c = BotConfig::review_h9_h12();
            c.freestyle_weights.h9_cavity_excavation = 0.0;
            (Arc::new(c), "review-minus-h9+h12")
        }
        "corrected_legacy_h12" => {
            (Arc::new(BotConfig::corrected_legacy_h12()), "corrected-legacy+h12")
        }
        "reset_h1_pending_safety_1_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.pending_safety = 1.0;
            (Arc::new(c), "reset-h1-pending1+h12")
        }
        "reset_h2_useful_attack_1_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.useful_attack_reward = 1.0;
            c.freestyle_weights.cancellation_reward = 0.0;
            (Arc::new(c), "reset-h2-useful1+h12")
        }
        "reset_h3_b2b_charge_1_5_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h3_b2b_charge_value = 1.5;
            c.freestyle_weights.h3_surge_bank_value = 0.0;
            (Arc::new(c), "reset-h3-charge1.5+h12")
        }
        "reset_h4_well_half_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.tetris_well_depth *= 0.5;
            (Arc::new(c), "reset-h4-well0.5+h12")
        }
        "reset_h5_combo_4_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.combo_attack *= 4.0;
            (Arc::new(c), "reset-h5-combo4+h12")
        }
        "reset_h6_height_0_75_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.height *= 0.75;
            (Arc::new(c), "reset-h6-height0.75+h12")
        }
        "reset_h6b_holes1_5_covered0_5_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h6_base_holes_scale = 1.5;
            c.freestyle_weights.h6_base_coveredness_scale = 0.5;
            (Arc::new(c), "reset-h6b-h1.5-c0.5+h12")
        }
        "reset_h6c_row2_5_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.row_transitions *= 2.5;
            (Arc::new(c), "reset-h6c-row2.5+h12")
        }
        "reset_h9_cavity_m1_5_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = -1.5;
            (Arc::new(c), "reset-h9-cavity-1.5+h12")
        }
        "reset_h9_cavity_m1_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = -1.0;
            (Arc::new(c), "reset-h9-cavity-1+h12")
        }
        "reset_h9_cavity_m0_875_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = -0.875;
            (Arc::new(c), "reset-h9-cavity-0.875+h12")
        }
        "reset_h9_cavity_m0_75_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = -0.75;
            (Arc::new(c), "reset-h9-cavity-0.75+h12")
        }
        "reset_h9_cavity_m0_625_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = -0.625;
            (Arc::new(c), "reset-h9-cavity-0.625+h12")
        }
        "reset_h9_cavity_m0_5_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = -0.5;
            (Arc::new(c), "reset-h9-cavity-0.5+h12")
        }
        "reset_h9_cavity_m0_375_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = -0.375;
            (Arc::new(c), "reset-h9-cavity-0.375+h12")
        }
        "reset_h9_cavity_m0_25_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = -0.25;
            (Arc::new(c), "reset-h9-cavity-0.25+h12")
        }
        "reset_h9_cavity_m0_125_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = -0.125;
            (Arc::new(c), "reset-h9-cavity-0.125+h12")
        }
        "reset_h9_cavity_0_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = 0.0;
            (Arc::new(c), "reset-h9-cavity0+h12")
        }
        "reset_h9_cavity_p0_25_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = 0.25;
            (Arc::new(c), "reset-h9-cavity+0.25+h12")
        }
        "reset_h9_cavity_p0_5_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = 0.5;
            (Arc::new(c), "reset-h9-cavity+0.5+h12")
        }
        "reset_h9_cavity_p1_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = 1.0;
            (Arc::new(c), "reset-h9-cavity+1+h12")
        }
        "reset_h9_cavity_p1_5_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_weights.h9_cavity_excavation = 1.5;
            (Arc::new(c), "reset-h9-cavity+1.5+h12")
        }
        "reset_h10_exploitation_0_7985_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_exploitation = 0.7985076962177716;
            c.freestyle_speculated_exploitation = 0.7985076962177716;
            (Arc::new(c), "reset-h10-exploitation0.7985+h12")
        }
        "reset_h11_k40s60_h12" => {
            let mut c = BotConfig::corrected_legacy_h12();
            c.freestyle_exploitation = 0.5108256237659907;
            c.freestyle_speculated_exploitation = 0.916290731874155;
            (Arc::new(c), "reset-h11-k40s60+h12")
        }
        _ => return Err("unknown analysis profile".into()),
        }
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
        let forecast = match clock_fields {
            (Some(frame),Some(multiplier),Some(margin),Some(rate)) => Forecast::new_timed_with_clock(
                &timed_incoming,
                request.pieces_placed,
                request.garbage_sent,
                request.frames_per_piece,
                frame,
                multiplier,
                margin,
                rate,
                scenario,
            )?,
            _ => Forecast::new_timed(
                &timed_incoming,
                request.pieces_placed,
                request.garbage_sent,
                request.frames_per_piece,
                scenario,
            )?,
        };
        let mut bot = try_create_bot_with_rules(start, config.clone(), request.rules)?;
        let mut forecast = forecast;
        forecast.set_opener_phase_pieces(request.rules.opener_phase_pieces);
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
        authority_attack_clock,
        assumptions: vec![
            "Only already observable incoming packets are modeled; no opponent board or future attacks.",
            "Unknown holes use ten equally weighted clean-hole scenarios when incoming garbage exists.",
            "Per-packet ready_in_frames is used when supplied; active-only callers fall back to pending_delay_frames.",
            "When supplied together, authority frame/multiplier/margin/rate drive time-based Tetrp attack scaling at each hypothetical lock frame.",
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
            rules: TetrioRules::default(),
            incoming: vec![],
            pieces_placed: 0,
            garbage_sent: 0,
            frames_per_piece: 30,
            pending_delay_frames: 20,
            authority_frame: None,
            garbage_multiplier: None,
            garbage_margin_frames: None,
            garbage_increase_per_second: None,
            node_budget: 5000,
        }
    }

    #[test]
    fn reset_profiles_are_accepted() {
        for profile in [
            "corrected_legacy_h12",
            "reset_h1_pending_safety_1_h12",
            "reset_h2_useful_attack_1_h12",
            "reset_h3_b2b_charge_1_5_h12",
            "reset_h4_well_half_h12",
            "reset_h5_combo_4_h12",
            "reset_h6_height_0_75_h12",
            "reset_h6b_holes1_5_covered0_5_h12",
            "reset_h6c_row2_5_h12",
            "reset_h9_cavity_m1_5_h12",
            "reset_h9_cavity_m1_h12",
            "reset_h9_cavity_m0_875_h12",
            "reset_h9_cavity_m0_75_h12",
            "reset_h9_cavity_m0_625_h12",
            "reset_h9_cavity_m0_5_h12",
            "reset_h9_cavity_m0_375_h12",
            "reset_h9_cavity_m0_25_h12",
            "reset_h9_cavity_m0_125_h12",
            "reset_h9_cavity_0_h12",
            "reset_h9_cavity_p0_25_h12",
            "reset_h9_cavity_p0_5_h12",
            "reset_h9_cavity_p1_h12",
            "reset_h9_cavity_p1_5_h12",
            "reset_h10_exploitation_0_7985_h12",
            "reset_h11_k40s60_h12",
        ] {
            let report = analyze_with_profile(empty_request(), profile).unwrap();
            assert!(!report.candidates.is_empty(), "profile {profile}");
            assert!(report.nodes <= report.node_budget as u64, "profile {profile}");
        }
    }

    #[test]
    fn tuner_profile_applies_dynamic_h9_override_on_corrected_baseline() {
        let c = tuner_config_from_profile(
            r#"tuner:{"h9_cavity_excavation":-0.375,"row_transitions_scale":1.25}"#
        ).unwrap().unwrap();
        let legacy = BotConfig::legacy();
        assert_eq!(c.freestyle_weights.h9_cavity_excavation, -0.375);
        assert_eq!(
            c.freestyle_weights.row_transitions,
            legacy.freestyle_weights.row_transitions * 1.25
        );
        assert!(c.dag_backprop_best_demotion);
        assert!(!c.dag_backprop_despeculated_values);
    }

    #[test]
    fn tuner_profile_rejects_unknown_or_empty_overrides() {
        assert!(tuner_config_from_profile(r#"tuner:{"made_up":1}"#).is_err());
        assert!(tuner_config_from_profile("tuner:{}").is_err());
    }

    #[test]
    fn tuner_profile_runs_through_snapshot_analysis() {
        let report = analyze_with_profile(
            empty_request(),
            r#"tuner:{"h9_cavity_excavation":-0.375}"#,
        ).unwrap();
        assert_eq!(report.config_profile, "tuner-overrides+h12");
        assert!(!report.candidates.is_empty());
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

