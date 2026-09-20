// Kiwi snapshot-v3.1 boundary. No SevenBagObserver, draw log or replay-prefix input.
const PIECES=new Set(['I','O','T','L','J','S','Z']);
export const SNAPSHOT_RULE_FIELDS=Object.freeze([
  'b2bcharging','b2bcharge_at','b2bcharge_base','b2bchaining','openerphase_pieces',
  'allclears','allclear_garbage','allclear_b2b','garbagespecialbonus','clutch',
]);
// Exact mechanical envelope shared by supported TL and 40L-source stacking snapshots.
// mode and garbageare/garbagearebump are intentionally NOT pinned here.
export const SNAPSHOT_EXACT_RULES=Object.freeze({
  boardwidth:10,boardheight:20,buffer:20,bagtype:'7-bag',nextcount:5,kickset:'SRS+',
  spinbonuses:'all-mini+',allow180:true,hold:true,infinite_hold:false,
  garbageblocking:'combo blocking',garbageentry:'instant',garbagecap:8,
  garbagecapmax:40,garbagecapincrease_per_second:0,garbageattackcap:0,
  garbagephase_frames:0,garbagequeue:false,garbagetargetbonus:'none',
  receivemultiplier:1,cancelmultiplier:1,garbageabsolutecap:0,
  combotable:'multiplier',roundmode:'down',passthrough:'zero',nolockout:true,
  lockresets:15,locktime_frames:30,gravitymay20g:true,messiness_change:1,messiness_inner:0,
  are:0,lineclear_are:0,
});
const rangedInteger=new Set(['b2bcharge_at','b2bcharge_base','openerphase_pieces','allclear_garbage','allclear_b2b']);
const booleanRule=new Set(['b2bcharging','allclears','garbagespecialbonus','clutch']);

export class KiwiSnapshotError extends Error{
  constructor(code,message,details={}){super(message);this.name='KiwiSnapshotError';this.code=code;this.details=details;}
}
const fail=(code,message,details={})=>{throw new KiwiSnapshotError(code,message,details);};
export function normalizeSnapshotError(error){
  if(error instanceof KiwiSnapshotError)return {code:error.code,message:error.message,details:error.details};
  const message=String(error?.message??error);
  const m=/^([A-Z][A-Z0-9_]+):\s*(.*)$/.exec(message);
  return {code:m?.[1]??'KIWI_CORE_REJECTED',message:m?.[2]??message,details:{}};
}
const piece=p=>{
  if(p==null)return null;
  const value=String(p).toUpperCase();
  if(!PIECES.has(value))fail('PIECE_UNSUPPORTED','Unsupported piece',{piece:p});
  return value;
};
function publicRules(rules){
  const out={};
  for(const key of SNAPSHOT_RULE_FIELDS){
    if(rules[key]===undefined)fail('RULE_FIELD_MISSING','Missing public rule '+key,{rule:key});
    const value=rules[key];
    if(key==='b2bchaining'){
      if(value!==false)fail('RULE_VALUE_UNSUPPORTED','b2bchaining=true is not supported',{rule:key,value,supported:false});
    }else if(rangedInteger.has(key)){
      if(!Number.isInteger(value)||value<0||value>10000)
        fail('RULE_VALUE_OUT_OF_RANGE','Unsupported integer public rule value',{rule:key,value,min:0,max:10000});
    }else if(booleanRule.has(key)&&typeof value!=='boolean'){
      fail('RULE_TYPE_INVALID','Public rule must be boolean',{rule:key,value});
    }
    out[key]=value;
  }
  return out;
}
function integerTimingRule(rules,key){
  const value=rules[key];
  if(!Number.isInteger(value)||value<0||value>10000)
    fail('RULE_VALUE_OUT_OF_RANGE','Unsupported timing rule value',{rule:key,value,min:0,max:10000});
  return value;
}
export function assertSupportedSnapshotRules(rules){
  if(!['tl','40l'].includes(rules.mode))
    fail('ANALYSIS_MODE_UNSUPPORTED','Only TL and 40L-source competitive stacking are supported',{mode:rules.mode});
  for(const [key,value] of Object.entries(SNAPSHOT_EXACT_RULES)){
    if(rules[key]!==value)fail('RULE_VALUE_UNSUPPORTED','Unsupported snapshot rule '+key,{rule:key,value:rules[key],supported:value});
  }
  integerTimingRule(rules,'garbageare');
  integerTimingRule(rules,'garbagearebump');
  publicRules(rules);
  return true;
}
function incomingTL(state){
  const a=state.attack;
  if(!a)fail('SNAPSHOT_ATTACK_STATE_MISSING','TL snapshot requires current attack state');
  const existingAreLines=(a.are??[]).reduce((n,p)=>n+Math.max(0,p?.amt||0),0);
  if(existingAreLines>0)
    fail('PENDING_ARE_QUEUE_UNSUPPORTED','Positive existing ARE queue is not modeled by snapshot forecast',{lines:existingAreLines});
  const out=[];
  for(const p of a.pending??[]){
    if(p.amt<=0)continue;
    if(p.hardened)fail('PENDING_PACKET_HARDENED_UNSUPPORTED','Hardened pending packet is unsupported',{cid:p.cid});
    if(p.shielded)fail('PENDING_PACKET_SHIELDED_UNSUPPORTED','Shielded pending packet is unsupported',{cid:p.cid});
    if(p.status!=='spawn')fail('PENDING_PACKET_STATUS_UNSUPPORTED','Pending packet status is unsupported',{cid:p.cid,status:p.status});
    let ready=0;
    if(!p.active){
      if(!Number.isInteger(p.activeFrame))
        fail('PENDING_ACTIVATION_UNKNOWN','Observable pending packet has no confirmed activation frame',{cid:p.cid});
      ready=p.activeFrame-state.frame;
      if(ready<=0)fail('PENDING_ACTIVATION_INVALID','Inactive packet activation is not in the future',{cid:p.cid,activeFrame:p.activeFrame,frame:state.frame});
    }
    out.push({lines:p.amt,ready_in_frames:ready});
  }
  return {packets:out,existingAreLines};
}
function rootState(pieceState){
  return {
    x:pieceState.x,y:pieceState.y,hy:pieceState.hy,rotation:pieceState.r,kick:pieceState.kick??0,
    rotated:Boolean(pieceState.rotated),spin:pieceState.spin??'none',
    total_rotations:pieceState.totalRotations??0,resets:pieceState.resets??0,
    rotation_resets:pieceState.rotationResets??0,locking:pieceState.locking??0,
    force_lock:Boolean(pieceState.forceLock),safelock:pieceState.safelock??0,
    soft_dropped:Boolean(pieceState.softDropped),wall:Boolean(pieceState.wall),
  };
}
function modeContract(state){
  const sourceMode=state.rules.mode;
  if(sourceMode==='tl'){
    const pending=incomingTL(state);
    return {
      analysisMode:'tl',sourceMode,
      combo:state.attack.combo??0,backToBack:(state.attack.btb??0)>0,
      b2bCount:Math.max(0,(state.attack.btb??0)-1),
      incoming:pending.packets,existingAreLines:pending.existingAreLines,
      piecesPlaced:state.stats.pieces,garbageSent:state.attack.cumulativeSent??0,
      authorityFrame:state.frame,
      garbageMultiplier:state.attack.multiplier??state.rules.garbagemultiplier,
      garbageMarginFrames:state.rules.garbagemargin_frames,
      garbageIncreasePerSecond:state.rules.garbageincrease_per_second,
    };
  }
  if(sourceMode==='40l'){
    if(state.attack!==null&&state.attack!==undefined)
      fail('STACKING_ATTACK_STATE_UNEXPECTED','40L competitive-stacking snapshot must not carry TL attack state');
    return {
      analysisMode:'competitive_stacking',sourceMode,
      combo:0,backToBack:false,b2bCount:0,incoming:[],existingAreLines:0,
      // These are deliberately neutral competitive-evaluator roots, not claims
      // about solo attack history or 40L attack semantics.
      piecesPlaced:0,garbageSent:0,
      authorityFrame:null,garbageMultiplier:null,garbageMarginFrames:null,
      garbageIncreasePerSecond:null,
    };
  }
  fail('ANALYSIS_MODE_UNSUPPORTED','Unsupported source mode',{mode:sourceMode});
}
export function captureSnapshot(state,{rootGeometry}={}){
  if(!state.playing||!state.piece||state.piece.sleeping)fail('SNAPSHOT_NOT_PLAYABLE','No active playable snapshot');
  assertSupportedSnapshotRules(state.rules);
  if(state.board.rows.length!==40||state.board.rows.some(r=>r.length!==10))
    fail('BOARD_GEOMETRY_UNSUPPORTED','Expected 10 by 40 board');
  if(!rootGeometry||!Array.isArray(rootGeometry.placements))
    fail('ROOT_GEOMETRY_REQUIRED','Authority root geometry enumeration is required before snapshot analysis');
  const board=state.board.rows.map(row=>row.map(cell=>{
    if(cell==null)return null;
    if(cell==='gb')return 'G';
    if(cell==='gbd')fail('PERMANENT_GARBAGE_UNSUPPORTED','Permanent garbage is unsupported');
    return piece(cell);
  })).reverse();
  const queue=[piece(state.piece.type),...state.bag.queue.slice(0,5).map(piece)];
  if(queue.length!==6||queue.some(p=>p===null))fail('VISIBLE_QUEUE_INVALID','Expected current plus NEXT 5');
  const current=queue[0];
  if(rootGeometry.placements.some(p=>piece(p?.location?.type)!==current))
    fail('ROOT_GEOMETRY_PIECE_MISMATCH','Authority root geometry contains another piece type');
  const mode=modeContract(state);
  return {
    analysis_mode:mode.analysisMode,source_mode:mode.sourceMode,
    board,queue,hold:piece(state.hold.piece),hold_locked:Boolean(state.hold.locked),
    combo:mode.combo,back_to_back:mode.backToBack,b2b_count:mode.b2bCount,
    root_state:rootState(state.piece),
    root_legal_placements:structuredClone(rootGeometry.placements),
    root_geometry_states:rootGeometry.states_explored??null,
    rules:publicRules(state.rules),
    timing_rules:{
      garbage_are_frames:integerTimingRule(state.rules,'garbageare'),
      garbage_are_bump_frames:integerTimingRule(state.rules,'garbagearebump'),
      garbage_locked_until_frame:Number.isInteger(state.garbageLockedUntil)?state.garbageLockedUntil:0,
    },
    incoming:mode.incoming,existing_are_lines:mode.existingAreLines,
    pieces_placed:mode.piecesPlaced,garbage_sent:mode.garbageSent,
    authority_frame:mode.authorityFrame,authority_subframe:state.subframe,
    garbage_multiplier:mode.garbageMultiplier,garbage_margin_frames:mode.garbageMarginFrames,
    garbage_increase_per_second:mode.garbageIncreasePerSecond,
  };
}
export function captureSnapshotFromEngine(engine,placementTools){
  const geometry=placementTools.enumerateRootPlacements(engine);
  return captureSnapshot(engine.state,{rootGeometry:geometry});
}
export function buildSnapshotRequest(v,{nodeBudget=200000,framesPerPiece=24}={}){
  if(!Number.isInteger(nodeBudget)||nodeBudget<1000||nodeBudget>2000000)
    fail('NODE_BUDGET_INVALID','Invalid hard node budget',{nodeBudget});
  if(!v.hold_locked&&nodeBudget<2000)
    fail('NODE_BUDGET_TOO_SMALL_FOR_HOLD','Unlocked roots need >=2000 nodes for split root actions',{nodeBudget});
  if(!Number.isInteger(framesPerPiece)||framesPerPiece<1||framesPerPiece>600)
    fail('PACE_ASSUMPTION_INVALID','Invalid hypothetical pace',{framesPerPiece});
  if(!Array.isArray(v.queue)||v.queue.length!==6)fail('VISIBLE_QUEUE_INVALID','Expected current plus exactly NEXT 5');
  if(!Array.isArray(v.root_legal_placements))fail('ROOT_GEOMETRY_REQUIRED','Missing authority root landing allowlist');
  if(!v.timing_rules)fail('TIMING_RULES_MISSING','garbage ARE/bump rules must be preserved explicitly');
  return {
    schema:'kiwi-snapshot/3',analysis_mode:v.analysis_mode,source_mode:v.source_mode,
    bag_knowledge:'unknown',unknown_tail:'finite_visible',
    timing_rules:{...v.timing_rules},existing_are_lines:v.existing_are_lines,
    start:{board:v.board.map(r=>[...r]),queue:v.queue.map(piece),hold:piece(v.hold),
      combo:v.combo,back_to_back:v.back_to_back,b2b_count:v.b2b_count},
    root_state:{...v.root_state},
    root_legal_placements:structuredClone(v.root_legal_placements),
    rules:publicRules(v.rules),hold_locked:Boolean(v.hold_locked),
    incoming:v.incoming.map(p=>{
      if(!Number.isInteger(p.ready_in_frames))
        fail('PENDING_ACTIVATION_UNKNOWN','Explicit packet activation is required');
      return {lines:p.lines,ready_in_frames:p.ready_in_frames};
    }),
    pieces_placed:v.pieces_placed,garbage_sent:v.garbage_sent,
    frames_per_piece:framesPerPiece,authority_frame:v.authority_frame,authority_subframe:v.authority_subframe,
    garbage_multiplier:v.garbage_multiplier,garbage_margin_frames:v.garbage_margin_frames,
    garbage_increase_per_second:v.garbage_increase_per_second,node_budget:nodeBudget,
  };
}
function expectedSamePiece(state,mode){
  const current=piece(state.piece.type);
  const replacement=mode==='empty'?piece(state.bag.queue[0]):piece(state.hold.piece);
  return replacement===current;
}
export function validateSnapshotAction(engine,action,{Engine,placementTools}){
  const state=engine.state;
  if(action?.kind==='hold'){
    if('placement'in action||'path'in action||action.requires_reanalysis!==true)
      fail('HOLD_ACTION_BUNDLED_LANDING','Hold action must not contain a landing');
    const expectedMode=state.hold.piece==null?'empty':'occupied';
    if(action.mode!==expectedMode)fail('HOLD_MODE_MISMATCH','Hold mode does not match authority state',{expected:expectedMode,actual:action.mode});
    if(state.hold.locked||!state.rules.hold)fail('HOLD_LOCKED','Hold is not available at this root');
    const same=expectedSamePiece(state,expectedMode);
    if(Boolean(action.same_piece)!==same)fail('HOLD_SAME_PIECE_FLAG_MISMATCH','same_piece flag does not match visible replacement',{expected:same});
    return {action:structuredClone(action),geometry_validated:true,timing_validated:false};
  }
  if(action?.kind!=='place'||action.placement?.location.type!==piece(state.piece.type))
    fail('PLACE_CURRENT_PIECE_REQUIRED','Place action must use current piece without Hold');
  let path;
  try{path=placementTools.findCurrentPath(state,action.placement);}
  catch(error){fail('ROOT_GEOMETRY_UNREACHABLE','Placement is not reachable from actual root pose',{cause:String(error?.message??error)});}
  if(path.useHold||path.moves.includes('hold'))fail('PLACE_IMPLICIT_HOLD','Place action cannot implicitly Hold');
  return {action:structuredClone(action),path,geometry_validated:true,timing_validated:false};
}
export function validateSnapshotTimingAction(engine,action,{Engine,placementTools,framesPerPiece=24}){
  const checked=validateSnapshotAction(engine,action,{Engine,placementTools});
  if(action.kind==='hold')return {...checked,timing_reason:'hold_is_atomic_and_requires_post_hold_request'};
  const startFrame=engine.state.frame,lockFrame=startFrame+framesPerPiece-1;
  let inputs;
  try{inputs=placementTools.schedulePath(startFrame,lockFrame,checked.path.moves,engine);}
  catch(error){fail('ROOT_TIMING_UNEXECUTABLE','Geometry is reachable but the requested timing transport failed',{cause:String(error?.message??error),startFrame,lockFrame});}
  return {...checked,inputs,startFrame,lockFrame,timing_validated:true};
}
export function selectReachableSnapshotAction(engine,report,dependencies,{validateTiming=false,framesPerPiece=24}={}){
  if(report?.schema!=='kiwi-snapshot-result/3')fail('RESULT_SCHEMA_INVALID','Unexpected snapshot result schema');
  const failures=[];
  for(let index=0;index<report.candidates.length;index++){
    const c=report.candidates[index];
    try{
      const checked=validateTiming
        ?validateSnapshotTimingAction(engine,c.action,{...dependencies,framesPerPiece})
        :validateSnapshotAction(engine,c.action,dependencies);
      return {...checked,candidate_index:index,search_basis:c.search_basis};
    }catch(error){failures.push({index,error:normalizeSnapshotError(error)});}
  }
  fail(validateTiming?'NO_TIMING_EXECUTABLE_ACTION':'NO_GEOMETRY_REACHABLE_ACTION',
    'No snapshot candidate passed authority validation',{failures});
}
export function applyHoldForReanalysis(engine,action,{Engine}){
  const state=engine.state,expectedMode=state.hold.piece==null?'empty':'occupied';
  if(action?.kind!=='hold'||action.requires_reanalysis!==true||'placement'in action||'path'in action)
    fail('HOLD_ACTION_INVALID','Expected standalone Hold action without landing');
  if(action.mode!==expectedMode)
    fail('HOLD_MODE_MISMATCH','Hold mode does not match authority state',{expected:expectedMode,actual:action.mode});
  const same=expectedSamePiece(state,expectedMode);
  if(Boolean(action.same_piece)!==same)
    fail('HOLD_SAME_PIECE_FLAG_MISMATCH','same_piece flag does not match visible replacement',{expected:same});
  const fork=Engine.restore(engine.serialize());
  if(!fork.hold())fail('HOLD_LOCKED','Authority rejected Hold');
  return fork;
}
export function snapshotRuleContract(){
  return {
    source_modes:{
      tl:{analysis_mode:'tl',attack_state:'required',root_counters:'snapshot'},
      '40l':{analysis_mode:'competitive_stacking',attack_state:'absent',root_counters:'neutral combo/B2B'},
    },
    exact:{...SNAPSHOT_EXACT_RULES},
    variable:{
      garbageare:'integer 0..10000, preserved but not exactly simulated',
      garbagearebump:'integer 0..10000, preserved but not exactly simulated',
      b2bcharging:'boolean',b2bcharge_at:'integer 0..10000',b2bcharge_base:'integer 0..10000',
      b2bchaining:'false only',openerphase_pieces:'integer 0..10000',allclears:'boolean',
      allclear_garbage:'integer 0..10000',allclear_b2b:'integer 0..10000',
      garbagespecialbonus:'boolean',clutch:'boolean',
    },
    approximations:{
      exact_are_bump_timing:false,
      garbageare_rule_preserved:true,
      garbagearebump_rule_preserved:true,
      competitive_stacking_is_tl_parity:false,
      competitive_stacking_is_40l_score_optimizer:false,
    },
    rejections:{
      positive_are:'PENDING_ARE_QUEUE_UNSUPPORTED',
      unknown_activation:'PENDING_ACTIVATION_UNKNOWN',
      hardened:'PENDING_PACKET_HARDENED_UNSUPPORTED',
      shielded:'PENDING_PACKET_SHIELDED_UNSUPPORTED',
      unsupported_status:'PENDING_PACKET_STATUS_UNSUPPORTED',
      unsupported_rule:'RULE_VALUE_UNSUPPORTED',
      unsupported_mode:'ANALYSIS_MODE_UNSUPPORTED',
    },
  };
}
