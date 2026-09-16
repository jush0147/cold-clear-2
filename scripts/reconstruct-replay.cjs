const fs = require('node:fs');
const path = require('node:path');
if (!process.argv[2] || !process.argv[3]) {
 console.error('Usage: node scripts/reconstruct-replay.cjs replay.ttrm /path/to/triangle.cjs [output-dir]'); process.exit(2);
}
const { Engine } = require(path.resolve(process.argv[3]));
const outputDir = path.resolve(process.argv[4] || 'replay-audit'); fs.mkdirSync(outputDir, {recursive:true});
const replay = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
// The seed belongs exclusively to the offline replay decoder, NEVER the bot observation.
function config(o, opponents, date) {
 return {
  board: { width:o.boardwidth, height:o.boardheight, buffer:o.boardbuffer },
  kickTable:o.kickset,
  options:{ comboTable:o.combotable, garbageBlocking:o.garbageblocking, clutch:o.clutch, garbageTargetBonus:o.garbagetargetbonus, spinBonuses:o.spinbonuses, stock:0 },
  queue:{ minLength:14, seed:o.seed, type:o.bagtype },
  garbage:{ bombs:o.usebombs, cap:{ absolute:o.garbageabsolutecap, increase:o.garbagecapincrease, max:o.garbagecapmax, value:o.garbagecap, marginTime:o.garbagecapmargin }, boardWidth:o.boardwidth,
   garbage:{speed:o.garbagespeed, holeSize:o.garbageholesize}, messiness:{change:o.messiness_change,nosame:o.messiness_nosame,timeout:o.messiness_timeout,within:o.messiness_inner,center:o.messiness_center},
   multiplier:{value:o.garbagemultiplier,increase:o.garbageincrease,marginTime:o.garbagemargin}, specialBonus:o.garbagespecialbonus, openerPhase:o.openerphase, seed:o.seed, rounding:o.roundmode },
  gravity:{ value:o.g, increase:o.gincrease, marginTime:o.gmargin }, handling:o.handling,
  b2b:{chaining:o.b2bchaining, charging:o.b2bcharging?{at:o.b2bcharge_at,base:o.b2bcharge_base}:false},
  pc:o.allclears?{b2b:o.allclear_b2b,garbage:o.allclear_garbage}:false,
  misc:{allowed:{hardDrop:o.allow_harddrop,spin180:o.allow180,hold:o.display_hold,retry:false,undo:false},infiniteHold:o.infinite_hold,movement:{infinite:o.infinite_movement,lockResets:o.lockresets,lockTime:o.locktime,may20G:o.gravitymay20g},stride:o.stride,date:new Date(date)},
  multiplayer:{opponents,passthrough:o.passthrough}
 };
}
const clone = v => JSON.parse(JSON.stringify(v));
let output=[];let allLocks=[];
for (let r=0;r<replay.replay.rounds.length;r++) {
 const round=replay.replay.rounds[r];
 const ends=round.map(p=>p.replay.events.find(e=>e.type==='end').data);
 for(let p=0;p<round.length;p++) {
  const source=round[p].replay;const end=ends[p];const o=end.options;
  const e=new Engine(config(o,ends.filter((_,i)=>i!==p).map(v=>v.options.gameid),replay.ts));
  const frames=Array.from({length:source.frames+2},()=>[]);
  for(const f of source.events) if(['keydown','keyup','ige'].includes(f.type)) frames[f.frame].push(f);
  const locks=[];let pre;
  const add=e.board.add.bind(e.board);
  e.board.add=(...args)=>{pre={frame:e.frame,subframe:e.subframe,board:clone(e.board.state),cells:clone(e.falling.absoluteBlocks),falling:clone(e.falling.snapshot()),stats:clone(e.stats),lastSpin:e.lastSpin,hold:e.held,next:Array.from(e.queue).slice(0,5),garbage:clone(e.garbageQueue.queue)};return add(...args);};
  e.events.on('falling.lock',(res)=>{locks.push({round:r+1,player:p,pre,lock:clone(res),after:clone(e.board.state)});});
  let error=null;
  try {while(e.frame<=source.frames){e.tick(frames[e.frame]||[]);}}catch(err){error=String(err.stack);}
  const expected={pieces:end.stats.piecesplaced,lines:end.stats.lines,combo:end.stats.combo-1,b2b:end.stats.btb-1,garbage:{attack:end.stats.garbage.attack,sent:end.stats.garbage.sent,receive:end.stats.garbage.received,cleared:end.stats.garbage.cleared}};
  const actual=clone(e.stats);
  const expectedBoard=end.game.board.slice().reverse();
  const occ=b=>b.map(row=>row.map(v=>v?1:0));
  const boardExact=JSON.stringify(e.board.state.map(row=>row.map(v=>v?.mino??null)))===JSON.stringify(expectedBoard);
  const boardOccupancy=JSON.stringify(occ(e.board.state))===JSON.stringify(occ(expectedBoard));
  const differences={};
  for(const k of ['pieces','lines','combo','b2b']) if(actual[k]!==expected[k]) differences[k]=[expected[k],actual[k]];
  for(const k of Object.keys(expected.garbage)) if(actual.garbage[k]!==expected.garbage[k]) differences['garbage.'+k]=[expected.garbage[k],actual.garbage[k]];
  const out={round:r+1,player:p,frames:source.frames,locks:locks.length,boardExact,boardOccupancy,differences,error};
  console.log(JSON.stringify(out));output.push(out);allLocks.push(...locks);
 }
}
fs.writeFileSync(path.join(outputDir,'replay-validation.json'),JSON.stringify({reference:'halp1/triangle@7837ee5bde8de2719472e0de3458abe6708593f5',streams:output},null,2));
fs.writeFileSync(path.join(outputDir,'replay-locks.json'),JSON.stringify(allLocks));
