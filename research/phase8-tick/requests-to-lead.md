# Requests to the lead (phase8-tick agent)

1. Client integration (aa-client is yours): drive the game with
   `aa_runtime::simulation::Simulation::step(now)` once per rendered frame,
   where `now` is wall-clock seconds as binary32 (native NetNode+0x58). This
   needs `aa-runtime` as an aa-client dependency (Cargo.toml / Cargo.lock).
   Never advance the world from render delta; `Action` says what ran.

2. Network/decree owner: implement `simulation::NetworkSeams`. Decrees must be
   applied only in `host_execute_requests` (host, 0x752E60, client-list order)
   and `client_replay_history(index)` (client, 0x74FD10); replay playback in
   `replay_playback` (0x751A40, before a world tick). The real admin clock
   (`real_admin_clock`) and `avi_pump` are wall-clock seams. All seams are
   required; there are no default successes.

3. API changes in my crates that other owners should know about:
   - `aa_world::Callbacks` gained `actor_b4`, `resolve_handle`, `actor_2c`
     (default implementations return an explicit error) and
     `aa_world::Operation` gained `RequestRemoval`, `ActorB4`,
     `ResolveHandle`, `Actor2C`. Exhaustive matches on `Operation` outside
     aa-world must add arms (none found in the checkout).
   - `aa_runtime::clocked::Effect::Actor2C`, `Predicate::ClassB4`, and
     `Services::resolve_handle` (default: explicit error) were added, plus
     `clocked::request_removal(...)` for service callbacks (deaths/removals).
     Exhaustive matches on Effect/Predicate outside aa-runtime must add arms
     (current tests use wildcards).
   - `aa_numeric::Binary::to_x87_extended_bits` (new, additive).

4. Decision needed: x87 precision for the authoritative tick. Static evidence
   (NiDX9Renderer::Create 0x891890 / KKC_Renderer::ChangeResolution 0x710AC0
   never request D3DCREATE_FPU_PRESERVE) points to PC24 on the main thread, but
   a library context (0x7F1965) switches to PC53 temporarily. Oracles cover
   PC24 and PC64 and the admin/world interleaving differs between them. Please
   assign a task (or approve one) to confirm the runtime control word, e.g. by
   tracing FLDCW/_controlfp order around device creation and the Lua/library
   context lifetime.

5. Owner of decree decoding (0x74D710/0x766CE0 registry) and of the GameSpeed
   conversion 0x76B230 (calls 0x96D890): needed so admin ticks do real work and
   `Timeline::game_per_admin_tick` follows GameSpeed changes natively.

6. docs/WORLD-ORDER.txt (not mine) still says 0x4F6760 is not implemented; it
   is now `aa_world::World::request_removal`, oracle-backed by
   research/phase8-tick/native-removal-request.json. Please update that doc
   and HANDOFF.txt's index with research/phase8-tick.

## Round 2

7. Precision policy: static evidence refutes PC24 for the simulation thread
   (fpu-precision.md). The tick runs on the net TGC_Thread (CW 0x027F, PC53);
   only the main/render thread is switched to PC24 by Direct3D. Please adopt
   `Precision::Double53` for the authoritative World/Queues/Driver and
   `Single24` only for code ported from the render/UI thread, pending the
   dynamic check described in fpu-precision.md section 6 (needs the original
   game running on a test machine; I did not run it).

8. Emulator limit affecting other agents' oracles (emulator-limits.json):
   Unicorn 2.1.4 computes FPATAN/FSIN/FCOS/FYL2X/F2XM1 through binary64 and
   never raises the x87 precision flag from arithmetic. Any fixture that
   records 80-bit results of transcendental instructions, or CRT tails that
   test the sticky precision flag (e.g. 0x97792B after asin/pow), is only
   binary64-accurate. In particular the 522 "unstored ST0" captures of
   0x445400 in research/phase8-ai/native-rout-score.json differ from real
   x87 in the low bits for 122 calls (all within one binary64 ulp, binary32
   stores identical). Please forward to the AI agent; aa_trig's
   `signed_angle_extended` executes the real instructions.

9. NetworkSeams changed (for the orders/net agent): `session_tail` was
   replaced by `backup_clients()` (connection wants-backup flags, AdminNode
   +0x2AC list order) and `backup_mode_entered()` (0x757290 diagnostics +
   GameSpy report 0x791800); `AdminNode.playback` is now `backup_mode` +
   `backup_since` (+0x2D4/+0x2D8, TGC_AdminNode::SetBackupMode 0x757250) and
   `session_tail_node` is `is_server` (+0x2C8 server node). The heartbeat
   0x75B820 is network-only (backup chat/vote decrees, statistics).
   `SyncLog` gained `GameTimePerTick` and `Frozen`. `Simulation::unfreeze`
   was replaced by `Simulation::set_frozen(bool)` (0x76DD90).

10. Decree handler for KKC_SetGameSpeedOrder (0x75D810) should call
    `Timeline::set_game_speed` with SVars GameSpeedMin/Max/FilmMin/FilmMax
    and AdminTicksPerSecond, and emit `SpeedChange::sync_log`; when
    `changed`, the UI owner shows the speed message (0x76F474 branch).
    Which CRT pow body the original used (`PowPath`) is a machine property
    (SSE2 on all modern CPUs); both give the same stored binary32 in every
    oracle row.

11. Session/UI owner: 0x76F600 (no human opponents remaining message) and
    client owner: 0x41C180 (KKC_TimingStats) are presentation seams called
    by the world tick wrapper; they never mutate simulation state.

12. For the movement agent: on the native tick path the sticky precision
    flag PE is SET whenever gameplay code runs (set by the frame-start
    binary32 time store 0x76544F, never cleared on the net thread; analysis in
    fpu-precision.md section 8). Call `ground_step_original`/OriginalTurning
    (and aa_trig angle helpers) with PE = set from the tick.
