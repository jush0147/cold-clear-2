//! Snapshot-only Kiwi product API, independent of historical SevenBag state.
//! Compatibility APIs in analysis/wasm remain available but are not v2 entrypoints.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::{analysis, data::{Board, Piece, Placement, TetrioRules}, tbp::{Start, Randomizer}};

const RULE_FIELDS: &[&str] = &[
    "b2bcharging", "b2bcharge_at", "b2bcharge_base", "b2bchaining",
    "openerphase_pieces", "allclears", "allclear_garbage", "allclear_b2b",
    "garbagespecialbonus", "clutch",
];
fn default_budget() -> u32 { 200_000 }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Root {
    board: Board,
    queue: Vec<Piece>,
    hold: Option<Piece>,
    combo: u32,
    back_to_back: bool,
    b2b_count: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pose { x: i8, y: f64, rotation: u8 }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Incoming { lines: u32, ready_in_frames: u32 }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    bag_knowledge: String,
    unknown_tail: String,
    start: Root,
    root_pose: Pose,
    rules: TetrioRules,
    hold_locked: bool,
    incoming: Vec<Incoming>,
    pieces_placed: u32,
    garbage_sent: u32,
    frames_per_piece: u32,
    authority_frame: u32,
    authority_subframe: f64,
    garbage_multiplier: f64,
    garbage_margin_frames: u32,
    garbage_increase_per_second: f64,
    #[serde(default = "default_budget")]
    node_budget: u32,
}

#[derive(Clone, Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    Place { placement: Placement },
    Hold { mode: &'static str, requires_reanalysis: bool },
}
#[derive(Serialize)]
pub struct ActionCandidate {
    pub action: Action,
    pub mean_score: f64,
    pub worst_score: f32,
    pub scenarios: u32,
}
#[derive(Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub action: Action,
    pub candidates: Vec<ActionCandidate>,
    pub nodes: u64,
    pub node_budget: u32,
    pub completion: &'static str,
    pub search_path: &'static str,
    pub config_profile: &'static str,
    pub bag_knowledge: &'static str,
    pub unknown_tail: &'static str,
    pub known_search_layers: usize,
    pub scenarios: u32,
    pub authority_attack_clock: bool,
    pub requires_authority_validation: bool,
    pub rules_parity_verified: bool,
    pub assumptions: Vec<&'static str>,
}

fn parse(text: &str) -> Result<Request, String> {
    if text.len() > 262_144 { return Err("snapshot exceeds 256 KiB".into()); }
    let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let rules = value.get("rules").and_then(Value::as_object)
        .ok_or("explicit public rules are required")?;
    for name in RULE_FIELDS {
        if !rules.contains_key(*name) { return Err(format!("missing public rule {name}")); }
    }
    let r: Request = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if r.schema != "kiwi-snapshot/2" || r.bag_knowledge != "unknown" || r.unknown_tail != "finite_visible" {
        return Err("expected kiwi-snapshot/2, unknown bag, finite_visible tail policy".into());
    }
    if r.start.queue.len() != 6 { return Err("snapshot requires current plus exactly NEXT 5".into()); }
    if !r.root_pose.y.is_finite() || !(-4..=13).contains(&r.root_pose.x)
        || !(-4.0..=40.0).contains(&r.root_pose.y) || r.root_pose.rotation > 3 {
        return Err("invalid current piece pose".into());
    }
    if !r.authority_subframe.is_finite() || !(0.0..1.0).contains(&r.authority_subframe) {
        return Err("invalid authority subframe".into());
    }
    if !(1..=600).contains(&r.frames_per_piece) { return Err("invalid hypothetical pace".into()); }
    if !r.garbage_multiplier.is_finite() || !(0.0..=100.0).contains(&r.garbage_multiplier)
        || r.garbage_multiplier == 0.0 { return Err("invalid attack multiplier".into()); }
    r.rules.validate()?;
    Ok(r)
}

fn legacy_request(r: Request) -> analysis::Request {
    analysis::Request {
        start: Start {
            board: r.start.board, queue: r.start.queue, hold: r.start.hold,
            combo: r.start.combo, back_to_back: r.start.back_to_back,
            b2b_count: r.start.b2b_count,
            // Unknown disables DAG speculation. The internal unused bag bitset
            // is not a known fresh-bag claim and may never enable speculation.
            randomizer: Randomizer::Unknown,
        },
        rules: r.rules, hold_locked: r.hold_locked,
        incoming: r.incoming.into_iter().map(|p| analysis::IncomingPacket {
            lines: p.lines, active: None, ready_in_frames: Some(p.ready_in_frames),
        }).collect(),
        pieces_placed: r.pieces_placed, garbage_sent: r.garbage_sent,
        frames_per_piece: r.frames_per_piece, pending_delay_frames: 0,
        authority_frame: Some(r.authority_frame), garbage_multiplier: Some(r.garbage_multiplier),
        garbage_margin_frames: Some(r.garbage_margin_frames),
        garbage_increase_per_second: Some(r.garbage_increase_per_second),
        node_budget: r.node_budget,
    }
}

fn root_action(placement: Placement, current: Piece, hold: Option<Piece>, next: Piece, locked: bool) -> Result<Action, String> {
    if placement.location.piece == current {
        // Same-piece Hold is deliberately NOT a separately searched action.
        // Do not expose an ambiguous useHold boolean inferred by a caller.
        return Ok(Action::Place { placement });
    }
    if locked { return Err("search returned a Hold action from a locked root".into()); }
    if placement.location.piece != hold.unwrap_or(next) { return Err("unexpected root piece".into()); }
    Ok(Action::Hold { mode: if hold.is_none() { "empty" } else { "occupied" }, requires_reanalysis: true })
}

pub fn analyze_text(text: &str) -> Result<Report, String> {
    let r = parse(text)?;
    let current = r.start.queue[0];
    let next = r.start.queue[1];
    let hold = r.start.hold;
    let locked = r.hold_locked;
    let known_search_layers = if hold.is_none() { 5 } else { 6 };
    // Both pending and no-pending roots ALWAYS use this stateless clock-aware
    // path. No retained DAG, search-call index, history scan or hidden bag state.
    let report = analysis::analyze(legacy_request(r))?;
    let mut candidates = Vec::new();
    let mut have_hold = false;
    for c in report.candidates {
        let action = root_action(c.placement, current, hold, next, locked)?;
        if matches!(action, Action::Hold { .. }) {
            if have_hold { continue; }
            have_hold = true;
        }
        candidates.push(ActionCandidate { action, mean_score: c.mean_score, worst_score: c.worst_score, scenarios: c.scenarios });
    }
    let action = candidates.first().ok_or("no suggestion at the visible search horizon")?.action.clone();
    Ok(Report {
        schema: "kiwi-snapshot-result/2", action, candidates,
        nodes: report.nodes, node_budget: report.node_budget,
        completion: if report.nodes == report.node_budget as u64 { "node_budget" } else { "visible_search_idle" },
        search_path: "stateless_clocked_snapshot", config_profile: report.config_profile,
        bag_knowledge: "unknown", unknown_tail: "finite_visible", known_search_layers,
        scenarios: report.scenarios, authority_attack_clock: report.authority_attack_clock,
        requires_authority_validation: true, rules_parity_verified: false,
        assumptions: vec![
            "Only the current visible snapshot is used; no draw history or inferred bag remainder.",
            "No speculative expansion past known queue layers; leaves retain the existing evaluator.",
            "Empty-Hold normalization conservatively searches five lock layers; occupied Hold six. This is NOT a continuation length limit.",
            "Hold is an action without an executable landing. Apply only Hold, refill NEXT 5, then submit a new locked snapshot.",
            "A separate same-piece Hold branch is excluded from this search version.",
            "All requests rebuild the DAG. Legacy WasmBot is not a snapshot-v2 product entrypoint.",
            "Root pose is transported for authority reachability validation; the core move generator remains spawn-based.",
            "Hypothetical lock timing uses the explicit frames_per_piece, integer clock and simplified ARE/bump model.",
            "Pending uses ten hypothetical clean-hole scenarios; scores are heuristics, not win probabilities.",
        ],
    })
}

pub fn capabilities() -> Value {
    json!({
        "schema":"kiwi-snapshot-capabilities/2", "snapshot_api":"analyze_snapshot_json",
        "default_node_budget":200000, "hard_node_budget":true, "visible_next":5,
        "bag_knowledge":"unknown", "unknown_tail":"finite_visible", "history_scan":false,
        "history_derived_bag":false, "speculative_tail_expansion":false,
        "explicit_hold_action":true, "hold_action_has_landing":false, "post_hold_reanalysis":true,
        "root_hold_lock":true, "same_piece_hold_search":false,
        "product_persistent_dag_reuse":false, "all_roots_use_snapshot":true,
        "no_pending_nonunit_multiplier":true, "public_rule_contract":true,
        "pending_unknown_activation":"reject", "pending_hole_scenarios":10,
        "root_geometry_in_search":false, "authority_geometry_validation_required":true,
        "rules_parity_verified":false, "full_opening_double_cancel_parity":false,
        "full_clutch_parity":false, "exact_are_bump_timing":false,
        "phase_4b_implemented":false, "strategy_profile":"review_h9_h12"
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn analyze_snapshot_json(text: &str) -> Result<String, wasm_bindgen::JsValue> {
    analyze_text(text).and_then(|r| serde_json::to_string(&r).map_err(|e|e.to_string()))
        .map_err(|e|wasm_bindgen::JsValue::from_str(&e))
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn snapshot_capabilities_json() -> String { capabilities().to_string() }

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::{bot::{BotConfig, Statistics}, data::{PieceLocation, Rotation, Spin}, ko_support::with_search_seed};
    fn input(held: bool, pending: bool) -> Value {
        json!({
            "schema":"kiwi-snapshot/2", "bag_knowledge":"unknown", "unknown_tail":"finite_visible",
            "start":{"board":[],"queue":["T","I","O","S","Z","J"],
                "hold":if held {Some("L")} else {None},"combo":0,"back_to_back":false,"b2b_count":0},
            "root_pose":{"x":4,"y":17.96,"rotation":0},
            "rules":TetrioRules{b2b_charge_base:3,..TetrioRules::default()},"hold_locked":false,
            "incoming":if pending {json!([{"lines":4,"ready_in_frames":0}])} else {json!([])},
            "pieces_placed":0,"garbage_sent":0,"frames_per_piece":24,"authority_frame":0,"authority_subframe":0,
            "garbage_multiplier":1.5,"garbage_margin_frames":10800,"garbage_increase_per_second":0.008,
            "node_budget":5000
        })
    }
    #[test]
    fn strict_unknown_contract_rejects_history_bags_missing_rules_and_unknown_arrival() {
        for key in ["observed_draws","bag_state","rng","hidden_next","history"] {
            let mut v=input(false,false);v[key]=json!([]);assert!(analyze_text(&v.to_string()).is_err());
        }
        let mut v=input(false,false);v["start"]["randomizer"]=json!({"type":"seven_bag","bag_state":["I","O","T","L","J","S","Z"]});
        assert!(analyze_text(&v.to_string()).is_err());
        let mut v=input(false,false);v["rules"].as_object_mut().unwrap().remove("b2bcharge_base");
        assert!(analyze_text(&v.to_string()).is_err());
        let mut v=input(false,true);v["incoming"][0].as_object_mut().unwrap().remove("ready_in_frames");
        assert!(analyze_text(&v.to_string()).is_err());
    }
    #[test]
    fn deterministic_pending_and_nonpending_share_strict_clock_and_budget_contract() {
        for pending in [false,true] {
            let v=input(true,pending);
            let a=serde_json::to_value(analyze_text(&v.to_string()).unwrap()).unwrap();
            let _=analyze_text(&input(false,!pending).to_string()).unwrap();
            let b=serde_json::to_value(analyze_text(&v.to_string()).unwrap()).unwrap();
            assert_eq!(a,b);assert!(a["nodes"].as_u64().unwrap()<=5000);
            assert_eq!(a["authority_attack_clock"],true);
            assert_eq!(a["scenarios"],if pending {10}else{1});
        }
    }
    #[test]
    fn unknown_dag_never_expands_a_speculative_layer() {
        for held in [false,true] { for pending in [false,true] {
            let req=legacy_request(parse(&input(held,pending).to_string()).unwrap());
            let mut bot=crate::try_create_bot_with_context(req.start,Arc::new(BotConfig::review_h9_h12()),req.rules,false).unwrap();
            let packets:Vec<_>=req.incoming.iter().map(|p|(p.lines,p.ready_in_frames.unwrap())).collect();
            bot.set_forecast(crate::forecast::Forecast::new_timed_with_clock(&packets,0,0,24,0,1.5,10800,0.008,0).unwrap());
            let mut stats=Statistics::default();
            with_search_seed(42,||{for _ in 0..3000 {
                if stats.nodes>=30000 {break;}
                stats.accumulate(bot.do_work_limited(30000-stats.nodes));
            }});
            assert_eq!(stats.speculative_expansions,0);
            assert!(stats.max_depth<=if held {6}else{5});assert!(stats.nodes<=30000);
        }}
    }
    #[test]
    fn hold_action_discards_prehold_landing_and_locked_roots_only_place_current() {
        let p=Placement{location:PieceLocation{piece:Piece::I,rotation:Rotation::North,x:4,y:0},spin:Spin::None};
        let a=root_action(p,Piece::T,None,Piece::I,false).unwrap();
        let v=serde_json::to_value(&a).unwrap();assert_eq!(v["kind"],"hold");assert!(v.get("placement").is_none());
        assert!(root_action(p,Piece::T,Some(Piece::I),Piece::L,true).is_err());
        for pending in [false,true] {
            let mut v=input(true,pending);v["hold_locked"]=json!(true);
            let report=analyze_text(&v.to_string()).unwrap();
            assert!(report.candidates.iter().all(|c| matches!(c.action,Action::Place{placement} if placement.location.piece==Piece::T)));
        }
    }
    #[test]
    fn same_piece_hold_is_explicitly_excluded_not_ambiguous() {
        let p=Placement{location:PieceLocation{piece:Piece::T,rotation:Rotation::North,x:4,y:0},spin:Spin::None};
        assert!(matches!(root_action(p,Piece::T,Some(Piece::T),Piece::I,false).unwrap(),Action::Place{..}));
        assert_eq!(capabilities()["same_piece_hold_search"],false);
    }
}
