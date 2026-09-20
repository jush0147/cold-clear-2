import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';

const root=path.resolve(process.argv[2]);
const out=path.resolve(process.argv[3]||'kiwi-rule-fixtures.json');
const load=p=>import(pathToFileURL(path.join(root,'src',p)).href);
const {Engine}=await load('engine.js');
const A=await load('attack.js');
const {createHoles}=await load('random.js');

const handling={safelock:false,irs:'off',ihs:'off'};
const make=(clutch=true,rules={})=>new Engine({
  mode:'tl',seed:42,rules:{g:0,gincrease:0,b2bcharge_base:3,clutch,...rules},handling
});
const rows=s=>s.board.rows.map(row=>row.map(c=>c==null?null:'X')).reverse();
const publicRules=s=>Object.fromEntries([
  'b2bcharging','b2bcharge_at','b2bcharge_base','b2bchaining','openerphase_pieces',
  'allclears','allclear_garbage','allclear_b2b','garbagespecialbonus','clutch'
].map(k=>[k,s.rules[k]]));
const fixtures=[];

// Opening double-cancel differential grid. placed is pieces already locked;
// Tetrp fight() stores pieces including the current placement, hence +1 below.
for(const limit of [0,2,14,20]){
  const placedValues=[0,1,Math.max(0,limit-2),Math.max(0,limit-1),limit,limit+1];
  for(const placed of [...new Set(placedValues)])
  for(const sent of [0,1,2,5,8,13])
  for(const amounts of [[],[1],[5],[1,4],[2,2,3],[8,1]])
  for(const attacks of [[1],[3],[7],[1,2,4],[4,4]]){
    const e=make(true,{openerphase_pieces:limit});
    const s=A.createAttack();
    s.pieces=placed+1;
    s.cumulativeSent=sent;
    s.pending=amounts.map((amt,i)=>({
      cid:i+1,amt,active:true,status:'spawn',hardened:false,shielded:false
    }));
    const holes=createHoles(42);
    for(const attack of attacks)A.fight(s,attack,e.state.rules,holes);
    fixtures.push({
      kind:'cancel',limit,placed,sent,amounts,attacks,
      expected_remaining:A.pendingCount(s),expected_sent:s.cumulativeSent
    });
  }
}

for(const type of ['i','o','t','l','j','s','z'])
for(const clutch of [false,true])
for(const lastClear of [false,true])
for(const height of [20,22,39,40])
for(const garbage of [false,true]){
  const e=make(clutch);
  for(let y=40-height;y<40;y++)e.state.board.rows[y][4]='i';
  e.state.lastClear=lastClear;
  e.state.board.lastWasAttack=garbage;
  const board=rows(e.state);
  e.spawn(type);
  fixtures.push({
    kind:'spawn_rescue',board,piece:type.toUpperCase(),clutch,lastClear,garbage,
    expected_playing:e.state.playing,authority_reason:e.state.reason
  });
}

for(const clutch of [false,true]){
  const e=make(clutch);
  for(let y=17;y<40;y++)e.state.board.rows[y][4]='i';
  for(let x=4;x<10;x++)e.state.board.rows[39][x]='i';
  const board=rows(e.state);
  Object.assign(e.state.piece,{
    type:'i',x:1,y:39,hy:39,r:0,kick:0,rotated:false,spin:'none',
    totalRotations:0,resets:0,rotationResets:0,locking:0,forceLock:false,safelock:0
  });
  e.state.bag.queue[0]='i';
  e.state.attack.combo=1;
  const rules=publicRules(e.state);
  e.step([{frame:0,type:'keydown',key:'hardDrop',subframe:0.5}]);
  fixtures.push({
    kind:'clear_rescue',board,rules,
    placement:{location:{type:'I',orientation:'north',x:1,y:0},spin:'none'},
    expected_board:rows(e.state),expected_playing:e.state.playing,
    expected_combo:e.state.attack.combo,expected_lines:e.state.stats.lines,
    authority_reason:e.state.reason
  });
}

for(const fill of ['partial','full']){
  const e=make(true);
  if(fill==='partial')e.state.board.rows[0][0]='i';
  else for(let x=0;x<10;x++)e.state.board.rows[0][x]='i';
  const board=rows(e.state);
  const inserted=e.insertGarbage(4);
  fixtures.push({
    kind:'garbage_top_boundary',fill,board,
    expected_inserted:inserted,expected_playing:e.state.playing,
    authority_reason:e.state.reason
  });
}


// Variable public attack-rule values that snapshot-v3 transports rather than
// exact-pins. These are read back from the pinned Tetrp authority and compared
// against the exact Kiwi rule struct in the Rust differential test.
const supportedRuleVariants=[
  {b2bcharging:false,b2bcharge_at:7,b2bcharge_base:5,b2bchaining:false,
   openerphase_pieces:20,allclears:false,allclear_garbage:7,allclear_b2b:2,
   garbagespecialbonus:false,clutch:false},
  {b2bcharging:true,b2bcharge_at:1,b2bcharge_base:9,b2bchaining:false,
   openerphase_pieces:0,allclears:true,allclear_garbage:0,allclear_b2b:0,
   garbagespecialbonus:true,clutch:true},
  {b2bcharging:true,b2bcharge_at:4,b2bcharge_base:3,b2bchaining:false,
   openerphase_pieces:14,allclears:true,allclear_garbage:5,allclear_b2b:1,
   garbagespecialbonus:true,clutch:true},
];
for(const requested of supportedRuleVariants){
  const e=make(requested.clutch,requested);
  fixtures.push({kind:'rule_variant_supported',requested,actual:publicRules(e.state)});
}

// Tetrp can expose rule combinations that this Kiwi contract intentionally does
// not implement. Record the authority value; the comparator must keep rejecting
// it rather than silently normalizing it to the supported rule.
{
  const requested={b2bchaining:true};
  const e=make(true,requested);
  fixtures.push({
    kind:'rule_variant_rejected',
    requested,
    actual:publicRules(e.state),
    expected_rejection:'b2bchaining=true is not supported'
  });
}

fs.writeFileSync(out,JSON.stringify(fixtures));
const kinds=Object.fromEntries([...new Set(fixtures.map(f=>f.kind))]
  .map(k=>[k,fixtures.filter(f=>f.kind===k).length]));
const reasons=Object.fromEntries([...new Set(fixtures.map(f=>f.authority_reason).filter(Boolean))]
  .map(k=>[k,fixtures.filter(f=>f.authority_reason===k).length]));
console.log(JSON.stringify({
  schema:'kiwi-rule-fixtures/2',fixtures:fixtures.length,kinds,reasons,
  scope:[
    'opening cancellation across pending/cumulative-sent/opener boundaries',
    'spawn rescue and topout/garbagesmash reason observations',
    'representative clear-to-rescue transitions',
    'full-vs-partial storage-top garbage-smash boundary',
    'supported transported public-rule variants plus an explicitly rejected Tetrp-visible variant'
  ],
  not_certified:[
    'positive existing ARE queue forecast',
    'garbage ARE/bump exact timing',
    'active-piece repair failure after garbage insertion',
    'complete terminal-reason parity'
  ]
},null,2));
