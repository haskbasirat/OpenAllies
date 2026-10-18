PHASE 8 - NATIVE GAME LOOP, ADMIN/WORLD TICK SCHEDULER (AA.exe, base 0x400000)

Original SHA256 5d2548a2...0d26; every executed range is byte-identical in the
accepted v14 executable 3bda77d7...c5c1 (checked by the oracle scripts).

FILES
  native_tick_driver.py     Unicorn oracle: original 0x75CA70 frame driver,
                            0x755290 admin tick, 0x76B780/0x76AF90/0x76AF30/
                            0x76DCE0 timeline, 0x76F790 + the 0x4F9970 clock
                            store bytes. -> native-tick-driver.json
                            (31 driver cases, 776 frames incl. PC53 and the
                            server backup-mode cycle; 180 pure timeline rows
                            at PC24/PC53/PC64 with 80-bit x87 results).
  native_tick_round2.py     Unicorn oracle: 0x76F370 SetGameSpeed, 0x76B230,
                            CRT _CIpow 0x96D890 (SSE2 0x977A10 and x87
                            0x96D8ED bodies), 0x74B180, 0x76DD90 SetFrozen
                            -> native-tick-round2.json (3132 pow rows, 96
                            speed rows, 180 clock-estimate rows, 48 SetFrozen
                            rows).
  native_angle_extended.py  Unicorn oracle: unstored ST0 of 0x445400
                            -> native-angle-extended.json (2400 rows).
  fpu_scan.py               Static census of control-word changes, threads
                            and CreateDevice -> fpu-sites.json.
  fpu-precision.md          x87 precision decision (evidence, table, dynamic
                            check procedure).
  emulator_limits.py        Unicorn x87 fidelity probe -> emulator-limits.json.
  native_removal_request.py Unicorn oracle: original 0x4F6760 removal request,
                            0x542A00, 0x4F53D0, 0x4F5490, 0x4F5370, 0x4F4C40,
                            0x4F1430, 0x50C1B0, 0x4F65B0 -> native-removal-request.json
                            (11 cases).
  rng_sites.py              Static (heuristic) census of RNG draw sites
                            -> rng-sites.json.
  requests-to-lead.md       Changes needed outside this agent's files.
  validation-*.log          Test/check logs of this round.
Rust:
  crates/aa-runtime/src/simulation.rs   scheduler (Driver, Timeline, Simulation)
  crates/aa-runtime/src/simulation/speed.rs  GameSpeed 0x76F370/0x76B230/_CIpow
  crates/aa-trig/src/angle.rs           + signed_angle_extended (0x445400 ST0)
  crates/aa-runtime/src/clocked.rs      + request_removal helper, B4/2C/handle leaves
  crates/aa-world/src/lib.rs            + World::request_removal (0x4F6760)
  crates/aa-numeric/src/lib.rs          + to/from_x87_extended_bits, exp2_binary32
  tests: aa-runtime/tests/simulation_native.rs, simulation_round2.rs,
         removal_request_services.rs, aa-world/tests/removal_request.rs,
         aa-trig/tests/angle_extended.rs

RUN
  set PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-tick/native_tick_driver.py
  python research/phase8-tick/native_removal_request.py
  python research/phase8-tick/rng_sites.py
  python research/phase8-tick/native_tick_round2.py
  python research/phase8-tick/native_angle_extended.py
  python research/phase8-tick/fpu_scan.py
  python research/phase8-tick/emulator_limits.py
  CARGO_TARGET_DIR=target/s-tick cargo test -p aa-runtime -p aa-world -p aa-numeric
      -p aa-simulation -p aa-trig --release --locked --offline -j 4

1. FRAME LOOP VS SIMULATION TICK
  TGC_NetNode::Update 0x765410 (once per rendered frame; real time =
  timeGetTime()*0.001 via 0x59ABA0 stored at NetNode+0x58) calls
  TGC_AdminNode frame update 0x75CA70, then 0x764FA0/0x77C5B0, then a
  real-time budgeted SAI think loop (UVars SAITimeBetweenThinks/
  SAIMaxAccumulateThinkTime, 0x7A0080), then frame-time statistics.
  0x75CA70 does AT MOST ONE simulation step per frame, in this order:
    a. disconnect timeout (+0x2CD, now-+0x2D0 > 2.5 [0xA874AC]) -> 0x751770
       reset + 0x7642A0                         [Rust: explicit Unsupported]
    b. 0x776540 network pump; 0x75C9A0 ProcessCommands (incoming messages)
    c. return unless +0x2C0 running and +0x158 admin clock exist
    d. gate = min(NextTickEventAdminTime 0x76AF90, admin time +0x28C)
       (event stored to binary32 when <= admin time); real admin clock
       (+0x158 vfunc +8, arg 1) < gate -> no step this frame
    e. RenderDumpAVI (UVars+0x1C8): pump 0x76B030 up to RenderAVIFramesPerTick
       (UVars+0x1CC) frames into KKC_Sim+0x38; fewer than N -> no step
    f. if 0x76AF90 < admin time (strict, x87 ST0 vs binary32):
         WORLD TICK: log "GetNextTickEventAdminTime()", 0x752AE0:
           0x74D640 replay record (if +0x268), 0x751A40 replay playback
           (if +0x26C: decodes and APPLIES replayed decrees via 0x74D710),
           0x76F790 (ecx=KKC_Sim): AVI counter +0x38 = 0, sync-log world
           begin time twice (logs 0xB3655C, 0xB36560), 0x41C180 timing
           smoothing, GWorld update 0x4F9970, 0x76F600 end-of-session check.
           (0x752AE0 also keeps wall-clock duration statistics, +0x60.)
       else ADMIN TICK if allowed:
         host (node == local node 0xB36558 +0x2C4, SetHost 0x757010):
           unless in BACKUP MODE (session f4 && session mode && +0x2D4,
           SetBackupMode 0x757250); client: only while processed +0x20C <
           received +0x218.
         0x755290: sync-log cadence counters 0xB40908 (client) / 0xB40904
           (host, mode 2, gate 0x76AEE0) flush after 17 ticks; 0x752DA0
           unless backup mode; host: 0x75FD80(0.1), 0x752E60 EXECUTE CLIENT
           REQUESTS (decrees), 0x752EA0 record history; client: 0x74FD10
           REPLAY HISTORY ENTRY +0x20C (decrees) then +0x20C += 1;
           admin time = f32(admin + 0.0625 [0xA8C084]); 0x76B780(admin time).
         then log "GetNextTickEventAdminTime()".
    g. heartbeat: now > +0x2DC -> 0x75B820; +0x2DC = f32(1/
       NetworkAdminAsyncTicksPerSecond (view SVars+0x11C) + now)
    h. server node (+0x2C8, SetServer 0x759790) -> 0x759680 backup-mode
       decision (round 2, section 9).
  Note: 0xB36558 is the local TGC_AdminNode itself (ctor 0x75CE80); round 1
  called +0x2D4 "playback" - it is the backup mode set by 0x757250.

2. CLOCKS AND FIXED STEP
  World clock GWorld (0xB36534): +0xDC tick (i32), +0xE0 begin, +0xE4 end,
    +0xE8 step = f32(1/GameTicksPerGameSecond) (SVars+0x1C; shipped
    SVars.tgi = 16 -> 1/16 s; compiled default 10). 0x4F9970 stores
    begin=end, end=f32(i32(tick)*step) (phase2/phase7 oracles).
  Admin time TGC_AdminNode+0x28C: +1/16 per admin tick (constant 0xA8C084,
    independent of the AdminTicksPerSecond SVar, which only scales +0x2C).
  KKC_Sim timeline (0xB36554, ctor 0x76F240): +0x10/+0x14 begin game/admin,
    +0x18/+0x1C end game/admin, +0x20 GameSpeed, +0x24 max 64.0, +0x28
    paused, +0x29 frozen (ctor: frozen), +0x2C game seconds per admin tick =
    min(f(GameSpeed),64)/AdminTicksPerSecond (0x76B230/0x76F370; ctor
    1/AdminTicksPerSecond, SVars+0x20, default 16), +0x38 AVI frames.
    0x76AF90: if timeline enabled (session f4 || 0xB396C4 bit1 || !UVar
      SinglePlayerAutoPause) and !paused and !frozen and end_game !=
      begin_game and g_gworld: (world.end - begin_game) * f32((end_admin -
      begin_admin)/(end_game - begin_game)) + begin_admin (ST0, x87 PC);
      else binary32 1e12 (0xA92F84).
    0x76B780(t): unless frozen: begin=end; end_game = f32(rate + end_game);
      end_admin = t (rate = 0x76AF30: +0x2C or 0 when disabled/paused).
    0x76DCE0 ResetAdminTime(t) (SetFrozen(false) 0x76DD90 when unfreezing):
      begin_game = end_game = world.begin; begin_admin = f32(t-1); end_admin
      = t; +0x24 = 64.
    0x76B120: render/presentation game time (interpolated, clamped to
      [world.begin, world.end]); not authoritative.
  CONSEQUENCE (proven by oracle): the sequence of admin and world ticks is a
  deterministic function of timeline state, game speed and x87 precision.
  Real time only delays frames. Speed 1: A W A W ...; speed 2: A W W A W W;
  speed 4: A W W W W; quarter: A W A A A A W. Rate 1/3 diverges between PC24
  and PC64 within 40 frames (precision changes the interleaving).

3. UPDATABLE REGISTRATION AND DISPATCH ORDER
  World update 0x4F9970 order (research/phase2-world-order, aa-world):
  fog cb, phase 1, first-tick, clock store, deferred +1A4 payloads, queues
  0 (+1AC, period .0001 -> every tick), 1 (+1C0, 1 s = 16 buckets), 2 (+1D4,
  5 s = 80 buckets), 3 (+1E8 deferred), phase 4, manager 0x54C280, phase 5,
  queue 4 (+208 deferred), phase 0, retained +110 scan.
  Queue vtables: periodic A30BE0 (+8 Update 0x50B330, +0xC add 0x50B2C0);
  deferred A30C24 (+8 Update 0x50B020, +0xC add_after 0x50AF10, +0x10 add_at
  0x50BA50). Updatable registration wrappers check the Updatable's own gate
  virtual first and then link the event at the head of the Updatable's
  list (+4): 0x50AC00 queue0 (gate +8, 17 callers: Kingdom init 0x4E5DF0,
  Organization, Performer, Construction, Properties, Structure, actor
  commands), 0x50AC60 queue1 (gate +8, 10 callers: Body, Character,
  Economy, Organization, Settlement, Structure, Wall), 0x50ACC0 queue3
  add_after (gate +0xC, 25 callers: GObject/GThing, CAI, Body, Character,
  Construction, Denizen, HQ, Performer, ...), 0x50AD20 queue3 add_at
  (Performer 0x57ED20/0x57F040), 0x50AD80 queue4 add_after (gate +0x10, only
  UAI 0x52DC10), 0x50B250 cancel all (46 callers). Source-file attribution
  of callers is by nearest .cpp string (heuristic).
  Per-actor order inside a queue is the native bucket order: periodic
  insertion draws ONE world RNG value for the bucket (0x50B2C0) and inserts
  at the bucket head; deferred insertion is time-bucketed; dispatch is head
  to tail with the native cursor rules (phase6-queues, aa-world queues).
  Ported in aa-world (queues.rs) and aa-runtime (clocked.rs,
  scheduler_owners.rs, updatables.rs) in earlier rounds.

4. DEFERRED QUEUES (DEATH/REMOVAL/SPAWN)
  Removal request 0x4F6760 (27 callers, e.g. 0x4CE750, 0x562380, 0x5656B0,
  0x58B130): an object already on +108 is ignored (returns 0, no flag);
  destroy sets object flag 2; in phases other than 3/6 (and phase 0 of
  session mode 3) the object is PREPENDED to +108 by 0x542A00, so the next
  phase transition drains the NEWEST request first (LIFO, proven). Phase
  3/6 (or 0 in mode 3) remove immediately: 0x4F53D0 (same body as the +108
  drain: availability/classification, phase!=6: 0x90, 0x8C retain ->
  F0(1), flags|=0x81, prepend +110; else F0(0), registry unlink, E0(1)) or
  0x4F5490 destroy (virtual +0xB4 and GTID 0xB364C0 handle at +0x158 may
  redirect to the live object; +0x2C(3), flags|=2, availability/
  classification, F0(0), unlink, E0(1)). Ported: World::request_removal.
  Drains: +100 pending components when leaving phase 1/4, +108 removals on
  every phase change, +110 retained scan after phase 0, +1A4 deferred
  payloads after the clock store (phase2 oracle). No separate spawn queue
  was found in the tick: spawned Updatables enter through the registration
  wrappers above (periodic RNG bucket / deferred time bucket).

5. AUTHORITATIVE RNG DURING A TICK
  World RNG = GWorld+0xD4 (m_rng), accessor GetRNG 0x4F02B0; primitive
  0x85D840 (cdecl, state pointer on the stack), bounded 0x40C8A0.
  rng-sites.json: 209 direct draw sites in Ghidra functions; 66 use the
  world RNG via 0x4F02B0; 25 further call sites lie outside Ghidra function
  bodies (10 of them call 0x4F02B0, including the periodic queue add
  0x50B2C0). Other RNG objects (presentation, NOT authoritative):
  [0xB3653C]+0xD4 (interface/view object, ctor 0x5CAE30, 36 sites),
  [[0xB36528]+0x2C] (app object, ctor 0x7A0AA0, 15 sites), others.
  Inside a world tick the world RNG is only drawn by callbacks dispatched in
  the native order above (queue deliveries, payloads, manager, removal
  leaves) and by registrations they perform; therefore draw order == native
  dispatch order, which the scheduler preserves (single thread). Admin ticks
  draw only through decree handlers (0x752E60/0x74FD10 -> 0x74D710 ->
  0x766CE0), in decree order. Notable non-tick draws: session launch dump
  0x775BF0 -> 0x4F8AE0 -> 0x4F6280 ("m_rng.GetFRand()" consumes ONE draw);
  0x50AB10 (TickCountPerTrackPlacement object ctor) draws world RNG.
  Direct-call reachability from 0x4F9970 finds only 7 world-RNG sites
  (virtual dispatch is not followed): a lower bound, not a call order.

6. DECREES
  Applied in admin ticks, never inside 0x4F9970: host 0x752E60 iterates
  client list +0x2AC and runs each client's queued requests through
  0x751B60 -> 0x74D710 (decree handlers via 0x766CE0) in encoded order,
  then 0x752EA0 appends the admin tick to m_decree_history (+0x218);
  clients replay history entry +0x20C via 0x74FD10 -> 0x74D710 and
  increment +0x20C. Replay playback (+0x26C) applies decrees in 0x751A40
  right before 0x4F9970. Admin time advances AFTER the decrees and the
  timeline is extended (0x76B780) with the new admin time.

7. THREADING (what may run on workers)
  Proven order-dependent: the whole world tick (shared world RNG, intrusive
  list cursors, retained/removal list mutation inside callbacks, queue
  bucket order), decree execution (encoded order), timeline arithmetic
  (sequential x87 rounding). Nothing inside a frame is proven
  order-independent, so simulation.rs implements no parallelism. Safe to run
  concurrently WITH the tick (outside it): rendering from a committed
  snapshot, 0x76B120-style interpolation reads, audio, asset loading. The
  existing aa-simulation laboratory provider batch shows the pattern for
  future proven-independent per-actor work (pure jobs merged in native
  order on one thread).

8. x87 PRECISION (round 2: fpu-precision.md)
  The tick runs on the net TGC_Thread (thunk NetNode+0x40 -> 0x765410,
  started at 0x41ABDC after CreateDevice at 0x41A200), whose CRT start
  0x97135A performs no FP init (_FPmtinit 0xB3509C = RET 0x7D723C). Direct3D
  (no FPU_PRESERVE: 0x89198C, 0x710AC0) switches only the main thread to
  PC24. No game code changes the control word; reachable CRT math restores
  it. Static verdict: the authoritative simulation runs at 0x027F (PC53),
  refuting the PC24 working policy, with the OS initial-thread CW as the one
  residual assumption; a dynamic check procedure is described. Oracles now
  cover PC24, PC53 and PC64 (rate 1/3: PC24 differs from PC53/PC64).

9. ROUND 2: SPEED, CLOCK ESTIMATE, FREEZE, BACKUP MODE, REMAINING SEAMS
  GameSpeed (simulation/speed.rs, Timeline::set_game_speed/apply_game_speed):
    0x76F370 clamps the argument to [GameSpeedMin, GameSpeedMax] (SVars
    +0x60/+0x64) or the film range (+0x68/+0x6C) in session mode 4 without
    f4; max also film while the world is in a cutscene (0x76B3A0); a value
    below min takes min without the max test. changed = |f32(old-new)| >=
    1e-4 (0xA92F8C) drives the UI message branch 0x76F474 (caller's seam);
    sync log "m_game_time_per_tick". 0x76B230: factor = f32(_CIpow(2.0,
    speed)) (stored twice), min with +0x24 unless greater, +0x2C =
    f32(factor / AdminTicksPerSecond). _CIpow 0x96D890 = MSVC CRT; SSE2 body
    0x977A10 when ___sse2_available_init 0x9773D4 set 0xB4B340 (SSE2 CPU but
    not AMD family 0xF) and MXCSR/CW mask all exceptions, x87 body 0x96D8ED
    otherwise (rounds to 24 bits at PC24). Neither body is correctly
    rounded (one-ulp binary64 deviations). Port: integral exponents exact
    in ST0; every exponent at the binary32 store via a 256-bit exp2
    bracket plus a one-ulp robustness check (Unsupported if ambiguous).
    Result: all 3132 native rows reproduced at the store (84 exact in ST0,
    0 refused). Caveat: Unicorn's x87 F2XM1/FYL2X are binary64-based, so
    the x87-body rows are emulator results.
  Admin clock estimate 0x74B180 (Driver::admin_clock_estimate):
    min(f32 real clock, min(0x76AF90, admin time)) in ST0; 180 rows exact
    (80-bit) at PC24/53/64.
  SetFrozen 0x76DD90 (Driver::set_frozen, Simulation::set_frozen): when a
    frozen timeline is unfrozen, ResetAdminTime(f32(0x74B180(1))) on the
    local node; sync log "_frozen"; then +0x29 = argument. 48 rows exact.
  Backup mode 0x759680 + 0x757250 (Driver::server_backup_update /
    set_backup_mode): outside active backup mode, any connection with a
    wants-backup flag (+0x49/+0x4B/+0x4A/+0x30) enters it; in active backup
    mode it is left once no connection has a flag or +0x48 (also with no
    connections). SetBackupMode stores +0x2D4 and +0x2D8 = now on every call;
    entering from normal mode also runs diagnostics and the GameSpy report
    0x791800 (seam backup_mode_entered). Verified in 3 driver cases.
  Heartbeat 0x75B820: network only - server dispatch of backup requests
    (0x7593E0: chat 0x753940, vote-to-disconnect 0x74F890), lag/connection
    state 0x75A2C0 -> 0x759530, statistics 0x74F1A0/0x752C60 every 16
    calls, ProcessBackupDecrees 0x75A0C0 (backup chat/votes, not sim
    decrees), file-transfer status 0x74F310 on the host. No simulation
    state; owner: network agent.
  0x76F600 end-of-session check (after each world update): if session
    +0x16D and no opposing human remains (0x76BA90), once (+0x16E) and
    mission state != 2, shows "ui_mission_no_human_opponents_remaining(
    _gamespy)" (0x40BC50) and reports to GameSpy. Reads world, mutates no
    simulation state; owner: session/UI (seam end_condition_check).
  0x41C180 KKC_TimingStats smoothing (0xB39BB8): wall-clock delta via
    0x866060, exponential smoothing with View SVar TimingSmoothingConstant
    (+0x68), +0x154 smoothed interval, +0x158 reciprocal; no reader found
    outside KKC_TimingStats (Source/Main/TimingStats.cpp). Diagnostics;
    owner: client (seam timing_smoothing).
  World creation: KKC_Sim::CreateGWorld 0x76AA90 constructs GWorld 0x4F7D00,
    assigns m_rng with the native seed constructor 0x85D880(seed) and runs
    GWorld::Init 0x4F27E0 (seed from KKC_WorldCreator 0x513410/0x5126C0).

10. aa-trig signed_angle_extended (0x445400 unstored ST0)
  aa_trig::signed_angle_extended returns the 80-bit register image that
  0x445400 leaves in ST0 (same host-x87 instruction sequence as
  signed_angle, ending in FSTP TBYTE instead of the caller's FSTP DWORD).
  Evidence: native-angle-extended.json (2400 own Unicorn rows, PC24/53/64,
  sticky PE clear/set) and the 522 calls captured in
  research/phase8-ai/native-rout-score.json. Unicorn computes FPATAN through
  binary64 and never raises PE from arithmetic (emulator-limits.json), so
  80-bit equality cannot be proven from these captures: every row agrees
  within one binary64 ulp and stores the identical binary32; 1910/2400 own
  rows and 400/522 captured calls are bit-identical in all 80 bits. The
  real-hardware FPATAN low bits (CPU-model dependent in the original too)
  remain unproven. The AI captures should not be used as 80-bit ground
  truth beyond binary64 (requests-to-lead.md).

PROVEN / COMPILED / UNTESTED (after round 2)
  Proven by replayed oracles: Driver::frame decisions, every seam call and
  sync-log value, admin time, heartbeat, backup mode/time, sync counters,
  timeline and world clock bits after each of 776 frames (31 cases,
  host/client/server, speeds, pause, freeze, backup gate, autopause gate,
  AVI mode, real-time gating, PC24/PC53/PC64); Timeline arithmetic (180
  rows, 80-bit); SetGameSpeed/0x76B230 (96 rows, both pow bodies, PC24/64);
  _CIpow at the binary32 store (3132 rows), exact for integral exponents;
  admin clock estimate (180 rows, 80-bit); SetFrozen (48 rows);
  World::request_removal (11 cases) and the clocked adapter (1 case);
  signed_angle_extended within one binary64 ulp + identical binary32 (2922).
  Compiled, not oracle-tested: Simulation::step/set_frozen composition,
  Timeline::new (one hand check), disconnect timeout error.
  Not ported (explicit seams, owners): network pump, command processing,
  decree decoding/handlers (orders/net agent), replay record/playback,
  sync-log transport, heartbeat (network), 0x76F600 (session/UI), 0x41C180
  (client diagnostics), SAI think loop (AI), 0x752AE0 statistics.
  Static only: thread/precision analysis (fpu-precision.md).
