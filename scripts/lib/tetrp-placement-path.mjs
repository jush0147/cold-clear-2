import { upperPiece } from './tetrp-authority-adapter.mjs';

export function createPlacementTools({Engine, boardModule:B, rotationModule:R}) {
  const baseCells = {
    I:[[-1,0],[0,0],[1,0],[2,0]],
    O:[[0,0],[1,0],[0,1],[1,1]],
    T:[[-1,0],[0,0],[1,0],[0,1]],
    L:[[-1,0],[0,0],[1,0],[1,1]],
    J:[[-1,0],[0,0],[1,0],[-1,1]],
    S:[[-1,0],[0,0],[0,1],[1,1]],
    Z:[[-1,1],[0,1],[0,0],[1,0]],
  };

  function rotateCell(c, orientation) {
    const x=c[0], y=c[1];
    if (orientation === 'north') return [x,y];
    if (orientation === 'east') return [y,-x];
    if (orientation === 'south') return [-x,-y];
    if (orientation === 'west') return [-y,x];
    throw new Error('unknown CC2 rotation '+orientation);
  }

  function targetFor(placement) {
    const l=placement.location;
    const cells=baseCells[l.type].map(c=>rotateCell(c,l.orientation))
      .map(c=>[l.x+c[0],39-(l.y+c[1])]);
    return {cells,spin:placement.spin,type:l.type.toLowerCase()};
  }

  const cellsKey=cells=>cells.map(c=>c[0]+','+Math.ceil(c[1])).sort().join(';');
  const copyPiece=p=>structuredClone(p);

  function dropped(board,piece) {
    const p=copyPiece(piece);
    while(B.legal(board,{...p,y:p.y+1})) p.y+=1;
    return p;
  }

  function pathStateKey(p,ruleset) {
    // totalRotations changes SRS+ kick Y after lockresets+15. Once beyond
    // that threshold, exact larger values are geometrically equivalent.
    const rotationBucket=Math.min(p.totalRotations||0,(ruleset.lockresets||15)+16);
    return [p.x,Number(p.y).toFixed(6),p.r,p.kick,p.rotated?1:0,p.spin,rotationBucket].join(',');
  }

  function applyPathMove(board,piece,action,ruleset) {
    let p=copyPiece(piece);
    if(action==='moveLeft'||action==='moveRight') {
      const x=p.x+(action==='moveLeft'?-1:1);
      if(!B.legal(board,{...p,x})) return null;
      p.x=x;p.rotated=false;p.spin='none';p.wall=false;p.resets=(p.resets||0)+1;p.locking=0;
      return p;
    }
    if(action==='down') {
      const q={...p,y:p.y+1};
      if(!B.legal(board,q)) return null;
      p=q;p.rotated=false;p.spin='none';
      return p;
    }
    const dir=action==='rotateCW'?1:action==='rotateCCW'?3:2;
    if(dir===2&&!ruleset.allow180) return null;
    const q=R.rotate(board,p,dir,ruleset.lockresets);
    if(!q) return null;
    p={...p,...q,rotated:true,totalRotations:(p.totalRotations||0)+1,
      rotationResets:Math.min(63,(p.rotationResets||0)+1),
      resets:(p.resets||0)+1,locking:0};
    p.spin=R.classifySpin(board,p,ruleset.spinbonuses);
    return p;
  }


  const orientationNames=['north','east','south','west'];
  function ccPlacementsForAuthorityCells(type,cells,spin) {
    const upper=upperPiece(type),actual=cellsKey(cells),out=[];
    for(const orientation of orientationNames) {
      const offsets=baseCells[upper].map(c=>rotateCell(c,orientation));
      for(const [ax,ay] of cells) for(const [dx,dy] of offsets) {
        const x=ax-dx,y=39-ay-dy;
        if(!Number.isInteger(x)||!Number.isInteger(y))continue;
        const placement={location:{type:upper,orientation,x,y},spin};
        if(cellsKey(targetFor(placement).cells)===actual)out.push(placement);
      }
    }
    const seen=new Set();
    return out.filter(p=>{const k=JSON.stringify(p);if(seen.has(k))return false;seen.add(k);return true;});
  }

  /**
   * Exhaustive geometry-only root landing enumeration from the ACTUAL active
   * piece state. No Hold, no frame clock and no lock-reset auto-lock timing.
   * The result is an allowlist for Kiwi's first search layer; scheduling remains
   * a separate authority validation step.
   */
  function enumerateRootPlacements(engine) {
    const state=engine.state;
    if(!state.playing||!state.piece||state.piece.sleeping)
      throw new Error('ROOT_GEOMETRY_NOT_PLAYABLE');
    // Deliberately clone ONLY current geometry inputs. Do not serialize the
    // Engine: that would read hidden queue/RNG even though geometry needs neither.
    const board=structuredClone(state.board),rules=structuredClone(state.rules);
    const q=[copyPiece(state.piece)],seen=new Set(),placements=new Map();
    const actions=['moveLeft','moveRight','rotateCW','rotateCCW','rotate180','down'];
    let head=0;
    while(head<q.length) {
      if(q.length>250000)throw new Error('ROOT_GEOMETRY_STATE_LIMIT');
      const p=q[head++],key=pathStateKey(p,rules);
      if(seen.has(key))continue;
      seen.add(key);
      const drop=dropped(board,p);
      const spin=p.rotated?R.classifySpin(board,p,rules.spinbonuses):'none';
      for(const placement of ccPlacementsForAuthorityCells(p.type,B.cells(drop),spin)) {
        placements.set(JSON.stringify(placement),placement);
      }
      for(const action of actions) {
        const next=applyPathMove(board,p,action,rules);
        if(next)q.push(next);
      }
    }
    const list=[...placements.values()].sort((a,b)=>
      a.location.type.localeCompare(b.location.type)||
      a.location.x-b.location.x||a.location.y-b.location.y||
      orientationNames.indexOf(a.location.orientation)-orientationNames.indexOf(b.location.orientation)||
      a.spin.localeCompare(b.spin)
    );
    return {
      placements:list,
      states_explored:seen.size,
      semantics:'geometry_only_no_hold_no_frame_clock',
      timing_validated:false,
    };
  }


  function findCurrentPath(state,placement) {
    const target=targetFor(placement);
    if(upperPiece(state.piece.type)!==placement.location.type)
      throw new Error('current-piece mismatch');
    const board=structuredClone(state.board),rules=structuredClone(state.rules);
    const targetKey=cellsKey(target.cells);
    const q=[{piece:copyPiece(state.piece),moves:[]}],seen=new Set();
    const actions=['moveLeft','moveRight','rotateCW','rotateCCW','rotate180','down'];
    let head=0;
    while(head<q.length) {
      if(q.length>250000)throw new Error('ROOT_GEOMETRY_STATE_LIMIT');
      const node=q[head++],p=node.piece,key=pathStateKey(p,rules);
      if(seen.has(key))continue;
      seen.add(key);
      const drop=dropped(board,p);
      const spin=p.rotated?R.classifySpin(board,p,rules.spinbonuses):'none';
      if(cellsKey(B.cells(drop))===targetKey&&spin===target.spin)
        return {useHold:false,moves:[...node.moves,'hardDrop'],target,geometryStates:seen.size};
      for(const action of actions) {
        const next=applyPathMove(board,p,action,rules);
        if(next)q.push({piece:next,moves:[...node.moves,action]});
      }
    }
    throw new Error('no current-pose Tetrp geometry path for '+JSON.stringify({placement,target,current:state.piece}));
  }

  function findPath(engine,placement) {
    const target=targetFor(placement);
    const root=Engine.restore(engine.serialize());
    const useHold=upperPiece(root.state.piece.type)!==placement.location.type;
    const prefix=[];
    if(useHold) {
      if(!root.hold()) throw new Error('Bot requested hold but authority could not hold');
      prefix.push('hold');
    }
    if(root.state.piece.type!==target.type) {
      throw new Error('piece mismatch after hold: authority='+root.state.piece.type+' target='+target.type);
    }
    const targetKey=cellsKey(target.cells),board=root.state.board;
    const q=[{piece:copyPiece(root.state.piece),moves:[]}];
    const seen=new Set();
    const actions=['moveLeft','moveRight','rotateCW','rotateCCW','rotate180','down'];
    let head=0;
    while(head<q.length&&head<100000) {
      const node=q[head++],p=node.piece,key=pathStateKey(p,root.state.rules);
      if(seen.has(key)) continue;
      seen.add(key);
      const drop=dropped(board,p);
      const spin=p.rotated?R.classifySpin(board,p,root.state.rules.spinbonuses):'none';
      if(cellsKey(B.cells(drop))===targetKey&&spin===target.spin) {
        return {useHold,moves:[...prefix,...node.moves,'hardDrop'],target};
      }
      if(node.moves.length>=32) continue;
      for(const action of actions) {
        const next=applyPathMove(board,p,action,root.state.rules);
        if(next) q.push({piece:next,moves:[...node.moves,action]});
      }
    }
    throw new Error('no Tetrp input path for '+JSON.stringify({
      placement,target,current:root.state.piece,
      boardTop:board.rows.findIndex(r=>r.some(Boolean))
    }));
  }

  function legacySchedulePath(startFrame,lockFrame,moves) {
    const inputs=[];
    let frame=startFrame,slot=0;
    const subframes=[0,0.2,0.4,0.6,0.8];
    const tap=key=>{
      if(slot>=subframes.length){frame++;slot=0;}
      const down=subframes[slot++];
      const up=Number((down+0.1).toFixed(1));
      inputs.push({frame,type:'keydown',key,subframe:down});
      inputs.push({frame,type:'keyup',key,subframe:up});
    };
    for(const move of moves) {
      if(move==='hardDrop') continue;
      tap(move==='down'?'softDrop':move);
    }
    if(frame>=lockFrame) {
      throw new Error('path needs too much synthetic time: start='+startFrame+' pathFrame='+frame+' lock='+lockFrame);
    }
    inputs.push({frame:lockFrame,type:'keydown',key:'hardDrop',subframe:0.5});
    inputs.push({frame:lockFrame,type:'keyup',key:'hardDrop',subframe:0.6});
    return inputs;
  }

  function compactSchedulePath(startFrame,lockFrame,moves) {
    const inputs=[];
    const pathMoves=moves.filter(move=>move!=='hardDrop');
    const softDrops=pathMoves.filter(move=>move==='down').length;

    // Fallback transport for reset-heavy placements. Horizontal, rotation and
    // hold taps consume no authority time; only soft drop needs a positive
    // segment because Tetrp advances it through fall(). Keeping grounded taps
    // at one subframe prevents synthetic tap spacing from tripping the 15-reset
    // auto-lock before the intended hard drop.
    const lockTick=lockFrame*10+5;
    let tick=lockTick-softDrops;
    const firstTick=startFrame*10;
    if(tick<firstTick) {
      throw new Error('path needs too much compact synthetic time: start='+startFrame+' firstTick='+tick+' lock='+lockFrame);
    }
    const emit=(type,key,at)=>{
      const frame=Math.floor(at/10);
      const subframe=Number(((at%10)/10).toFixed(1));
      inputs.push({frame,type,key,subframe});
    };

    for(const move of pathMoves) {
      const key=move==='down'?'softDrop':move;
      emit('keydown',key,tick);
      if(move==='down') tick++;
      emit('keyup',key,tick);
    }
    if(tick!==lockTick) throw new Error('internal compact placement transport tick drift');
    emit('keydown','hardDrop',lockTick);
    emit('keyup','hardDrop',lockTick);
    return inputs;
  }

  function scheduleLocksAtAuthorityInstant(engine,inputs,startFrame,lockFrame) {
    const probe=Engine.restore(engine.serialize());
    const beforePieces=probe.state.stats.pieces;
    for(let frame=startFrame;frame<=lockFrame;frame++) {
      probe.step(inputsForFrame(inputs,frame));
    }
    const locks=probe.trace.filter(event=>event.type==='lock');
    if(locks.length===0) {
      // A genuine authority death before the planned placement is not a
      // transport failure; the scored match will end for the same reason.
      return !probe.state.playing && probe.state.stats.pieces===beforePieces;
    }
    if(locks.length!==1 || probe.state.stats.pieces!==beforePieces+1) return false;
    const lock=locks[0];
    return lock.frame===lockFrame && Math.abs(lock.subframe-0.5)<1e-9;
  }

  function schedulePath(startFrame,lockFrame,moves,engine=null) {
    // Preserve the established transport exactly for ordinary placements so
    // completed experiment games remain comparable. Only fall back to compact
    // equal-subframe transport when the pinned Tetrp authority proves that the
    // ordinary spacing would auto-lock early or lock more than one piece.
    const ordinary=legacySchedulePath(startFrame,lockFrame,moves);
    if(engine===null || scheduleLocksAtAuthorityInstant(engine,ordinary,startFrame,lockFrame)) {
      return ordinary;
    }
    const compact=compactSchedulePath(startFrame,lockFrame,moves);
    if(!scheduleLocksAtAuthorityInstant(engine,compact,startFrame,lockFrame)) {
      throw new Error('no reset-safe Tetrp transport for placement path');
    }
    return compact;
  }

  function inputsForFrame(inputs,frame) {
    return inputs.filter(e=>e.frame===frame).map(e=>({
      frame:e.frame,type:e.type,key:e.key,subframe:e.subframe
    }));
  }

  return {findPath,findCurrentPath,enumerateRootPlacements,schedulePath,inputsForFrame,targetFor};
}
