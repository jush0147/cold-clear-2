const PIECES = Object.freeze(['I','O','T','L','J','S','Z']);

const eq = (a,b) => a.length === b.length && a.every((v,i) => v === b[i]);

export function upperPiece(piece) {
  return piece == null ? null : String(piece).toUpperCase();
}

export function visibleBagSix(state) {
  const queue = [upperPiece(state.piece?.type), ...state.bag.queue.slice(0,5).map(upperPiece)];
  if (queue.length !== 6 || queue.some(p => !PIECES.includes(p))) {
    throw new Error('authority must expose current + exactly NEXT5');
  }
  return queue;
}

export class SevenBagObserver {
  constructor(observedDraws, visibleSix) {
    this.observed = observedDraws.map(upperPiece);
    this.visible = visibleSix.map(upperPiece);
    if (this.visible.length !== 6 || this.observed.length < 6) {
      throw new Error('SevenBag observer needs visible current + NEXT5 history');
    }
    if (!eq(this.observed.slice(-6), this.visible)) {
      throw new Error('visible window must be the suffix of observed bag draws');
    }
    this.#validateObservedBags();
  }

  static fromGameStart(visibleSix) {
    const visible = visibleSix.map(upperPiece);
    if (visible.length !== 6) throw new Error('initial visible window must contain six pieces');
    if (new Set(visible).size !== 6) {
      throw new Error('initial current + NEXT5 must be six distinct pieces under 7-bag');
    }
    return new SevenBagObserver(visible, visible);
  }

  #validateObservedBags() {
    for (let start=0; start<this.observed.length; start+=7) {
      const chunk=this.observed.slice(start,Math.min(start+7,this.observed.length));
      if (new Set(chunk).size !== chunk.length) {
        throw new Error('observed draw history violates 7-bag uniqueness');
      }
    }
  }

  frontierBagState() {
    const used=this.observed.length % 7;
    if (used === 0) return [...PIECES];
    const seen=new Set(this.observed.slice(this.observed.length-used));
    return PIECES.filter(p => !seen.has(p));
  }

  advance(postVisibleSix, drawsAdvanced) {
    const post=postVisibleSix.map(upperPiece);
    if (![1,2].includes(drawsAdvanced)) {
      throw new Error('visible window may advance by only one draw, or two after first empty hold');
    }
    if (post.length !== 6) throw new Error('post-move visible window must contain six pieces');
    const overlap=6-drawsAdvanced;
    const expected=this.visible.slice(drawsAdvanced);
    const actual=post.slice(0,overlap);
    if (!eq(expected,actual)) {
      throw new Error('visible 7-bag window did not advance by the declared draw count');
    }
    this.observed.push(...post.slice(overlap));
    this.visible=post;
    this.#validateObservedBags();
  }

  snapshot() {
    return {
      observed_draws:this.observed.length,
      visible:[...this.visible],
      frontier_bag_state:this.frontierBagState(),
    };
  }
}

function ccBoard(state) {
  return [...state.board.rows].reverse().map(row => row.map(cell => {
    if (cell == null) return null;
    if (cell === 'gb' || cell === 'gbd') return 'G';
    return upperPiece(cell);
  }));
}

function observableIncoming(state) {
  const packets=[];
  for (const p of state.attack.are) {
    if (p.amt > 0) packets.push({lines:p.amt,ready_in_frames:0});
  }
  for (const p of state.attack.pending) {
    if (p.amt <= 0) continue;
    if (p.hardened || p.shielded || p.status !== 'spawn') {
      throw new Error('unsupported observable garbage packet state');
    }
    if (p.active) {
      packets.push({lines:p.amt,ready_in_frames:0});
      continue;
    }
    if (!Number.isInteger(p.activeFrame)) {
      throw new Error('inactive observable packet needs a confirmed activation frame');
    }
    const ready=p.activeFrame-state.frame;
    if (ready <= 0) {
      throw new Error('inactive packet activation frame is not in the future');
    }
    packets.push({lines:p.amt,ready_in_frames:ready});
  }
  return packets;
}

/// Capture only information available to this player. Hidden queue tail, bag RNG,
/// garbage-hole RNG, opponent board and future opponent attack are intentionally
/// absent from the returned object.
export function captureVisibleState(engine) {
  const s=engine.state;
  if (s.phase !== 'ready') throw new Error('authority snapshot must be taken at a ready-frame boundary');
  const queue=visibleBagSix(s);
  return {
    frame:s.frame,
    board:ccBoard(s),
    queue,
    hold:upperPiece(s.hold.piece),
    combo:s.attack.combo,
    back_to_back:s.attack.btb > 0,
    // CC2's root convention is x0 on the first difficult clear.
    b2b_count:Math.max(0,s.attack.btb-1),
    hold_locked:Boolean(s.hold.locked),
    rules:{
      b2bcharging:s.rules.b2bcharging,
      b2bcharge_at:s.rules.b2bcharge_at,
      b2bcharge_base:s.rules.b2bcharge_base,
      b2bchaining:s.rules.b2bchaining,
      openerphase_pieces:s.rules.openerphase_pieces,
      allclears:s.rules.allclears,
      allclear_garbage:s.rules.allclear_garbage,
      allclear_b2b:s.rules.allclear_b2b,
      garbagespecialbonus:s.rules.garbagespecialbonus,
      clutch:s.rules.clutch,
    },
    incoming:observableIncoming(s),
    pieces_placed:s.stats.pieces,
    garbage_sent:s.attack.cumulativeSent,
    garbage_speed_frames:s.rules.garbagespeed_frames,
    authority_attack_multiplier:s.attack.multiplier,
    garbage_margin_frames:s.rules.garbagemargin_frames,
    garbage_increase_per_second:s.rules.garbageincrease_per_second,
  };
}

export function buildAnalysisRequest(visible, bagObserver, {nodeBudget,framesPerPiece}) {
  if (!Number.isInteger(nodeBudget) || nodeBudget < 1000) throw new Error('invalid hard node budget');
  if (!Number.isInteger(framesPerPiece) || framesPerPiece < 1) throw new Error('invalid frames-per-piece pace');
  const bag=bagObserver.snapshot();
  if (!eq(bag.visible,visible.queue)) {
    throw new Error('SevenBag observer is not aligned with the authority visible window');
  }
  return {
    start:{
      board:visible.board,
      queue:[...visible.queue],
      hold:visible.hold,
      combo:visible.combo,
      back_to_back:visible.back_to_back,
      b2b_count:visible.b2b_count,
      randomizer:{type:'seven_bag',bag_state:bag.frontier_bag_state},
    },
    rules:{...visible.rules},
    hold_locked:Boolean(visible.hold_locked),
    incoming:visible.incoming.map(p=>({...p})),
    pieces_placed:visible.pieces_placed,
    garbage_sent:visible.garbage_sent,
    frames_per_piece:framesPerPiece,
    pending_delay_frames:visible.garbage_speed_frames,
    authority_frame:visible.frame,
    garbage_multiplier:visible.authority_attack_multiplier,
    garbage_margin_frames:visible.garbage_margin_frames,
    garbage_increase_per_second:visible.garbage_increase_per_second,
    node_budget:nodeBudget,
  };
}

export function inferredUseHold(visible, placement) {
  // The current CC2 search does not model a distinct same-piece hold branch.
  // Therefore every represented hold choice changes the played piece type.
  return upperPiece(placement.location.type) !== visible.queue[0];
}

export function drawsAdvancedByPlacement(visible, placement) {
  const useHold=inferredUseHold(visible,placement);
  return useHold && visible.hold == null ? 2 : 1;
}

export function visibleFingerprint(visible) {
  return JSON.stringify({
    frame:visible.frame,
    board:visible.board,
    queue:visible.queue,
    hold:visible.hold,
    combo:visible.combo,
    back_to_back:visible.back_to_back,
    b2b_count:visible.b2b_count,
    hold_locked:visible.hold_locked,
    rules:visible.rules,
    incoming:visible.incoming,
    pieces_placed:visible.pieces_placed,
    garbage_sent:visible.garbage_sent,
    authority_attack_multiplier:visible.authority_attack_multiplier,
    garbage_margin_frames:visible.garbage_margin_frames,
    garbage_increase_per_second:visible.garbage_increase_per_second,
  });
}
