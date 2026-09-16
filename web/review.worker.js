// Serve this directory next to pkg/. Use only for offline replay analysis.
import init, { ReviewSession } from '../pkg/cold_clear_2.js';
const ready = init();
let generation = 0;
let current = null;
let activeId = null;
let timer = null;
function dispose() {
  if (timer !== null) clearTimeout(timer);
  timer = null;
  if (current) current.free();
  current = null;
}
function fail(id, error) {
  self.postMessage({type: 'error', id, error: String(error?.message ?? error)});
}
function pump(token) {
  if (token !== generation || !current) return;
  try {
    const done = current.step();
    const report = JSON.parse(current.report_json());
    self.postMessage({type: done ? 'complete' : 'progress', id: activeId, report});
    // Returning to the event loop makes seek/cancel messages actionable.
    if (!done) timer = setTimeout(() => pump(token), 0);
  } catch (error) { fail(activeId, error); dispose(); }
}
self.onmessage = async ({data}) => {
  if (!data || typeof data !== 'object') return;
  const {type, id} = data;
  if (type === 'cancel') {
    // A stale cancellation must not cancel a newer review.
    if (id === activeId) {generation++; dispose(); activeId = null;}
    return;
  }
  if (type === 'details') {
    if (id !== activeId || !current) return;
    try {self.postMessage({type:'details', id, action_id:data.action_id,
      details:JSON.parse(current.candidate_json(data.action_id))});}
    catch(error) {fail(id,error);}
    return;
  }
  if (type !== 'review') return;
  const token = ++generation;
  dispose(); activeId = id;
  try {
    await ready;
    if (token !== generation) return;
    current = new ReviewSession(JSON.stringify(data.request));
    timer = setTimeout(() => pump(token), 0);
  } catch (error) {
    if (token === generation) {fail(id,error); dispose();}
  }
};
