// Ranked CC2 placements are proposals, not authoritative Tetrp inputs.
// Reject placements that cannot be scheduled on the visible Tetrp checkpoint;
// do not silently substitute a geometric teleport or score an invalid game.
export function chooseExecutableCandidate(candidates, engine, startFrame, lockFrame, {findPath, schedulePath}) {
  if (!Array.isArray(candidates) || candidates.length === 0) {
    throw new Error('search produced no candidate');
  }
  const rejected=[];
  for (const [rank, candidate] of candidates.entries()) {
    if (!candidate?.placement) {
      rejected.push(`rank ${rank}: missing placement`);
      continue;
    }
    try {
      const path=findPath(engine,candidate.placement);
      const inputs=schedulePath(startFrame,lockFrame,path.moves,engine);
      return {placement:candidate.placement,path,inputs,rank};
    } catch (error) {
      rejected.push(`rank ${rank}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  throw new Error(`no executable Tetrp candidate (${candidates.length} searched ranks): ${rejected.slice(0,5).join('; ')}`);
}
