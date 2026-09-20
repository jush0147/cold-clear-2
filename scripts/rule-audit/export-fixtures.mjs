import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const root=path.resolve(process.argv[2]);
const load=p=>import(pathToFileURL(path.join(root,'src',p)).href);
const {Engine}=await load('engine.js'),A=await load('attack.js'),{createHoles}=await load('random.js');
const make=clutch=>new Engine({mode:'tl',seed:42,rules:{g:0,gincrease:0,b2bcharge_base:3,clutch},handling:{safelock:false,irs:'off',ihs:'off'}});
const rows=s=>s.board.rows.map(row=>row.map(c=>c==null?null:'X')).reverse();
const fixtures=[];
for(const limit of [0,2,14])for(const placed of [0,1,13,14])for(const sent of [0,2,8])
for(const amounts of [[],[5],[1,4],[2,2,3]])for(const attacks of [[3],[7],[1,2,4]]){
  const rules={...make(true).state.rules,openerphase_pieces:limit};
  const s=A.createAttack();s.pieces=placed+1;s.cumulativeSent=sent;
  s.pending=amounts.map((amt,i)=>({cid:i+1,amt,active:true,status:'spawn',hardened:false,shielded:false}));
  const holes=createHoles(42);
  for(const attack of attacks)A.fight(s,attack,rules,holes);
  fixtures.push({kind:'cancel',limit,placed,sent,amounts,attacks,
    expected_remaining:A.pendingCount(s),expected_sent:s.cumulativeSent});
}
for(const type of ['i','o','t','l','j','s','z'])for(const clutch of [false,true])
for(const lastClear of [false,true])for(const height of [22,40])for(const garbage of [false,true]){
  const e=make(clutch);
  for(let y=40-height;y<40;y++)e.state.board.rows[y][4]='i';
  e.state.lastClear=lastClear;e.state.board.lastWasAttack=garbage;
  const board=rows(e.state);e.spawn(type);
  fixtures.push({kind:'rescue',board,piece:type.toUpperCase(),clutch,lastClear,
    expected_playing:e.state.playing,authority_reason:e.state.reason});
}
for(const clutch of [false,true]){
  const e=make(clutch);
  for(let y=17;y<40;y++)e.state.board.rows[y][4]='i';
  for(let x=4;x<10;x++)e.state.board.rows[39][x]='i';
  const board=rows(e.state);
  Object.assign(e.state.piece,{type:'i',x:1,y:39,hy:39,r:0,rotated:false,spin:'none'});
  e.state.bag.queue[0]='i';e.state.attack.combo=1;
  const rules=Object.fromEntries(['b2bcharging','b2bcharge_at','b2bcharge_base','b2bchaining','openerphase_pieces','allclears','allclear_garbage','allclear_b2b','garbagespecialbonus','clutch'].map(k=>[k,e.state.rules[k]]));
  e.step([{frame:0,type:'keydown',key:'hardDrop',subframe:0.5}]);
  fixtures.push({kind:'clear_rescue',board,rules,
    placement:{location:{type:'I',orientation:'north',x:1,y:0},spin:'none'},
    expected_board:rows(e.state),expected_playing:e.state.playing,
    expected_combo:e.state.attack.combo,expected_lines:e.state.stats.lines,
    authority_reason:e.state.reason});
}
fs.writeFileSync(process.argv[3],JSON.stringify(fixtures));
console.log(JSON.stringify({fixtures:fixtures.length,counts:Object.fromEntries(['cancel','rescue','clear_rescue'].map(k=>[k,fixtures.filter(f=>f.kind===k).length])),
  scope:'normal pending cancellation, opener threshold, spawn rescue and representative clear-to-rescue; not full ARE/bump or terminal-reason parity'}));
