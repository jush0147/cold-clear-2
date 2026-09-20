import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const root=path.resolve(process.argv[2]);
const output=process.argv[3]||'kiwi-product-rule-fixtures.json';
const load=p=>import(pathToFileURL(path.join(root,'src',p)).href);
const {Engine}=await load('engine.js');
const A=await load('attack.js');
const B=await load('board.js');
const {createHoles}=await load('random.js');
const handling={safelock:false,irs:'off',ihs:'off'};
const make=(rules={})=>new Engine({mode:'tl',seed:42,rules:{g:0,gincrease:0,b2bcharge_base:3,...rules},handling});
const rows=s=>s.board.rows.map(row=>row.map(c=>c==null?null:(c==='gb'||c==='gbd'?'G':'X'))).reverse();
const fixtures=[];

// Opener double-cancel: fresh direct Tetrp attack-state observations around the
// exact opener boundary, cumulative-sent threshold and packet splits.
for(const limit of [0,1,2,14,20]){
  const placedValues=[0,Math.max(0,limit-1),limit,limit+1].filter((v,i,a)=>a.indexOf(v)===i);
  for(const placed of placedValues)for(const sent of [0,1,4,8])
  for(const amounts of [[1],[5],[1,4],[2,2,3],[8,1]])
  for(const attacks of [[1],[3],[7],[1,2,4],[8,1]]){
    const authority=make({openerphase_pieces:limit});
    const rules=authority.state.rules,s=A.createAttack();
    // Tetrp increments placement count before fight(); CC2 Forecast tracks
    // placements completed before the hypothetical next placement.
    s.pieces=placed+1;s.cumulativeSent=sent;
    s.pending=amounts.map((amt,i)=>({cid:i+1,sender:'P2',iid:i+1,ackiid:0,amt,
      active:true,status:'spawn',hardened:false,shielded:false,column:null,
      confirmFrame:0,activeFrame:0}));
    const holes=createHoles(42);
    for(const attack of attacks)A.fight(s,attack,rules,holes);
    fixtures.push({kind:'cancel',limit,placed,sent,amounts,attacks,
      expected_remaining:A.pendingCount(s),expected_sent:s.cumulativeSent});
  }
}

// Spawn/blockout differential matrix. The authority reason is preserved so the
// report distinguishes topout from garbagesmash; Kiwi only claims the bounded
// playable/no-playable geometry correspondence here, not terminal-reason parity.
for(const type of ['i','o','t','l','j','s','z'])for(const clutch of [false,true])
for(const lastClear of [false,true])for(const height of [22,40])for(const garbage of [false,true]){
  const e=make({clutch});
  for(let y=40-height;y<40;y++)e.state.board.rows[y][4]='i';
  e.state.lastClear=lastClear;e.state.board.lastWasAttack=garbage;
  const board=rows(e.state);e.spawn(type);
  fixtures.push({kind:'spawn_rescue',board,piece:type.toUpperCase(),clutch,lastClear,garbage,
    expected_playing:e.state.playing,authority_reason:e.state.reason});
}

// Representative clear -> next-spawn rescue, including the public clutch toggle.
for(const clutch of [false,true]){
  const e=make({clutch});
  for(let y=17;y<40;y++)e.state.board.rows[y][4]='i';
  for(let x=4;x<10;x++)e.state.board.rows[39][x]='i';
  const board=rows(e.state);
  Object.assign(e.state.piece,{type:'i',x:1,y:39,hy:39,r:0,kick:0,rotated:false,
    spin:'none',sleeping:false,locking:0,resets:0,rotationResets:0,totalRotations:0,
    safelock:0,keys:0,softDropped:false,forceLock:false,wall:false});
  e.state.bag.queue[0]='i';e.state.attack.combo=1;
  const rules=Object.fromEntries(['b2bcharging','b2bcharge_at','b2bcharge_base','b2bchaining',
    'openerphase_pieces','allclears','allclear_garbage','allclear_b2b','garbagespecialbonus','clutch']
    .map(k=>[k,e.state.rules[k]]));
  e.step([{frame:0,type:'keydown',key:'hardDrop',subframe:0.5}]);
  fixtures.push({kind:'clear_rescue',board,rules,
    placement:{location:{type:'I',orientation:'north',x:1,y:0},spin:'none'},
    expected_board:rows(e.state),expected_playing:e.state.playing,
    expected_combo:e.state.attack.combo,expected_lines:e.state.stats.lines,
    authority_reason:e.state.reason});
}

// Garbage-smash storage boundary. A full top row rejects insertion; a partial
// top row is shifted. This directly guards Forecast's top boundary.
for(const full of [false,true]){
  const e=make();
  if(full)for(let x=0;x<10;x++)e.state.board.rows[0][x]='i';
  else e.state.board.rows[0][0]='i';
  const before=rows(e.state);
  const ok=e.insertGarbage(4);
  fixtures.push({kind:'garbage_boundary',full,before,expected_ok:ok,
    expected_playing:e.state.playing,authority_reason:e.state.reason});
}

// Public rule variants that the product deliberately transports rather than
// pinning. This proves the pinned authority accepts/reflects those values. Other
// engine rules are separately exact-match gated by the JS adapter.
const variants=[
  {b2bcharging:false,b2bcharge_at:7,b2bcharge_base:5,openerphase_pieces:20,
   allclears:false,allclear_garbage:7,allclear_b2b:2,garbagespecialbonus:false,clutch:false},
  {b2bcharging:true,b2bcharge_at:1,b2bcharge_base:9,openerphase_pieces:0,
   allclears:true,allclear_garbage:0,allclear_b2b:0,garbagespecialbonus:true,clutch:true},
];
for(const requested of variants){
  const e=make(requested);
  fixtures.push({kind:'rule_variant',requested,
    actual:Object.fromEntries(Object.keys(requested).map(k=>[k,e.state.rules[k]]))});
}

fs.writeFileSync(output,JSON.stringify(fixtures));
const counts=Object.fromEntries(['cancel','spawn_rescue','clear_rescue','garbage_boundary','rule_variant']
  .map(k=>[k,fixtures.filter(f=>f.kind===k).length]));
console.log(JSON.stringify({status:'generated',fixtures:fixtures.length,counts,
  tetrp_ref_process:'pinned checkout passed by workflow',
  scope:'opener cancellation/cumulative-sent boundary, spawn rescue, representative clear-rescue, garbage-smash storage boundary, transported public-rule variants; full ARE/bump and terminal-reason parity remain out of scope'},null,2));
