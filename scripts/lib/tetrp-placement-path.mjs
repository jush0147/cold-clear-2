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

  function pathStateKey(p) {
    return [p.x,Number(p.y).toFixed(6),p.r,p.kick,p.rotated?1:0,p.spin].join(',');
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
      const node=q[head++],p=node.piece,key=pathStateKey(p);
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

  function schedulePath(startFrame,lockFrame,moves) {
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
      // With g=0 and SDF=20, a held soft-drop segment advances exactly one
      // row. Tetrp processes subframe events in insertion order, so down can be
      // transported as a normal tap instead of wasting one whole source frame.
      // Placement transport must not impose an artificial PPS-dependent reachability limit.
      tap(move==='down'?'softDrop':move);
    }
    if(frame>=lockFrame) {
      throw new Error('path needs too much synthetic time: start='+startFrame+' pathFrame='+frame+' lock='+lockFrame);
    }
    inputs.push({frame:lockFrame,type:'keydown',key:'hardDrop',subframe:0.5});
    inputs.push({frame:lockFrame,type:'keyup',key:'hardDrop',subframe:0.6});
    return inputs;
  }

  function inputsForFrame(inputs,frame) {
    return inputs.filter(e=>e.frame===frame).map(e=>({
      frame:e.frame,type:e.type,key:e.key,subframe:e.subframe
    }));
  }

  return {findPath,schedulePath,inputsForFrame,targetFor};
}
