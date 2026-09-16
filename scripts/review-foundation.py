from pathlib import Path
import hashlib

def edit(name, old, new, count=1):
    p=Path(name); s=p.read_text()
    assert s.count(old)==count, (name, old[:100], s.count(old))
    p.write_text(s.replace(old,new))

# Refuse to apply against a different implementation. No force-push is used.
for name, digest in {
    'src/analysis.rs':'92660e1a354428c9bf931b34b3a5a7612d86ee0cb79993598036f4f8ebcbb024',
    'src/bot.rs':'4311531ec33ced8930a20c4b13981a751db2973013ab2c0e3a56f800f908d5ba',
    'src/dag.rs':'e2de59145d368e7ff6aade98958331a85065c37dc124d092ee26f703d239de7d',
    'src/movegen.rs':'a996366b7152a70dd7b1f2efb0e372d65befaf6be9ceda7f76bed50a43a3545d',
    'scripts/reconstruct-replay.cjs':'348f1435e96a27418868312104f155637a38b4b0ffae136c222db1eb5c01e88b',
}.items():
    assert hashlib.sha256(Path(name).read_bytes()).hexdigest()==digest, name

edit('src/bot.rs','pub freestyle_exploitation: f64,','pub freestyle_exploitation: f64,\n    /// Internal search randomness, unrelated to the game randomizer.\n    #[serde(default)]\n    pub search_seed: Option<u64>,')
edit('src/bot/freestyle.rs','pub fn new(_options: &BotOptions,','pub fn new(options: &BotOptions,')
edit('src/bot/freestyle.rs','Dag::new(root, queue)','Dag::new(root, queue, options.config.search_seed)')
edit('src/dag.rs','use ouroboros::self_referencing;','use ouroboros::self_referencing;\nuse parking_lot::Mutex;\nuse rand::{rngs::StdRng, SeedableRng};')
edit('src/dag.rs','pub struct Dag<E: Evaluation> {','pub struct Dag<E: Evaluation> {\n    search_rng: Mutex<StdRng>,')
edit('src/dag.rs','pub fn new(root: GameState, queue: &[Piece]) -> Self {','pub fn new(root: GameState, queue: &[Piece], seed: Option<u64>) -> Self {')
edit('src/dag.rs','        Dag {\n            root,','        Dag {\n            search_rng: Mutex::new(seed.map(StdRng::seed_from_u64).unwrap_or_else(StdRng::from_entropy)),\n            root,')
edit('src/dag.rs','        let mut layers = vec![&*self.top_layer];','        let mut rng = self.search_rng.lock();\n        let mut layers = vec![&*self.top_layer];')
edit('src/dag.rs','layer.kind.select(&game_state, speculate, exploration)','layer.kind.select(&game_state, speculate, exploration, &mut rng)')
edit('src/dag.rs','fn select(&self, game_state: &GameState, speculate: bool, exploration: f64) -> SelectResult {','fn select(&self, game_state: &GameState, speculate: bool, exploration: f64, rng: &mut StdRng) -> SelectResult {')
edit('src/dag.rs','l.select(game_state, exploration)','l.select(game_state, exploration, rng)',2)
for name in ['src/dag/known.rs','src/dag/speculated.rs']:
    edit(name,'pub fn select(&self, game_state: &GameState, exploration: f64) -> SelectResult {','pub fn select(&self, game_state: &GameState, exploration: f64, rng: &mut StdRng) -> SelectResult {')
    edit(name,'let s: f64 = thread_rng().gen();','let s: f64 = 1.0 - rng.gen::<f64>();')
edit('src/dag/speculated.rs','thread_rng().gen_range','rng.gen_range')
edit('src/movegen.rs','    locks.extend(underground_locks.into_iter());\n    locks','''    locks.extend(underground_locks.into_iter());
    // Keep hash-map iteration out of search order and retain minimum path cost.
    locks.sort_by_key(|(mv, cost)| (mv.location.piece as u8, mv.location.rotation as u8,
        mv.location.x, mv.location.y, mv.spin as u8, *cost));
    locks.dedup_by_key(|(mv, _)| *mv);
    locks''')
edit('src/forecast.rs','    pub fn remaining(&self)->u32','''    /// Clock-free snapshot. Unconfirmed arrival times are not invented.
    /// all_ready is an alternative pressure assumption, not actual timing.
    pub fn at_snapshot(packets:&[GarbagePacket], pieces_placed:u32, sent:u32, scenario:u32, all_ready:bool) -> Result<Self,String> {
        let mut f=Self::new(packets,pieces_placed,sent,1,0,scenario)?;
        f.frames_per_piece=0;
        for (i,p) in packets.iter().enumerate() {
            f.packets[i].ready_at=if p.active || all_ready {0} else {u32::MAX};
        }
        Ok(f)
    }
    pub fn remaining(&self)->u32''')
Path('src/analysis.rs').write_text(Path('scripts/review-analysis.rs').read_text())
edit('tests/replay_fixtures.rs','"frames_per_piece":12,"pending_delay_frames":20,','"activation_model":"snapshot",')
p=Path('scripts/recorded-wasm-smoke.cjs')
s=p.read_text().replace('garbage_sent: 0, frames_per_piece: 12, pending_delay_frames: 20, iterations: 10','garbage_sent: 0, activation_model: "snapshot", iterations: 10')
p.write_text(s)

edit('scripts/reconstruct-replay.cjs',"const path = require('node:path');", "const path = require('node:path');\nconst {attachReceiveLedger} = require('./receive-ledger.cjs');")
edit('scripts/reconstruct-replay.cjs','let output=[];let allLocks=[];','let output=[];let allLocks=[];let receiveTraces=[];')
edit('scripts/reconstruct-replay.cjs','const frames=Array.from','const receiveAudit=attachReceiveLedger(e);\n  const frames=Array.from')
edit('scripts/reconstruct-replay.cjs','const actual=clone(e.stats);','''const actual=clone(e.stats);
  const receipt=receiveAudit.ledger.summary();
  // Reference receive means queue admission. Recorded v19 received means
  // remaining lines at confirmation. Preserve both without mutating gameplay.
  const referenceAdmissionCounter=actual.garbage.receive;
  actual.garbage.receive=receipt.confirmed;
  receiveTraces.push({stream:receiveTraces.length,expected_received:expected.garbage.receive,events:receiveAudit.events});''')
edit('scripts/reconstruct-replay.cjs','const out={round:','const out={receive_audit:receipt,reference_admission_counter:referenceAdmissionCounter,round:')
p=Path('scripts/reconstruct-replay.cjs')
p.write_text(p.read_text()+"\nfs.writeFileSync(path.join(outputDir,'receive-traces.json'),JSON.stringify({streams:receiveTraces}));\n")
