//! Deterministic finite-horizon review, not a live versus agent.
//! Every legal root is searched at the same lock depth and beam widths.
//! Scenario policies cannot condition an action on unrevealed garbage holes.
use std::{cmp::Ordering, collections::{HashMap, HashSet}, sync::Arc};
use enumset::EnumSet;
use serde::{Deserialize, Serialize};
use crate::{bot::{BotConfig, review_score}, data::{Board, GameState, Piece, Placement},
    forecast::{Forecast, Resolution}, movegen::find_moves_with_clutch,
    tbp::{Start, Randomizer}, tetrio::{self, AttackBreakdown, garbage::GarbagePacket}};

const LOSS: f32 = -1_000_000.0;
fn default_depth() -> usize { 4 }
fn default_width() -> usize { 8 }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Action { pub placement: Placement, pub use_hold: bool }
fn action_key(a: Action) -> (bool, u8, u8, i8, i8, u8) {
    let p = a.placement;
    (a.use_hold, p.location.piece as u8, p.location.rotation as u8,
        p.location.x, p.location.y, p.spin as u8)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub start: Start,
    pub incoming: Vec<GarbagePacket>,
    pub pieces_placed: u32,
    pub garbage_sent: u32,
    /// Optional actual player action. Always evaluated, never just shortlisted.
    #[serde(default)] pub actual: Option<Action>,
    /// At most five locks: an empty-hold branch may consume two known pieces.
    #[serde(default = "default_depth")] pub depth: usize,
    #[serde(default = "default_width")] pub beam_width: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Position { state: GameState, hold: Option<Piece>, index: usize }
impl Position {
    fn observed_key(&self) -> Self {
        let mut key = self.clone();
        key.state.forecast = key.state.forecast.observed_key();
        key
    }
}
#[derive(Clone)]
struct Path { pos: Position, reward: f32, leaf: f32, dead: bool, actions: Vec<Action> }
impl Path { fn value(&self) -> f32 { if self.dead { LOSS } else {self.reward + self.leaf} } }
#[derive(Clone)]
struct Policy { paths: Vec<Path> }
impl Policy {
    fn alive(&self) -> usize { self.paths.iter().filter(|p| !p.dead).count() }
    fn mean(&self) -> f64 { self.paths.iter().map(|p| p.value() as f64).sum::<f64>() / self.paths.len() as f64 }
    fn worst(&self) -> f32 { self.paths.iter().map(|p| p.value()).fold(f32::INFINITY, f32::min) }
}
fn compare(a: &Policy, b: &Policy) -> Ordering {
    b.alive().cmp(&a.alive()).then_with(|| b.mean().total_cmp(&a.mean()))
        .then_with(|| b.worst().total_cmp(&a.worst()))
}
fn retain_best(mut policies: Vec<Policy>, width: usize) -> Vec<Policy> {
    // Stable sorting preserves deterministic action order when scores tie.
    policies.sort_by(compare);
    let mut seen = HashSet::new();
    policies.retain(|p| seen.insert(p.paths.iter().map(|s|
        (s.pos.clone(), s.dead, s.reward.to_bits(), s.leaf.to_bits())).collect::<Vec<_>>()));
    policies.truncate(width);
    policies
}
#[derive(Default)]
struct MoveCache { moves: HashMap<(Board, Piece, bool), Arc<Vec<Placement>>>, transitions: u64 }
impl MoveCache {
    fn piece_moves(&mut self, pos: &Position, piece: Piece) -> Arc<Vec<Placement>> {
        let key = (pos.state.board, piece, pos.state.combo > 0);
        if let Some(v) = self.moves.get(&key) { return Arc::clone(v); }
        let mut moves: Vec<_> = find_moves_with_clutch(&key.0, piece, key.2).into_iter().map(|m| m.0).collect();
        moves.sort_by_key(|&p| action_key(Action{placement:p,use_hold:false}));
        moves.dedup();
        let moves = Arc::new(moves);
        // A cache is a speed aid, not unbounded retained search memory.
        if self.moves.len() < 8192 { self.moves.insert(key, Arc::clone(&moves)); }
        moves
    }
    fn actions(&mut self, pos: &Position, queue: &[Piece]) -> Vec<Action> {
        let Some(&current) = queue.get(pos.index) else { return vec![]; };
        let mut actions: Vec<_> = self.piece_moves(pos, current).iter().map(|&placement|
            Action{placement,use_hold:false}).collect();
        if let Some(piece) = pos.hold.or_else(|| queue.get(pos.index+1).copied()) {
            actions.extend(self.piece_moves(pos, piece).iter().map(|&placement| Action{placement,use_hold:true}));
        }
        actions.sort_by_key(|&a| action_key(a));
        actions
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct BoardMetrics { pub height: u32, pub holes: u32, pub occupied: u32 }
fn metrics(board: &Board) -> BoardMetrics {
    let mut height = 0; let mut holes = 0; let mut occupied = 0;
    for &col in &board.cols {
        let h = 64 - col.leading_zeros();
        height = height.max(h); occupied += col.count_ones();
        holes += ((!col) & ((1u64 << h) - 1)).count_ones();
    }
    BoardMetrics{height,holes,occupied}
}
#[derive(Serialize)]
pub struct Step {
    pub action: Action, pub rows: Vec<String>, pub metrics: BoardMetrics,
    pub hold: Option<Piece>, pub known_queue: Vec<Piece>,
    pub lines: u32, pub garbage_cleared: u32, pub perfect_clear: bool,
    pub back_to_back: bool, pub b2b_count: u16, pub consecutive_clears: u8,
    pub attack: AttackBreakdown, pub packets: Vec<u32>,
    pub garbage: Resolution, pub incoming_remaining: u32,
}
struct Transition { path: Path, lines: u32, garbage_cleared: u32, pc: bool, attack: AttackBreakdown, resolution: Resolution }
fn advance(path: &Path, action: Action, queue: &[Piece], config: &BotConfig) -> Transition {
    let mut next = path.clone();
    let current = queue[next.pos.index];
    let empty_hold = next.pos.hold.is_none();
    let expected = if action.use_hold {next.pos.hold.unwrap_or_else(||queue[next.pos.index+1])} else {current};
    debug_assert_eq!(expected, action.placement.location.piece);
    if action.use_hold { next.pos.hold = Some(current); }
    let mut forecast = next.pos.state.forecast;
    next.pos.state.forecast = Forecast::default();
    let info = next.pos.state.advance(current, action.placement);
    let attack = tetrio::attack(&info);
    let resolution = forecast.resolve(&mut next.pos.state.board, &attack.packets(), info.lines_cleared);
    next.pos.state.forecast = forecast;
    next.pos.index += 1 + usize::from(action.use_hold && empty_hold);
    // No invented bag boundary, and no fake pre-hold in the review state.
    next.pos.state.bag = EnumSet::all();
    next.pos.state.reserve = next.pos.hold.unwrap_or(Piece::O);
    let visible_ts = queue[next.pos.index..].iter().filter(|&&p| p == Piece::T).count()
        + usize::from(next.pos.hold == Some(Piece::T));
    let (leaf, reward) = review_score(config, next.pos.state, &info, visible_ts);
    next.leaf = leaf; next.reward += reward;
    next.dead = forecast.topped_out;
    next.actions.push(action);
    Transition {path:next,lines:info.lines_cleared,garbage_cleared:info.garbage_cleared,
        pc:info.perfect_clear,attack,resolution}
}
fn step_details(t: Transition, queue: &[Piece]) -> Step {
    let p = t.path.pos; let board = p.state.board;
    let height = metrics(&board).height;
    let rows = (0..height).map(|y| (0..10).map(|x|
        if board.cols[x] & (1u64 << y) != 0 {'X'} else {'.'}).collect()).collect();
    Step { action:*t.path.actions.last().unwrap(),rows,metrics:metrics(&board),hold:p.hold,
        known_queue:queue[p.index..].to_vec(),lines:t.lines,garbage_cleared:t.garbage_cleared,
        perfect_clear:t.pc,back_to_back:p.state.back_to_back,b2b_count:p.state.b2b_count,
        consecutive_clears:p.state.combo,attack:t.attack,packets:t.attack.packets(),
        garbage:t.resolution,incoming_remaining:p.state.forecast.remaining() }
}
fn groups(policy: &Policy) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = vec![];
    for (i,path) in policy.paths.iter().enumerate() {
        if path.dead {continue;}
        if let Some(group) = groups.iter_mut().find(|g|
            policy.paths[g[0]].pos.observed_key() == path.pos.observed_key()) { group.push(i); }
        else { groups.push(vec![i]); }
    }
    groups
}
fn extend(policy: &Policy, width: usize, queue: &[Piece], config: &BotConfig, cache: &mut MoveCache) -> Vec<Policy> {
    let mut variants = vec![policy.clone()];
    for group in groups(policy) {
        let actions = cache.actions(&policy.paths[group[0]].pos, queue);
        if actions.is_empty() {
            for v in &mut variants {for &i in &group { v.paths[i].dead = true; }}
            continue;
        }
        // Evaluate a common action jointly over an information set. No scenario
        // may pick a different action merely because its hidden hole differs.
        let mut options = Vec::new();
        for action in actions {
            let mut candidate = policy.clone();
            for &i in &group {
                candidate.paths[i] = advance(&policy.paths[i], action, queue, config).path;
                cache.transitions += 1;
            }
            options.push(candidate);
        }
        let options = retain_best(options, width);
        let mut expanded = Vec::new();
        for v in &variants {
            for option in &options {
                let mut combined = v.clone();
                for &i in &group {combined.paths[i] = option.paths[i].clone();}
                expanded.push(combined);
            }
        }
        variants = retain_best(expanded,width);
    }
    variants
}
fn search(root: &Policy, action: Action, depth: usize, width: usize,
    queue: &[Piece], config: &BotConfig, cache: &mut MoveCache) -> Policy {
    let mut policy = root.clone();
    for path in &mut policy.paths {
        *path = advance(path,action,queue,config).path;
        cache.transitions += 1;
    }
    let mut beam = vec![policy];
    for _ in 1..depth {
        let mut expanded = Vec::new();
        for p in &beam { expanded.extend(extend(p,width,queue,config,cache)); }
        beam = retain_best(expanded,width);
    }
    beam.remove(0)
}
struct Completed { id: usize, action: Action, coarse: Policy, full: Policy, nodes: u64 }
#[derive(Serialize)]
pub struct Candidate {
    pub action_id: usize, pub action: Action, pub is_actual: bool,
    pub score: f64, pub worst_scenario_score: f32, pub coarse_score: f64,
    pub surviving_scenarios: usize, pub depth: usize, pub beam_width: usize,
    pub evaluated_transitions: u64,
}
#[derive(Serialize)]
pub struct Comparison {
    pub actual_action_id: usize, pub actual_rank: usize, pub preferred_action_id: usize,
    pub score_gap: f64, pub coarse_score_gap: f64,
    /// Agreement at two beam widths is sensitivity evidence, not confidence.
    pub same_preferred_at_two_widths: bool, pub preference_direction_agrees: bool,
    pub assessment: &'static str,
}
#[derive(Serialize)]
pub struct Report {
    pub status: &'static str, pub processed: usize, pub total_actions: usize,
    pub depth: usize, pub beam_width: usize, pub coarse_width: usize,
    pub scenarios: usize, pub gravity: u32, pub execution_costs: bool,
    pub information_set_consistent: bool, pub rules_parity_verified: bool,
    pub candidates: Vec<Candidate>, pub comparison: Option<Comparison>,
    pub assumptions: Vec<&'static str>,
}
#[derive(Serialize)]
pub struct ScenarioLine { pub scenario: usize, pub continuation_found: bool, pub steps: Vec<Step> }
#[derive(Serialize)]
pub struct Details { pub candidate: Candidate, pub lines: Vec<ScenarioLine> }

pub struct Session {
    request: Request, config: BotConfig, root: Policy, actions: Vec<Action>,
    order: Vec<usize>, completed: Vec<Completed>, coarse_width: usize,
}
impl Session {
    pub fn new(request: Request) -> Result<Self,String> {
        request.start.validate()?;
        if request.start.queue.len() != 6 {return Err("review requires current + exactly five visible NEXT pieces".into());}
        if !matches!(request.start.randomizer,Randomizer::Unknown) {return Err("hidden randomizer state is not a review observation".into());}
        if !(1..=5).contains(&request.depth) {return Err("depth must be 1..5 known-piece locks".into());}
        if !(1..=32).contains(&request.beam_width) {return Err("beam_width must be 1..32".into());}
        let scenarios = if request.incoming.iter().any(|p|p.active) {10} else {1};
        let mut paths = Vec::new();
        for scenario in 0..scenarios {
            let forecast = Forecast::at_snapshot(&request.incoming,request.pieces_placed,request.garbage_sent,scenario,false)?;
            let state = GameState {board:request.start.board,bag:EnumSet::all(),reserve:request.start.hold.unwrap_or(Piece::O),
                combo:request.start.combo as u8,back_to_back:request.start.back_to_back,b2b_count:request.start.b2b_count as u16,forecast};
            paths.push(Path {pos:Position{state,hold:request.start.hold,index:0},reward:0.0,leaf:0.0,dead:false,actions:vec![]});
        }
        let root = Policy{paths};
        let mut cache = MoveCache::default();
        let actions = cache.actions(&root.paths[0].pos,&request.start.queue);
        if actions.len()>1024 {return Err("root action count exceeds bounded review capacity".into());}
        if request.actual.map(|a|!actions.contains(&a)).unwrap_or(false) {
            return Err("actual placement/hold decision is not legal under this move generator".into());
        }
        // Actual first makes the incremental UI useful; every action still gets
        // exactly the same search parameters. Never prune the actual action.
        let mut order: Vec<_> = (0..actions.len()).collect();
        if let Some(actual) = request.actual {order.sort_by_key(|&id| actions[id] != actual);}
        let mut config = BotConfig::default(); config.freestyle_weights.softdrop = 0.0;
        let coarse_width = (request.beam_width/4).max(1);
        Ok(Self{request,config,root,actions,order,completed:vec![],coarse_width})
    }
    pub fn step(&mut self) -> bool {
        if self.completed.len() == self.actions.len() {return true;}
        let id = self.order[self.completed.len()]; let action = self.actions[id];
        let mut cache = MoveCache::default();
        let coarse = search(&self.root,action,self.request.depth,self.coarse_width,&self.request.start.queue,&self.config,&mut cache);
        let full = if self.coarse_width == self.request.beam_width {coarse.clone()} else {
            search(&self.root,action,self.request.depth,self.request.beam_width,&self.request.start.queue,&self.config,&mut cache)
        };
        self.completed.push(Completed{id,action,coarse,full,nodes:cache.transitions});
        self.completed.len() == self.actions.len()
    }
    fn candidate(&self, c: &Completed) -> Candidate {
        Candidate{action_id:c.id,action:c.action,is_actual:self.request.actual == Some(c.action),
            score:c.full.mean(),worst_scenario_score:c.full.worst(),coarse_score:c.coarse.mean(),
            surviving_scenarios:c.full.alive(),depth:self.request.depth,beam_width:self.request.beam_width,evaluated_transitions:c.nodes}
    }
    pub fn report(&self) -> Report {
        let done = self.completed.len() == self.actions.len();
        let mut ranked: Vec<_> = self.completed.iter().collect();
        ranked.sort_by(|a,b| compare(&a.full,&b.full).then_with(||a.id.cmp(&b.id)));
        let mut coarse: Vec<_> = self.completed.iter().collect();
        coarse.sort_by(|a,b| compare(&a.coarse,&b.coarse).then_with(||a.id.cmp(&b.id)));
        let comparison = if done {
            ranked.iter().position(|c| self.request.actual == Some(c.action)).map(|rank| {
                let actual = ranked[rank]; let preferred = ranked[0];
                let gap = preferred.full.mean() - actual.full.mean();
                let coarse_gap = preferred.coarse.mean() - actual.coarse.mean();
                let same = preferred.id == coarse[0].id;
                let direction = gap.signum() == coarse_gap.signum() || (gap.abs()<1e-6 && coarse_gap.abs()<1e-6);
                Comparison {actual_action_id:actual.id,actual_rank:rank+1,preferred_action_id:preferred.id,
                    score_gap:gap,coarse_score_gap:coarse_gap,same_preferred_at_two_widths:same,
                    preference_direction_agrees:direction,
                    assessment:if actual.full.alive()<self.root.paths.len() || preferred.full.alive()<self.root.paths.len() {"survival_search_incomplete"}
                        else if self.coarse_width==self.request.beam_width {"single_width_only"}
                        else if !same || !direction {"search_sensitive"}
                        else if preferred.id==actual.id || gap.abs()<1e-6 {"no_demonstrated_improvement"}
                        else {"alternative_found_not_a_proven_mistake"}}
            })
        } else {None};
        Report {status:if self.actions.is_empty(){"no_legal_root"}else if done{"complete"}else{"working"},
            processed:self.completed.len(),total_actions:self.actions.len(),depth:self.request.depth,beam_width:self.request.beam_width,
            coarse_width:self.coarse_width,scenarios:self.root.paths.len(),gravity:0,execution_costs:false,
            information_set_consistent:true,rules_parity_verified:false,
            candidates:ranked.iter().map(|c|self.candidate(c)).collect(),comparison,
            assumptions:vec!["Clock-free snapshot: pending is cancelable, only already-active packets may rise.",
                "All legal root actions use the same lock depth and beam widths, not necessarily equal node counts.",
                "Only the six observed queue pieces and actual hold are used; new unknown previews are never supplied.",
                "Identical observable states within a policy require identical actions until information is revealed.",
                "Ten clean-hole scenarios approximate a joint distribution; they are not win probabilities or all possible garbage.",
                "Beam pruning can miss continuations. A missing continuation is not a proof of forced loss.",
                "Two-width agreement tests sensitivity only. Scores do not justify automatic blunder labels.",
                "The existing S2 hybrid evaluator is reused with zero execution cost and visible-T-only slot valuation."]}
    }
    pub fn details(&self,id:usize) -> Result<Details,String> {
        let c = self.completed.iter().find(|c|c.id==id).ok_or("action is not evaluated yet")?;
        let mut lines = Vec::new();
        for (i,p) in c.full.paths.iter().enumerate() {
            let mut path = self.root.paths[i].clone(); let mut steps = Vec::new();
            for &action in &p.actions {
                let t = advance(&path,action,&self.request.start.queue,&self.config);
                path = t.path.clone(); steps.push(step_details(t,&self.request.start.queue));
            }
            lines.push(ScenarioLine{scenario:i,continuation_found:!p.dead,steps});
        }
        Ok(Details{candidate:self.candidate(c),lines})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json,Value};
    fn request(incoming:Value) -> Request {
        serde_json::from_value(json!({"start":{"board":[],"queue":["I","O","T","L","J","S"],"hold":"Z",
            "combo":0,"back_to_back":false},"incoming":incoming,"pieces_placed":30,"garbage_sent":0,"depth":3,"beam_width":2})).unwrap()
    }
    fn finish(session:&mut Session) {while !session.step() {}}
    #[test]
    fn all_roots_receive_equal_horizon_and_actual_is_always_included() {
        let r=request(json!([])); let mut s=Session::new(r).unwrap();
        let actual=*s.actions.last().unwrap(); s.request.actual=Some(actual);
        assert!(s.report().comparison.is_none()); finish(&mut s);
        let report=s.report();
        assert_eq!(report.candidates.len(),s.actions.len());
        assert_eq!(report.comparison.unwrap().actual_action_id,s.actions.len()-1);
        assert!(report.candidates.iter().all(|c|c.depth==3 && c.beam_width==2));
        for c in &s.completed {for p in &c.full.paths {assert!(p.dead || p.actions.len()==3);}}
    }
    #[test]
    fn same_type_empty_hold_remains_two_distinct_actions_with_correct_consumption() {
        let mut r=request(json!([]));r.start.hold=None;r.start.queue[1]=Piece::I;
        let s=Session::new(r).unwrap();let root=&s.root.paths[0];
        let nohold=s.actions.iter().find(|a|!a.use_hold).unwrap();
        let held=Action{use_hold:true,..*nohold};assert!(s.actions.contains(&held));
        let a=advance(root,*nohold,&s.request.start.queue,&s.config).path;
        let b=advance(root,held,&s.request.start.queue,&s.config).path;
        assert_eq!((a.pos.index,a.pos.hold),(1,None));
        assert_eq!((b.pos.index,b.pos.hold),(2,Some(Piece::I)));
        assert_eq!(a.pos.state.board,b.pos.state.board);
        assert_ne!(a.pos.observed_key(),b.pos.observed_key());
    }
    #[test]
    fn empty_hold_can_finish_five_locks_without_revealing_a_seventh_piece() {
        let mut r=request(json!([]));r.start.hold=None;r.depth=5;r.beam_width=1;
        let s=Session::new(r).unwrap();let action=*s.actions.iter().find(|a|a.use_hold).unwrap();
        let p=search(&s.root,action,5,1,&s.request.start.queue,&s.config,&mut MoveCache::default());
        assert!(!p.paths[0].dead);assert_eq!(p.paths[0].pos.index,6);assert_eq!(p.paths[0].actions.len(),5);
    }
    #[test]
    fn identical_observations_never_choose_different_hidden_hole_actions() {
        let s=Session::new(request(json!([{"lines":8,"active":true},{"lines":4,"active":true}]))).unwrap();
        assert_eq!(groups(&s.root).len(),1);
        for action in s.actions.iter().step_by(10) {
            let policy=search(&s.root,*action,3,2,&s.request.start.queue,&s.config,&mut MoveCache::default());
            let mut paths=s.root.paths.clone();
            for ply in 0..3 {
                for i in 0..paths.len() {for j in 0..i {
                    if paths[i].pos.observed_key()==paths[j].pos.observed_key() {
                        assert_eq!(policy.paths[i].actions.get(ply),policy.paths[j].actions.get(ply));
                    }
                }}
                for i in 0..paths.len() {if let Some(&a)=policy.paths[i].actions.get(ply) {
                    paths[i]=advance(&paths[i],a,&s.request.start.queue,&s.config).path;
                }}
            }
        }
    }
    #[test]
    fn deterministic_incremental_reports_and_details() {
        let mut a=Session::new(request(json!([]))).unwrap();let mut b=Session::new(request(json!([]))).unwrap();
        for _ in 0..4 {assert_eq!(a.step(),b.step());}
        assert_eq!(serde_json::to_value(a.report()).unwrap(),serde_json::to_value(b.report()).unwrap());
        for id in 0..4 {assert_eq!(serde_json::to_value(a.details(id).unwrap()).unwrap(),serde_json::to_value(b.details(id).unwrap()).unwrap());}
    }
    #[test]
    fn malformed_actual_hidden_future_and_pace_fields_are_rejected() {
        let mut r=request(json!([]));r.depth=6;assert!(Session::new(r).is_err());
        let mut r=request(json!([]));r.start.queue.push(Piece::Z);assert!(Session::new(r).is_err());
        let mut v=serde_json::json!({"start":{"board":[],"queue":["I","O","T","L","J","S"],"hold":null,"combo":0,"back_to_back":false},
            "incoming":[],"pieces_placed":30,"garbage_sent":0,"frames_per_piece":12});
        assert!(serde_json::from_value::<Request>(v.clone()).is_err());
        v.as_object_mut().unwrap().remove("frames_per_piece");
        v["start"]["randomizer"]=json!({"type":"seven_bag","bag_state":["I"]});
        assert!(Session::new(serde_json::from_value(v).unwrap()).is_err());
        let mut r=request(json!([]));let s=Session::new(request(json!([]))).unwrap();
        let mut invalid=s.actions[0];invalid.placement.location.x=100;r.actual=Some(invalid);
        assert!(Session::new(r).is_err());
    }
    #[test]
    fn immediate_explanations_match_all_recorded_fixtures() {
        let fixtures:Value=serde_json::from_str(include_str!("../tests/fixtures/replay-v19.json")).unwrap();
        for c in fixtures["cases"].as_array().unwrap() {
            let mut start=c["start"].clone();
            start["board"]=json!(c["rows"].as_array().unwrap().iter().map(|r|r.as_str().unwrap().chars().map(|ch|
                if ch=='.'{Value::Null}else{json!(ch.to_string())}).collect::<Vec<_>>()).collect::<Vec<_>>());
            let mut r=request(json!([]));r.start=serde_json::from_value(start).unwrap();
            let s=Session::new(r).unwrap();let action=Action{placement:serde_json::from_value(c["placement"].clone()).unwrap(),use_hold:false};
            assert!(s.actions.contains(&action),"fixture {}",c["index"]);
            let t=advance(&s.root.paths[0],action,&s.request.start.queue,&s.config);
            assert_eq!(json!({"lines":t.lines,"combo":t.path.pos.state.combo,
                "b2b":if t.path.pos.state.back_to_back {t.path.pos.state.b2b_count as u32+1}else{0},
                "garbage_cleared":t.garbage_cleared,"raw_attack":t.attack.packets(),"surge":t.attack.surge_released,"pc":t.pc}),c["expected"]);
        }
    }
    #[test]
    fn active_garbage_is_applied_in_first_step_and_counted_separately_from_attack() {
        let mut s=Session::new(request(json!([{"lines":8,"active":true}]))).unwrap();s.step();
        let details=s.details(0).unwrap();
        assert_eq!(details.lines.len(),10);
        for line in details.lines {let first=&line.steps[0];
            assert_eq!(first.garbage.risen,8);assert_eq!(first.garbage.cancelled,0);
            assert_eq!(first.attack.total,0);assert_eq!(first.incoming_remaining,0);
        }
    }
}
