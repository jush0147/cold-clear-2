//! Snapshot-only Kiwi product API. No historical bag inference or speculative tail.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::{
    analysis,
    data::{Board, Piece, Placement, Spin, TetrioRules},
    tbp::{Randomizer, Start},
};

const RULE_FIELDS: &[&str] = &[
    "b2bcharging", "b2bcharge_at", "b2bcharge_base", "b2bchaining",
    "openerphase_pieces", "allclears", "allclear_garbage", "allclear_b2b",
    "garbagespecialbonus", "clutch",
];
fn default_budget() -> u32 { 200_000 }
fn reject(code:&str, message:impl std::fmt::Display) -> String {
    format!("{code}: {message}")
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Root {
    board: Board,
    queue: Vec<Piece>,
    hold: Option<Piece>,
    combo: u32,
    back_to_back: bool,
    b2b_count: u32,
}
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RootState {
    x: i8,
    y: f64,
    hy: f64,
    rotation: u8,
    kick: u8,
    rotated: bool,
    spin: Spin,
    total_rotations: u32,
    resets: u32,
    rotation_resets: u32,
    locking: f64,
    force_lock: bool,
    safelock: u32,
    soft_dropped: bool,
    wall: bool,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Incoming { lines: u32, ready_in_frames: u32 }
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TimingRules {
    garbage_are_frames: u32,
    garbage_are_bump_frames: u32,
    garbage_locked_until_frame: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    analysis_mode: String,
    source_mode: String,
    bag_knowledge: String,
    unknown_tail: String,
    timing_rules: TimingRules,
    existing_are_lines: u32,
    start: Root,
    root_state: RootState,
    root_legal_placements: Vec<Placement>,
    rules: TetrioRules,
    hold_locked: bool,
    incoming: Vec<Incoming>,
    pieces_placed: u32,
    garbage_sent: u32,
    frames_per_piece: u32,
    authority_frame: Option<u32>,
    authority_subframe: f64,
    garbage_multiplier: Option<f64>,
    garbage_margin_frames: Option<u32>,
    garbage_increase_per_second: Option<f64>,
    #[serde(default = "default_budget")]
    node_budget: u32,
}

#[derive(Clone, Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    Place { placement: Placement },
    Hold { mode: &'static str, same_piece: bool, requires_reanalysis: bool },
}
#[derive(Clone, Serialize, Debug)]
pub struct ActionCandidate {
    pub action: Action,
    pub mean_score: f64,
    pub worst_score: f32,
    pub scenarios: u32,
    pub search_basis: &'static str,
    pub candidate_index: usize,
}
#[derive(Serialize, Debug)]
pub struct BranchNodes { pub place: u64, pub hold: u64, pub total: u64 }
#[derive(Serialize, Debug)]
pub struct Report {
    pub schema: &'static str,
    pub action: Action,
    pub candidates: Vec<ActionCandidate>,
    pub nodes: u64,
    pub branch_nodes: BranchNodes,
    pub node_budget: u32,
    pub completion: &'static str,
    pub search_path: &'static str,
    pub analysis_mode: &'static str,
    pub source_mode: &'static str,
    pub config_profile: &'static str,
    pub bag_knowledge: &'static str,
    pub unknown_tail: &'static str,
    pub place_known_search_layers: usize,
    pub hold_known_search_layers: Option<usize>,
    pub scenarios: u32,
    pub authority_attack_clock: bool,
    pub garbage_are_frames: u32,
    pub garbage_are_bump_frames: u32,
    pub garbage_locked_until_frame: u32,
    pub exact_are_bump_timing: bool,
    pub competitive_stacking_neutral_root_counters: bool,
    pub root_geometry_filtered: bool,
    pub root_geometry_candidates: usize,
    pub candidate_truncation: &'static str,
    pub hold_information_gain_optimized: bool,
    pub requires_authority_timing_validation: bool,
    pub rules_parity_verified: bool,
    pub assumptions: Vec<&'static str>,
}

fn parse(text: &str) -> Result<Request, String> {
    if text.len() > 262_144 { return Err(reject("REQUEST_TOO_LARGE","snapshot exceeds 256 KiB")); }
    let value: Value = serde_json::from_str(text)
        .map_err(|e| reject("REQUEST_JSON_INVALID",e))?;
    let rules = value.get("rules").and_then(Value::as_object)
        .ok_or_else(||reject("RULE_CONTRACT_MISSING","explicit public rules are required"))?;
    for name in RULE_FIELDS {
        if !rules.contains_key(*name) {
            return Err(reject("RULE_FIELD_MISSING",format!("missing public rule {name}")));
        }
    }
    let r: Request = serde_json::from_value(value)
        .map_err(|e| reject("REQUEST_SCHEMA_INVALID",e))?;
    if r.schema != "kiwi-snapshot/3" || r.bag_knowledge != "unknown" || r.unknown_tail != "finite_visible" {
        return Err(reject("SNAPSHOT_POLICY_INVALID","expected kiwi-snapshot/3, unknown bag, finite_visible tail policy"));
    }
    let tl = r.analysis_mode == "tl" && r.source_mode == "tl";
    let stacking = r.analysis_mode == "competitive_stacking" && r.source_mode == "40l";
    if !tl && !stacking {
        return Err(reject("ANALYSIS_MODE_UNSUPPORTED",
            format!("unsupported analysis/source mode pair {}/{}", r.analysis_mode, r.source_mode)));
    }
    if r.existing_are_lines != 0 {
        return Err(reject("PENDING_ARE_QUEUE_UNSUPPORTED",
            format!("positive current ARE queue has {} lines", r.existing_are_lines)));
    }
    for (name,value) in [
        ("garbageare", r.timing_rules.garbage_are_frames),
        ("garbagearebump", r.timing_rules.garbage_are_bump_frames),
    ] {
        if value > 10_000 {
            return Err(reject("ARE_RULE_VALUE_OUT_OF_RANGE", format!("{name} exceeds 10000 frames")));
        }
    }
    if stacking {
        if !r.incoming.is_empty() || r.garbage_sent != 0 || r.start.combo != 0
            || r.start.back_to_back || r.start.b2b_count != 0
        {
            return Err(reject("STACKING_NEUTRAL_STATE_INVALID",
                "competitive_stacking requires no incoming/garbage-sent and neutral root combo/B2B counters"));
        }
        if r.authority_frame.is_some() || r.garbage_multiplier.is_some()
            || r.garbage_margin_frames.is_some() || r.garbage_increase_per_second.is_some()
        {
            return Err(reject("STACKING_ATTACK_CLOCK_INVALID",
                "competitive_stacking must not carry a TL attack clock"));
        }
    } else if !(r.authority_frame.is_some() && r.garbage_multiplier.is_some()
        && r.garbage_margin_frames.is_some() && r.garbage_increase_per_second.is_some())
    {
        return Err(reject("ATTACK_CLOCK_INCOMPLETE","TL snapshot requires the complete attack clock"));
    }
    if r.start.queue.len() != 6 {
        return Err(reject("VISIBLE_QUEUE_INVALID","snapshot requires current plus exactly NEXT 5"));
    }
    if !r.root_state.y.is_finite() || !r.root_state.hy.is_finite()
        || !r.root_state.locking.is_finite() || r.root_state.locking < 0.0
        || !(-4..=13).contains(&r.root_state.x)
        || !(-4.0..=40.0).contains(&r.root_state.y)
        || !(-4.0..=40.0).contains(&r.root_state.hy) || r.root_state.rotation > 3 {
        return Err(reject("ROOT_STATE_INVALID","invalid current piece geometry/timing metadata"));
    }
    // These fields are transported because x/y/rotation alone do not describe
    // SRS+ spin/kick history or lock-state semantics. Geometry uses the
    // authority-derived allowlist; exact timing remains an authority-side check.
    let _root_metadata = (
        r.root_state.kick, r.root_state.rotated, r.root_state.spin,
        r.root_state.total_rotations, r.root_state.resets,
        r.root_state.rotation_resets, r.root_state.force_lock,
        r.root_state.safelock, r.root_state.soft_dropped, r.root_state.wall,
    );
    if r.root_legal_placements.len() > 512 {
        return Err(reject("ROOT_GEOMETRY_SET_TOO_LARGE","more than 512 authority root placements"));
    }
    let current=r.start.queue[0];
    if r.root_legal_placements.iter().any(|p|p.location.piece!=current) {
        return Err(reject("ROOT_GEOMETRY_PIECE_MISMATCH","root placement allowlist must contain only current-piece placements"));
    }
    let mut dedup=r.root_legal_placements.clone();
    dedup.sort_by_key(|p|(p.location.piece as u8,p.location.x,p.location.y,p.location.rotation as u8,p.spin as u8));
    dedup.dedup();
    if dedup.len()!=r.root_legal_placements.len() {
        return Err(reject("ROOT_GEOMETRY_DUPLICATE","root placement allowlist must be unique"));
    }
    if r.hold_locked && r.start.hold.is_none() {
        return Err(reject("HOLD_LOCK_STATE_INVALID","hold_locked=true requires occupied Hold"));
    }
    if !r.authority_subframe.is_finite() || !(0.0..1.0).contains(&r.authority_subframe) {
        return Err(reject("AUTHORITY_SUBFRAME_INVALID","invalid authority subframe"));
    }
    if !(1..=600).contains(&r.frames_per_piece) {
        return Err(reject("PACE_ASSUMPTION_INVALID","frames_per_piece must be 1..600"));
    }
    if let Some(multiplier)=r.garbage_multiplier {
        if !multiplier.is_finite() || multiplier <= 0.0 || multiplier > 100.0 {
            return Err(reject("ATTACK_MULTIPLIER_INVALID","garbage multiplier must be finite and within (0,100]"));
        }
    }
    if let Some(rate)=r.garbage_increase_per_second {
        if !rate.is_finite() || rate < 0.0 || rate > 10.0 {
            return Err(reject("ATTACK_GROWTH_INVALID","garbage increase must be finite and within [0,10]"));
        }
    }
    if !(1000..=2_000_000).contains(&r.node_budget) {
        return Err(reject("NODE_BUDGET_INVALID","node_budget must be 1000..2000000"));
    }
    if !r.hold_locked && r.node_budget < 2000 {
        return Err(reject("NODE_BUDGET_TOO_SMALL_FOR_HOLD","unlocked roots need at least 2000 nodes so place and Hold branches each receive >=1000"));
    }
    r.rules.validate().map_err(|e|reject("RULE_VALUE_UNSUPPORTED",e))?;
    Ok(r)
}

fn make_start(root: Root) -> Start {
    Start {
        board: root.board,
        queue: root.queue,
        hold: root.hold,
        combo: root.combo,
        back_to_back: root.back_to_back,
        b2b_count: root.b2b_count,
        randomizer: Randomizer::Unknown,
    }
}
fn analysis_request(r:&Request, root:Root, budget:u32, hold_locked:bool) -> analysis::Request {
    analysis::Request {
        start: make_start(root),
        rules: r.rules,
        hold_locked,
        incoming: r.incoming.iter().copied().map(|p| analysis::IncomingPacket {
            lines:p.lines, active:None, ready_in_frames:Some(p.ready_in_frames),
        }).collect(),
        pieces_placed:r.pieces_placed,
        garbage_sent:r.garbage_sent,
        frames_per_piece:r.frames_per_piece,
        pending_delay_frames:0,
        authority_frame:r.authority_frame,
        garbage_multiplier:r.garbage_multiplier,
        garbage_margin_frames:r.garbage_margin_frames,
        garbage_increase_per_second:r.garbage_increase_per_second,
        node_budget:budget,
    }
}
fn post_hold_root(r:&Request) -> (Root,usize,bool,&'static str) {
    let current=r.start.queue[0];
    match r.start.hold {
        Some(held)=>{
            let mut queue=Vec::with_capacity(6);
            queue.push(held);
            queue.extend_from_slice(&r.start.queue[1..]);
            (Root{board:r.start.board,queue,hold:Some(current),combo:r.start.combo,
                back_to_back:r.start.back_to_back,b2b_count:r.start.b2b_count},
             6, held==current, "post_hold_visible_state")
        }
        None=>{
            // NEXT[5] after the Hold is genuinely unknown at this request. Do NOT
            // peek at it. Evaluate only current=N1 plus the four still-known previews.
            let queue=r.start.queue[1..].to_vec();
            (Root{board:r.start.board,queue,hold:Some(current),combo:r.start.combo,
                back_to_back:r.start.back_to_back,b2b_count:r.start.b2b_count},
             5, r.start.queue[1]==current, "post_empty_hold_known_prefix_without_revealed_next")
        }
    }
}

pub fn analyze_text(text:&str)->Result<Report,String>{
    let r=parse(text)?;
    let unlocked=!r.hold_locked;
    let hold_budget=if unlocked {r.node_budget/2} else {0};
    let place_budget=r.node_budget-hold_budget;
    if r.root_legal_placements.len() as u32 > place_budget {
        return Err(reject(
            "ROOT_GEOMETRY_BUDGET_INSUFFICIENT",
            format!("{} authority root placements exceed the {}-node Place branch budget",
                r.root_legal_placements.len(), place_budget),
        ));
    }

    let place_report=analysis::analyze_snapshot_branch(
        analysis_request(&r,r.start.clone(),place_budget,r.hold_locked),
        "review_h9_h12",
        6,
        Some(r.root_legal_placements.clone()),
    ).map_err(|e|reject("PLACE_SEARCH_REJECTED",e))?;

    let mut candidates:Vec<ActionCandidate>=place_report.candidates.into_iter().map(|c|ActionCandidate{
        action:Action::Place{placement:c.placement},
        mean_score:c.mean_score,
        worst_score:c.worst_score,
        scenarios:c.scenarios,
        search_basis:"authority_root_geometry_filtered",
        candidate_index:0,
    }).collect();

    let mut hold_nodes=0u64;
    let mut hold_layers=None;
    if unlocked {
        let (post,len,same_piece,basis)=post_hold_root(&r);
        let hold_report=analysis::analyze_snapshot_branch(
            analysis_request(&r,post,hold_budget,true),
            "review_h9_h12",
            len,
            None,
        ).map_err(|e|reject("HOLD_SEARCH_REJECTED",e))?;
        hold_nodes=hold_report.nodes;
        hold_layers=Some(len);
        let (mean,worst,scenarios)=match hold_report.candidates.first() {
            Some(c)=>(c.mean_score,c.worst_score,c.scenarios),
            None=>(-1_000_000.0,-1_000_000.0,hold_report.scenarios),
        };
        candidates.push(ActionCandidate{
            action:Action::Hold{
                mode:if r.start.hold.is_none(){"empty"}else{"occupied"},
                same_piece,
                requires_reanalysis:true,
            },
            mean_score:mean,
            worst_score:worst,
            scenarios,
            search_basis:basis,
            candidate_index:0,
        });
    }
    candidates.sort_by(|a,b|{
        b.mean_score.total_cmp(&a.mean_score)
            .then_with(||b.worst_score.total_cmp(&a.worst_score))
            .then_with(||match (&a.action,&b.action) {
                (Action::Place{..},Action::Hold{..})=>std::cmp::Ordering::Less,
                (Action::Hold{..},Action::Place{..})=>std::cmp::Ordering::Greater,
                _=>std::cmp::Ordering::Equal,
            })
    });
    for (index,candidate) in candidates.iter_mut().enumerate() {
        candidate.candidate_index=index;
    }
    let action=candidates.first()
        .ok_or_else(||reject("NO_ROOT_ACTION","no geometrically reachable placement and Hold is locked"))?
        .action.clone();
    let nodes=place_report.nodes+hold_nodes;
    if nodes>r.node_budget as u64 {
        return Err(reject("NODE_BUDGET_EXCEEDED","combined root branches exceeded request cap"));
    }
    Ok(Report{
        schema:"kiwi-snapshot-result/3",
        action,
        candidates,
        nodes,
        branch_nodes:BranchNodes{place:place_report.nodes,hold:hold_nodes,total:nodes},
        node_budget:r.node_budget,
        completion:if nodes==r.node_budget as u64{"node_budget"}else{"visible_search_idle"},
        search_path:if r.analysis_mode=="tl" {
            "stateless_clocked_snapshot_split_root_actions"
        } else {
            "stateless_competitive_stacking_snapshot_split_root_actions"
        },
        analysis_mode:if r.analysis_mode=="tl"{"tl"}else{"competitive_stacking"},
        source_mode:if r.source_mode=="tl"{"tl"}else{"40l"},
        config_profile:place_report.config_profile,
        bag_knowledge:"unknown",
        unknown_tail:"finite_visible",
        place_known_search_layers:if r.start.hold.is_none(){5}else{6},
        hold_known_search_layers:hold_layers,
        scenarios:place_report.scenarios,
        authority_attack_clock:place_report.authority_attack_clock,
        garbage_are_frames:r.timing_rules.garbage_are_frames,
        garbage_are_bump_frames:r.timing_rules.garbage_are_bump_frames,
        garbage_locked_until_frame:r.timing_rules.garbage_locked_until_frame,
        exact_are_bump_timing:false,
        competitive_stacking_neutral_root_counters:r.analysis_mode=="competitive_stacking",
        root_geometry_filtered:true,
        root_geometry_candidates:r.root_legal_placements.len(),
        candidate_truncation:"none: complete authority allowlist is scored directly; if it cannot fit the Place root budget the request rejects",
        hold_information_gain_optimized:false,
        requires_authority_timing_validation:true,
        rules_parity_verified:false,
        assumptions:vec![
            "Only the current detached visible snapshot is used; no draw history, bag remainder, hidden RNG/tail, opponent board or future original placement.",
            "Unknown tail never expands speculatively; frontier leaves keep the existing evaluator.",
            "Place search scores the complete geometry-only root landing allowlist derived by pinned Tetrp from the actual active piece; it is not intersected with spawn movegen.",
            "Geometry reachability and frame-accurate input timing are separate. Tetrp must still validate execution timing before any future automated execution.",
            "Hold is a standalone action with no landing. Tetrp applies Hold, refills NEXT 5, then sends a new hold_locked=true request.",
            "Same-piece Hold is a distinct action. It is not assumed equivalent because Hold respawns/resets the active piece and changes Hold lock state.",
            "Empty-Hold pre-reveal scoring deliberately omits the newly revealed unknown preview. Information-gain value is NOT optimized; the real revealed piece is used only by the required post-Hold request.",
            "The request node cap is split deterministically between Place and Hold branches when Hold is available; post-Hold reanalysis is a separate request with its own cap.",
            "Real garbageare/garbagearebump rule values are preserved in the request/result. Positive existing ARE is rejected; exact ARE/bump timing is not simulated.",
            "competitive_stacking is a distinct 40L-source heuristic mode with neutral root combo/B2B and no TL attack clock; it is neither TL parity nor 40L score optimization.",
            "Pending uses explicit frames_per_piece and ten hypothetical clean-hole scenarios; ARE/bump timing remains approximate.",
        ],
    })
}

pub fn capabilities()->Value{
    json!({
        "schema":"kiwi-snapshot-capabilities/3",
        "snapshot_api":"analyze_snapshot_json",
        "request_schema":"kiwi-snapshot/3",
        "result_schema":"kiwi-snapshot-result/3",
        "default_node_budget":200000,
        "hard_node_budget_per_request":true,
        "visible_next":5,
        "bag_knowledge":"unknown",
        "unknown_tail":"finite_visible",
        "history_scan":false,
        "history_derived_bag":false,
        "speculative_tail_expansion":false,
        "explicit_hold_action":true,
        "hold_action_has_landing":false,
        "post_hold_reanalysis":true,
        "root_hold_lock":true,
        "same_piece_hold_search":true,
        "same_piece_hold_action_explicit":true,
        "hold_information_gain_optimized":false,
        "empty_hold_unknown_reveal_scoring":"known_prefix_only_until_required_post_hold_reanalysis",
        "product_persistent_dag_reuse":false,
        "all_roots_use_snapshot":true,
        "supported_source_modes":["tl","40l"],
        "competitive_stacking_mode":true,
        "competitive_stacking_semantics":"40L source; neutral root combo/B2B; no pending or TL attack clock",
        "garbage_are_rule_transport":true,
        "garbage_are_bump_rule_transport":true,
        "garbage_are_effect_model":"preserved rule values; positive existing ARE rejected; exact ARE/bump timing not simulated",
        "no_pending_nonunit_multiplier":true,
        "public_rule_contract":true,
        "pending_unknown_activation":"reject",
        "pending_positive_are_queue":"reject",
        "pending_hole_scenarios":10,
        "structured_rejections":true,
        "root_geometry_in_search":true,
        "root_geometry_source":"pinned_tetrp_authority_complete_current_pose_allowlist",
        "root_candidate_top_k":null,
        "root_timing_in_search":false,
        "authority_timing_validation_required":true,
        "rules_parity_verified":false,
        "full_opening_double_cancel_parity":false,
        "full_clutch_parity":false,
        "exact_are_bump_timing":false,
        "phase_4b_implemented":false,
        "strategy_profile":"review_h9_h12"
    })
}

#[cfg(target_arch="wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn analyze_snapshot_json(text:&str)->Result<String,wasm_bindgen::JsValue>{
    analyze_text(text)
        .and_then(|r|serde_json::to_string(&r).map_err(|e|reject("RESULT_SERIALIZATION_FAILED",e)))
        .map_err(|e|wasm_bindgen::JsValue::from_str(&e))
}
#[cfg(target_arch="wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn snapshot_capabilities_json()->String{capabilities().to_string()}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::data::{PieceLocation,Rotation};
    fn p(piece:Piece,x:i8)->Placement{
        Placement{location:PieceLocation{piece,rotation:Rotation::North,x,y:0},spin:Spin::None}
    }
    fn input(held:Option<Piece>,next0:Piece,pending:bool)->Value{
        json!({
            "schema":"kiwi-snapshot/3","analysis_mode":"tl","source_mode":"tl",
            "bag_knowledge":"unknown","unknown_tail":"finite_visible",
            "timing_rules":{"garbage_are_frames":5,"garbage_are_bump_frames":12,"garbage_locked_until_frame":0},
            "existing_are_lines":0,
            "start":{"board":[],"queue":["T",next0,"O","S","Z","J"],"hold":held,
                "combo":0,"back_to_back":false,"b2b_count":0},
            "root_state":{"x":4,"y":17.96,"hy":18.0,"rotation":0,"kick":0,"rotated":false,"spin":"none","total_rotations":0,
                "resets":0,"rotation_resets":0,"locking":0.0,"force_lock":false,"safelock":0,"soft_dropped":false,"wall":false},
            "root_legal_placements":[p(Piece::T,4)],
            "rules":TetrioRules{b2b_charge_base:3,..TetrioRules::default()},
            "hold_locked":false,
            "incoming":if pending{json!([{"lines":4,"ready_in_frames":0}])}else{json!([])},
            "pieces_placed":0,"garbage_sent":0,"frames_per_piece":24,
            "authority_frame":0,"authority_subframe":0,
            "garbage_multiplier":1.5,"garbage_margin_frames":10800,
            "garbage_increase_per_second":0.008,"node_budget":5000
        })
    }
    #[test]
    fn same_piece_empty_and_occupied_hold_are_explicit_without_landing(){
        for (held,next0,mode) in [
            (None,Piece::T,"empty"),
            (Some(Piece::T),Piece::I,"occupied"),
        ]{
            let r=analyze_text(&input(held,next0,false).to_string()).unwrap();
            let h=r.candidates.iter().find(|c|matches!(&c.action,Action::Hold{..})).unwrap();
            let v=serde_json::to_value(&h.action).unwrap();
            assert_eq!(v["mode"],mode);assert_eq!(v["same_piece"],true);
            assert!(v.get("placement").is_none());
        }
        assert_eq!(capabilities()["same_piece_hold_search"],true);
        assert_eq!(capabilities()["hold_information_gain_optimized"],false);
    }
    #[test]
    fn locked_root_has_no_hold_and_geometry_filter_is_enforced(){
        let mut v=input(Some(Piece::L),Piece::I,true);
        v["hold_locked"]=json!(true);
        let r=analyze_text(&v.to_string()).unwrap();
        assert!(r.candidates.iter().all(|c|matches!(&c.action,Action::Place{..})));
        for c in &r.candidates {
            if let Action::Place{placement}=&c.action { assert_eq!(*placement,p(Piece::T,4)); }
        }
    }
    #[test]
    fn strict_contract_rejects_history_unknown_activation_and_bad_geometry(){
        let base=input(None,Piece::I,false);
        for key in ["observed_draws","bag_state","rng","hidden_next","history"]{
            let mut v=base.clone();v[key]=json!([]);assert!(analyze_text(&v.to_string()).is_err());
        }
        let mut v=input(None,Piece::I,true);
        v["incoming"][0].as_object_mut().unwrap().remove("ready_in_frames");
        assert!(analyze_text(&v.to_string()).unwrap_err().starts_with("REQUEST_SCHEMA_INVALID:"));
        let mut v=base.clone();v["root_legal_placements"][0]["location"]["type"]=json!("I");
        assert!(analyze_text(&v.to_string()).unwrap_err().starts_with("ROOT_GEOMETRY_PIECE_MISMATCH:"));
    }
    #[test]
    fn real_tl_are_rules_are_preserved_but_not_claimed_exact(){
        let r=analyze_text(&input(Some(Piece::L),Piece::I,false).to_string()).unwrap();
        assert_eq!(r.analysis_mode,"tl");
        assert_eq!(r.source_mode,"tl");
        assert_eq!(r.garbage_are_frames,5);
        assert_eq!(r.garbage_are_bump_frames,12);
        assert!(!r.exact_are_bump_timing);
        assert!(r.authority_attack_clock);
    }
    #[test]
    fn positive_existing_are_is_a_distinct_rejection(){
        let mut v=input(Some(Piece::L),Piece::I,false);
        v["existing_are_lines"]=json!(2);
        assert!(analyze_text(&v.to_string()).unwrap_err().starts_with("PENDING_ARE_QUEUE_UNSUPPORTED:"));
    }
    #[test]
    fn competitive_stacking_is_explicit_40l_with_neutral_root_and_no_attack_clock(){
        let mut v=input(Some(Piece::L),Piece::I,false);
        v["analysis_mode"]=json!("competitive_stacking");
        v["source_mode"]=json!("40l");
        v["authority_frame"]=Value::Null;
        v["garbage_multiplier"]=Value::Null;
        v["garbage_margin_frames"]=Value::Null;
        v["garbage_increase_per_second"]=Value::Null;
        let r=analyze_text(&v.to_string()).unwrap();
        assert_eq!(r.analysis_mode,"competitive_stacking");
        assert_eq!(r.source_mode,"40l");
        assert!(!r.authority_attack_clock);
        assert!(r.competitive_stacking_neutral_root_counters);
    }
    #[test]
    fn deterministic_requests_do_not_share_search_state(){
        let v=input(Some(Piece::L),Piece::I,true);
        let a=serde_json::to_value(analyze_text(&v.to_string()).unwrap()).unwrap();
        let _=analyze_text(&input(None,Piece::T,false).to_string()).unwrap();
        let b=serde_json::to_value(analyze_text(&v.to_string()).unwrap()).unwrap();
        assert_eq!(a,b);assert!(a["nodes"].as_u64().unwrap()<=5000);
    }
}
