import assert from 'node:assert/strict';
import {
  SevenBagObserver,
  captureVisibleState,
  buildAnalysisRequest,
  drawsAdvancedByPlacement,
} from './lib/tetrp-authority-adapter.mjs';

const bag1=['I','O','T','L','J','S','Z'];
const bag2=['Z','S','J','L','T','O','I'];
const seq=[...bag1,...bag2];

{
  const o=SevenBagObserver.fromGameStart(seq.slice(0,6));
  assert.deepEqual(o.frontierBagState(),['Z']);

  o.advance(seq.slice(1,7),1);
  assert.deepEqual(o.frontierBagState(),['I','O','T','L','J','S','Z']);

  o.advance(seq.slice(2,8),1);
  assert.deepEqual(o.frontierBagState(),['I','O','T','L','J','S']);

  // First empty hold consumes one extra bag draw before the placement locks.
  o.advance(seq.slice(4,10),2);
  assert.deepEqual(o.frontierBagState(),['I','O','T','L']);
  assert.equal(o.snapshot().observed_draws,10);
}

{
  const o=SevenBagObserver.fromGameStart(seq.slice(0,6));
  assert.throws(()=>o.advance(seq.slice(2,8),1),/did not advance/);
}

function fakeEngine(hiddenTail, rngSeed, holeSeed) {
  return {
    state:{
      phase:'ready',
      frame:100,
      piece:{type:'i'},
      bag:{queue:['o','t','l','j','s',...hiddenTail],rng:{seed:rngSeed}},
      holes:{rng:{seed:holeSeed}},
      hold:{piece:null,locked:false},
      board:{rows:Array.from({length:40},()=>Array(10).fill(null))},
      attack:{
        combo:2,
        btb:5,
        cumulativeSent:17,
        multiplier:1,
        are:[{amt:2,column:7}],
        pending:[
          {amt:3,active:false,activeFrame:112,hardened:false,shielded:false,status:'spawn'},
          {amt:4,active:true,activeFrame:95,hardened:false,shielded:false,status:'spawn'},
        ],
      },
      stats:{pieces:23},
      rules:{
        garbagespeed_frames:20,
        garbagemargin_frames:10800,
        garbageincrease_per_second:0.008,
        b2bcharging:true,
        b2bcharge_at:4,
        b2bcharge_base:3,
        b2bchaining:false,
        openerphase_pieces:14,
        allclears:true,
        allclear_garbage:5,
        allclear_b2b:1,
        garbagespecialbonus:true,
        clutch:true,
      },
    }
  };
}

{
  const a=captureVisibleState(fakeEngine(['z','i','o'],111,222));
  const b=captureVisibleState(fakeEngine(['l','j','s'],999,888));
  assert.deepEqual(a,b,'hidden NEXT6+, bag RNG and hole RNG must not affect visible capture');
  assert.deepEqual(a.queue,['I','O','T','L','J','S']);
  assert.deepEqual(a.incoming,[
    {lines:2,ready_in_frames:0},
    {lines:3,ready_in_frames:12},
    {lines:4,ready_in_frames:0},
  ]);
  assert.equal(a.b2b_count,4);
  assert.equal(a.hold_locked,false);
  assert.equal(a.rules.b2bcharge_base,3);
  assert.equal(a.rules.openerphase_pieces,14);

  const observer=SevenBagObserver.fromGameStart(a.queue);
  const req=buildAnalysisRequest(a,observer,{nodeBudget:200000,framesPerPiece:30});
  assert.deepEqual(req.start.randomizer,{type:'seven_bag',bag_state:['Z']});
  assert.deepEqual(req.incoming,a.incoming);
  assert.equal(req.frames_per_piece,30);
  assert.equal(req.pending_delay_frames,20);
  assert.equal(req.authority_frame,100);
  assert.equal(req.garbage_multiplier,1);
  assert.equal(req.garbage_margin_frames,10800);
  assert.equal(req.garbage_increase_per_second,0.008);
  assert.equal(req.rules.b2bcharge_base,3);
  assert.equal(req.rules.b2bcharge_at,4);
  assert.equal(req.rules.openerphase_pieces,14);
  assert.equal(req.hold_locked,false);
  assert.equal(JSON.stringify(req).includes('111'),false);
  assert.equal(JSON.stringify(req).includes('222'),false);
}

{
  const e=fakeEngine(['z'],123,456);
  e.state.hold={piece:'z',locked:true};
  const locked=captureVisibleState(e);
  assert.equal(locked.hold,'Z');
  assert.equal(locked.hold_locked,true);
  const observer=SevenBagObserver.fromGameStart(locked.queue);
  const req=buildAnalysisRequest(locked,observer,{nodeBudget:5000,framesPerPiece:24});
  assert.equal(req.hold_locked,true);
}

{
  const visible={queue:['I','O','T','L','J','S'],hold:null};
  const current={location:{type:'I'}};
  const held={location:{type:'O'}};
  assert.equal(drawsAdvancedByPlacement(visible,current),1);
  assert.equal(drawsAdvancedByPlacement(visible,held),2);

  const withHold={...visible,hold:'T'};
  assert.equal(drawsAdvancedByPlacement(withHold,held),1);
}

console.log(JSON.stringify({
  ok:true,
  checks:[
    'seven-bag boundary tracking',
    'empty-hold two-draw tracking',
    'hidden-state noninterference',
    'per-packet remaining garbage timing',
    'authority attack scaling clock',
    'public TL rule transport including surge base',
    'root Hold lock transport',
    'hold-driven visible draw advance'
  ]
},null,2));
