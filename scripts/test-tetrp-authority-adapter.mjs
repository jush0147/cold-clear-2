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
      hold:{piece:null},
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
      rules:{garbagespeed_frames:20},
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

  const observer=SevenBagObserver.fromGameStart(a.queue);
  const req=buildAnalysisRequest(a,observer,{nodeBudget:200000,framesPerPiece:30});
  assert.deepEqual(req.start.randomizer,{type:'seven_bag',bag_state:['Z']});
  assert.deepEqual(req.incoming,a.incoming);
  assert.equal(req.frames_per_piece,30);
  assert.equal(req.pending_delay_frames,20);
  assert.equal(JSON.stringify(req).includes('111'),false);
  assert.equal(JSON.stringify(req).includes('222'),false);
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
    'hold-driven visible draw advance'
  ]
},null,2));
