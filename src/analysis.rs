//! Offline zero-gravity placement analysis. Execution speed is not scored.
use std::{collections::HashMap, sync::Arc};
use serde::{Deserialize, Serialize};
use crate::{bot::BotConfig, data::Placement, forecast::Forecast,
    tbp::{Start, Randomizer}, tetrio::garbage::GarbagePacket, try_create_bot};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationModel {
    /// Only packets currently active may rise; pending remains cancelable.
    #[default]
    Snapshot,
    /// Alternative pressure assumption, not predicted arrival timing.
    AllReady,
    /// Opt-in timing approximation, never the default review model.
    FixedPace,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub start: Start,
    pub incoming: Vec<GarbagePacket>,
    pub pieces_placed: u32,
    pub garbage_sent: u32,
    #[serde(default)]
    pub activation_model: ActivationModel,
    #[serde(default)]
    pub frames_per_piece: Option<u32>,
    #[serde(default)]
    pub pending_delay_frames: Option<u32>,
    /// Total work count, not a simulated execution-time budget.
    pub iterations: u32,
}
#[derive(Serialize)]
pub struct Candidate {
    pub placement: Placement, pub mean_score: f64, pub worst_score: f32, pub scenarios: u32,
}
#[derive(Serialize)]
pub struct Report {
    pub candidates: Vec<Candidate>, pub nodes: u64, pub scenarios: u32, pub iterations: u32,
    pub pending_garbage_in_search: bool, pub rules_parity_verified: bool,
    pub activation_model: ActivationModel,
    pub frames_per_piece: Option<u32>, pub pending_delay_frames: Option<u32>,
    pub gravity: u32, pub execution_costs: bool, pub deterministic_search: bool,
    pub assumptions: Vec<&'static str>,
}
fn review_config() -> BotConfig {
    let mut config = BotConfig::default();
    // This internal search seed is NOT a game seed and is never a request field.
    config.search_seed = Some(0x4343_3252_4556_4945);
    config.freestyle_weights.softdrop = 0.0;
    config
}
pub fn analyze(request: Request) -> Result<Report, String> {
    request.start.validate()?;
    if request.start.queue.len() != 6 { return Err("exactly current + five NEXT pieces are required".into()); }
    if !matches!(request.start.randomizer, Randomizer::Unknown) {
        return Err("hidden randomizer state is not accepted by replay analysis".into());
    }
    if !(10..=10000).contains(&request.iterations) { return Err("iterations must be 10..10000".into()); }
    let timed = request.activation_model == ActivationModel::FixedPace;
    if timed {
        if request.frames_per_piece.is_none() || request.pending_delay_frames.is_none() {
            return Err("fixed_pace requires both explicit timing assumptions".into());
        }
    } else if request.frames_per_piece.is_some() || request.pending_delay_frames.is_some() {
        return Err("clock-free analysis does not accept pace fields; omit them or explicitly select fixed_pace".into());
    }
    // Identical no-rise hole scenarios need only one DAG. Reuse the full budget.
    let scenarios = if request.incoming.iter().any(|p| p.active ||
        request.activation_model != ActivationModel::Snapshot) {10} else {1};
    let mut scores: HashMap<Placement, (f64, f32, u32)> = HashMap::new();
    let mut nodes = 0;
    let config = Arc::new(review_config());
    for scenario in 0..scenarios {
        let start = Start {board: request.start.board, queue: request.start.queue.clone(),
            hold: request.start.hold, combo: request.start.combo,
            back_to_back: request.start.back_to_back, b2b_count: request.start.b2b_count,
            randomizer: Randomizer::Unknown};
        let forecast = if timed {
            Forecast::new(&request.incoming, request.pieces_placed, request.garbage_sent,
                request.frames_per_piece.unwrap(), request.pending_delay_frames.unwrap(), scenario)?
        } else {
            Forecast::at_snapshot(&request.incoming, request.pieces_placed, request.garbage_sent,
                scenario, request.activation_model == ActivationModel::AllReady)?
        };
        let mut bot = try_create_bot(start, config.clone())?;
        bot.set_forecast(forecast);
        let iterations = request.iterations / scenarios + u32::from(scenario < request.iterations % scenarios);
        for _ in 0..iterations {nodes += bot.do_work().nodes;}
        for (placement, score) in bot.ranked_suggestions() {
            let entry = scores.entry(placement).or_insert((0.0, f32::INFINITY, 0));
            entry.0 += score as f64; entry.1 = entry.1.min(score); entry.2 += 1;
        }
    }
    let mut candidates: Vec<_> = scores.into_iter().filter(|(_,v)| v.2 == scenarios)
        .map(|(placement,(sum,worst,count))| Candidate {placement,
            mean_score: sum / count as f64, worst_score: worst, scenarios: count}).collect();
    candidates.sort_by(|a,b| b.mean_score.total_cmp(&a.mean_score).then_with(||
        placement_key(a.placement).cmp(&placement_key(b.placement))));
    let mut assumptions = vec![
        "Zero-gravity placement search; no PPS, soft-drop, or execution-speed penalty.",
        "Only observed incoming packets and own history; no opponent board or future attacks.",
        "Unknown holes use fixed clean-hole scenarios, not hidden replay RNG.",
        "Scores are heuristic, not win probabilities or certified mistake labels.",
        "Future per-scenario decisions may be optimistic about information not yet revealed.",
    ];
    assumptions.push(match request.activation_model {
        ActivationModel::Snapshot => "Activation is frozen at the observation; inactive packets remain cancelable but do not rise. This is not a real-time forecast.",
        ActivationModel::AllReady => "All currently observed packets are treated as ready; this is a separate pressure assumption, not actual arrival timing.",
        ActivationModel::FixedPace => "Garbage activation uses supplied fixed pace and delay; gravity and execution costs remain disabled.",
    });
    Ok(Report {candidates, nodes, scenarios, iterations: request.iterations,
        pending_garbage_in_search: true, rules_parity_verified: false,
        activation_model: request.activation_model, frames_per_piece: request.frames_per_piece,
        pending_delay_frames: request.pending_delay_frames, gravity: 0, execution_costs: false,
        deterministic_search: true, assumptions})
}
fn placement_key(p: Placement) -> (u8,u8,i8,i8,u8) {
    (p.location.piece as u8, p.location.rotation as u8, p.location.x, p.location.y, p.spin as u8)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    fn request(incoming: Value) -> Request {
        serde_json::from_value(json!({"start":{"board":[],"queue":["I","O","T","L","J","S"],
            "hold":"Z","combo":0,"back_to_back":false,"b2b_count":0,"randomizer":{"type":"unknown"}},
            "incoming":incoming,"pieces_placed":30,"garbage_sent":0,"iterations":30})).unwrap()
    }
    #[test]
    fn review_is_clock_free_and_has_no_execution_penalty() {
        assert_eq!(review_config().freestyle_weights.softdrop,0.0);
        let r=analyze(request(json!([]))).unwrap();
        assert_eq!(r.activation_model,ActivationModel::Snapshot);
        assert_eq!(r.gravity,0); assert!(!r.execution_costs);
        assert!(r.frames_per_piece.is_none()); assert_eq!(r.scenarios,1);
        assert!(!r.candidates.is_empty());
    }
    #[test]
    fn repeated_review_is_exactly_reproducible() {
        let make=||request(json!([{"lines":8,"active":true}]));
        let a=serde_json::to_value(analyze(make()).unwrap()).unwrap();
        for _ in 0..3 {assert_eq!(a,serde_json::to_value(analyze(make()).unwrap()).unwrap());}
    }
    #[test]
    fn pace_is_opt_in_not_silently_used_or_ignored() {
        let mut r=request(json!([]));r.frames_per_piece=Some(12);r.pending_delay_frames=Some(20);
        assert!(analyze(r).is_err());
        let mut r=request(json!([]));r.activation_model=ActivationModel::FixedPace;
        assert!(analyze(r).is_err());
    }
    #[test]
    fn inactive_snapshot_does_not_invent_an_arrival_clock() {
        let p=[GarbagePacket{lines:8,active:false}];
        let mut f=Forecast::at_snapshot(&p,30,0,2,false).unwrap();
        let mut b=crate::data::Board::default();
        for _ in 0..100 {f.resolve(&mut b,&[],0);}
        assert_eq!(b,crate::data::Board::default()); assert_eq!(f.remaining(),8);
        assert_eq!(f.elapsed_frames,0);
        let mut f=Forecast::at_snapshot(&p,30,0,2,true).unwrap();
        f.resolve(&mut b,&[],0);assert_eq!(f.remaining(),0);assert_ne!(b,crate::data::Board::default());
    }
}
