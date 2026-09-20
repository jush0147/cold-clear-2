// Kiwi snapshot-v2 boundary. No SevenBagObserver, draw log or replay-prefix input.
const PIECES = new Set(['I','O','T','L','J','S','Z']);
export const SNAPSHOT_RULE_FIELDS = Object.freeze([
  'b2bcharging','b2bcharge_at','b2bcharge_base','b2bchaining','openerphase_pieces',
  'allclears','allclear_garbage','allclear_b2b','garbagespecialbonus','clutch',
]);
const piece = p => {
  if (p == null) return null;
  const value=String(p).toUpperCase();
  if(!PIECES.has(value))throw new Error('Unsupported piece');
  return value;
};
function publicRules(rules) {
  const out={};
  for(const key of SNAPSHOT_RULE_FIELDS) {
    if(rules[key]===undefined)throw new Error('Missing public rule '+key);
    out[key]=rules[key];
  }
  return out;
}
function supportedRules(r) {
  const exact={boardwidth:10,boardheight:20,buffer:20,bagtype:'7-bag',kickset:'SRS+',
    spinbonuses:'all-mini+',allow180:true,hold:true,infinite_hold:false,
    garbageblocking:'combo blocking',garbageentry:'instant',garbagecap:8,
    garbageattackcap:0,allclear_b2b:1,receivemultiplier:1,cancelmultiplier:1,
    garbageabsolutecap:0,combotable:'multiplier',roundmode:'down'};
  for(const [key,value] of Object.entries(exact)) {
    if(r[key]!==value)throw new Error('Unsupported snapshot public rule '+key);
  }
  if(r.b2bchaining!==false)throw new Error('Unsupported b2bchaining');
}
function incoming(state) {
  const a=state.attack;
  if(!a)return [];
  if(a.are.some(p=>p.amt>0))throw new Error('Active ARE queue is not supported by the snapshot forecast');
  const out=[];
  for(const p of a.pending) {
    if(p.amt<=0)continue;
    if(p.hardened||p.shielded||p.status!=='spawn')throw new Error('Unsupported observable packet');
    let ready=0;
    if(!p.active) {
      if(!Number.isInteger(p.activeFrame))throw new Error('Observable packet has unknown activation frame');
      ready=p.activeFrame-state.frame;
      if(ready<=0)throw new Error('Inactive packet does not have future activation');
    }
    out.push({lines:p.amt,ready_in_frames:ready});
  }
  return out;
}

// Lightweight allowlist projection from an already reconstructed current state.
// This does NOT seek, replay, serialize the checkpoint, or read any earlier draw.
export function captureSnapshot(state) {
  if(!state.playing||!state.piece||state.piece.sleeping)throw new Error('No active playable snapshot');
  supportedRules(state.rules);
  if(state.board.rows.length!==40||state.board.rows.some(r=>r.length!==10))throw new Error('Expected 10 by 40 board');
  const board=state.board.rows.map(row=>row.map(cell=>{
    if(cell==null)return null;
    if(cell==='gb')return 'G';
    if(cell==='gbd')throw new Error('Permanent garbage is unsupported');
    return piece(cell);
  })).reverse();
  const queue=[piece(state.piece.type),...state.bag.queue.slice(0,5).map(piece)];
  if(queue.length!==6||queue.some(p=>p===null))throw new Error('Expected current plus NEXT 5');
  return {
    board,queue,hold:piece(state.hold.piece),hold_locked:Boolean(state.hold.locked),
    combo:state.attack?.combo??0,back_to_back:(state.attack?.btb??0)>0,
    b2b_count:Math.max(0,(state.attack?.btb??0)-1),
    root_pose:{x:state.piece.x,y:state.piece.y,rotation:state.piece.r},
    rules:publicRules(state.rules),incoming:incoming(state),
    pieces_placed:state.stats.pieces,garbage_sent:state.attack?.cumulativeSent??0,
    authority_frame:state.frame,authority_subframe:state.subframe,
    garbage_multiplier:state.attack?.multiplier??state.rules.garbagemultiplier,
    garbage_margin_frames:state.rules.garbagemargin_frames,
    garbage_increase_per_second:state.rules.garbageincrease_per_second,
  };
}

// Input is the detached projection above, NOT the complete replay/Engine state.
// Intentionally copy named fields rather than serializing the caller's object.
export function buildSnapshotRequest(v,{nodeBudget=200000,framesPerPiece=24}={}) {
  if(!Number.isInteger(nodeBudget)||nodeBudget<1000||nodeBudget>2000000)throw new Error('Invalid hard node budget');
  if(!Number.isInteger(framesPerPiece)||framesPerPiece<1||framesPerPiece>600)throw new Error('Invalid hypothetical pace');
  if(!Array.isArray(v.queue)||v.queue.length!==6)throw new Error('Expected current plus exactly NEXT 5');
  return {
    schema:'kiwi-snapshot/2',bag_knowledge:'unknown',unknown_tail:'finite_visible',
    start:{board:v.board.map(r=>[...r]),queue:v.queue.map(piece),hold:piece(v.hold),
      combo:v.combo,back_to_back:v.back_to_back,b2b_count:v.b2b_count},
    root_pose:{x:v.root_pose.x,y:v.root_pose.y,rotation:v.root_pose.rotation},
    rules:publicRules(v.rules),hold_locked:v.hold_locked,
    incoming:v.incoming.map(p=>{
      if(!Number.isInteger(p.ready_in_frames))throw new Error('Explicit packet activation is required');
      return {lines:p.lines,ready_in_frames:p.ready_in_frames};
    }),
    pieces_placed:v.pieces_placed,garbage_sent:v.garbage_sent,
    frames_per_piece:framesPerPiece,authority_frame:v.authority_frame,authority_subframe:v.authority_subframe,
    garbage_multiplier:v.garbage_multiplier,garbage_margin_frames:v.garbage_margin_frames,
    garbage_increase_per_second:v.garbage_increase_per_second,node_budget:nodeBudget,
  };
}

// Tetrp-side validation only; never mutates the supplied authority.
// The core remains spawn-based. Validate candidate geometry from the real pose.
// Returned Hold actions have no path/landing. The caller must Hold then re-analyze.
export function validateSnapshotAction(engine,action,{Engine,placementTools}) {
  const root=Engine.restore(engine.serialize());
  if(action?.kind==='hold') {
    if('placement' in action||'path' in action||action.requires_reanalysis!==true)
      throw new Error('A Hold action must not bundle a landing');
    const expectedMode=root.state.hold.piece==null?'empty':'occupied';
    if(action.mode!==expectedMode||root.state.hold.locked||!root.hold()||!root.state.playing)
      throw new Error('Hold is not legal at this snapshot');
    return {action:{kind:'hold',mode:expectedMode,requires_reanalysis:true}};
  }
  if(action?.kind!=='place'||action.placement?.location.type!==piece(root.state.piece.type))
    throw new Error('Place must use the current piece without Hold');
  const path=placementTools.findPath(root,action.placement);
  if(path.useHold||path.moves.includes('hold'))throw new Error('Place action cannot implicitly Hold');
  return {action:structuredClone(action),path};
}

export function selectReachableSnapshotAction(engine,report,dependencies) {
  if(report?.schema!=='kiwi-snapshot-result/2')throw new Error('Unexpected snapshot result schema');
  for(let index=0;index<report.candidates.length;index++) {
    const c=report.candidates[index];
    try { return {...validateSnapshotAction(engine,c.action,dependencies),candidate_index:index}; }
    catch(error) {
      // Geometry-filtering is separate from the heuristic's ranking. If every
      // action is unreachable, fail rather than painting a fabricated placement.
      if(index===report.candidates.length-1)throw new Error('No authority-reachable snapshot action');
    }
  }
  throw new Error('Snapshot search returned no actions');
}
