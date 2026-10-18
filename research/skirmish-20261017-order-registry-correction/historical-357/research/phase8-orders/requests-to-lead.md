# Requests to the lead (phase8 orders/net agent)

## 1. Home for the NetworkSeams adapter (Cargo.toml / Cargo.lock)
`aa_runtime::simulation::NetworkSeams` cannot be implemented inside aa-orders
without an aa-orders -> aa-runtime dependency edge (Cargo.lock change), and
aa-runtime must not depend on aa-orders' internals the other way either.
I ported the native order/decree/sync part as `aa_orders::net::seams::NetworkCore`
(aa-orders types only) and wrote the 1:1 adapter:

    research/phase8-orders/network-seams-adapter.rs.txt

It compiles against today's workspace sources (checked from a scratch project
with path dependencies; target dir target/s-orders-adapter; your Cargo.lock
untouched). Please either
  (a) create a crate `aa-netcode` (deps: aa-orders, aa-runtime, aa-simulation)
      with that file as `src/lib.rs` and give it to me, or
  (b) paste it into aa-client.
The adapter implements natively: sync_log (A/S routing and hashing),
client_sync_flush (302), host_sync_flush (299), admin_history (0x752DA0),
host_execute_requests (0x752E60/0x751B60 + every ProcessRequest),
host_record_history (0x752EA0), client_replay_history (0x74FD10),
replay_record (0x74D640), replay_playback (0x751A40). The remaining seams
(net_pump 0x776540, process_commands 0x75C9A0, real_admin_clock,
avi_pump, timing_smoothing, end_condition_check, host_rate 0x75FD80,
heartbeat 0x75B820, backup_clients, backup_mode_entered, film streams) are
forwarded to an explicit `SessionSeams` trait: no default successes.

## 2. Game effects needed from other owners (GameEffects::process)
`NetworkCore` calls `GameEffects::process(decree, &mut fields)` for every
decree on every node (host inside ProcessRequest, clients on replay). Each
native process address is in inventory.json (`process`). Specifically for
your note (3):
* 73 KKC_SetGameSpeedOrderDecree, process 0x75D810: applies only when the
  session exists, the ordering player admin+0x284 exists, and 0x76AD90 holds
  ([0xB36534]+0x275 == 0, the player's client exists and is the authorizing
  admin 0x74B900, i.e. the host's player); then 0x76F370(fields[0] raw u32)
  -> `Timeline::set_game_speed` (not present in simulation.rs yet) which
  logs A "m_game_time_per_tick" (SyncLog::GameTimePerTick).
* 298/320 SetPlayer: aa_orders::commands::process_set_player (phase5, proven).
* 47/66 move: aa_orders::native::process_move (phase4, proven).
* 85/87 actor command: aa_orders::commands::process_actor (phase5, proven).
* 84/86 selected command: commands::process_selected boundary; 0x4ED360
  (selection distribution, formations) is still unported (actors owner).

## 3. FPU policy
Acknowledged: PC53 for the tick thread. The only x87 statements in my code
are the transport RTT (`net::transport::smoothed_rtt`, sample subtraction);
precision is an explicit `X87Precision` argument (oracle fixtures cover PC24
and PC53; PC64 returns `Error::X87Unsupported` unless exact). Pass Double.

## 4. aa-client compatibility (checked: `cargo check -p aa-client --locked --offline` passes, validation-client-check.log; no aa-orders warnings)
aa-client uses `aa_orders::{dispatch, Gate, decode_payload, KnownPayload}`
and `commands::{CommandWire, ...}` (mechanics_check.rs, world_check.rs).
Unchanged signatures. `commands::ParameterSet` gained
`negative_count: Option<i32>` (`#[serde(default)]`), so existing JSON
fixtures still deserialize; struct literals elsewhere would need the field
(none found by grep). New crate-root items: `dispatch_observed`, modules
`payload`, `flow`, `net` (a glob import `use aa_orders::*` could shadow a
local name only if the client defines items with these names).

## 5. Dedicated crate for the socket/channel layer (next round)
The native channel sections (BlockSender/Receiver, MessageHistory blocks),
file transfer and admin packet types need the native bitstream in
aa-orders. aa-transfer (my other crate) does not depend on aa-orders and adding
that edge changes Cargo.lock. Request: allow aa-transfer -> aa-orders, or a
new crate `aa-netcode` (see 1) that I own for transport I/O, channels, file
transfer, connect/join, heartbeat and backup/host migration.

## 6. UI item: 0x76F600 "no human opponents remaining"
Called after every world update (driver end_condition_check). Native rule
(pseudocode 0x76F600, 0x76BA90, 0x76BB90): if session+0x16D is set and
0x76BA90 finds NO pair of non-AI session players whose bound kingdoms are
not defeated (kingdom+0x19C != 2) and are in different teams (+0x1EC
differs, or both 0), and session+0x16E is still 0 (it is then set to 1, once
per session) and [GWorld]+0x48 != 2: take the first non-AI player with a
live kingdom (0x76BB90); if one exists show localized
"ui_mission_no_human_opponents_remaining" (or the "_gamespy" variant when
[0xB40C9C]+0xB8 is set) through 0x4B9F40/0x405D60, play sound
0x40BC50(0xB399B0, 0xF), and with GameSpy report 0x793B40(kingdom IDS +0x24).
Pseudocode level only; not ported by me.

## 7. HANDOFF / docs
Please index research/phase8-orders (README.txt, inventory.json) and
research/phase8-net (README.txt, findings.json) in HANDOFF.txt.

## 8. Stale statement outside my files
research/phase5-orders/README.txt ("Negative parameter counts ... return typed
errors") is now contradicted by native evidence (phase8-orders README section
3; 0x4C2C60/0x50B530/0x4C2050): negative counts read no entries and are
written back unchanged, and CommandWire now does that. Please annotate that
README (I did not edit it; it is not in my file set).

# Round 2 (crates/aa-netcode)

## 7. Dependencies (nothing needed)
* aa-transfer was NOT made to depend on aa-orders; aa-transfer is unchanged.
* aa-netcode does not use its aa-transfer dependency (you may drop it).
* `Precision` (aa-numeric) is reached through `aa_runtime::simulation::Driver`
  (speed.rs binds 0x75D810 to `driver.timeline.set_game_speed(..,
  driver.precision)`), so aa-netcode needs no aa-numeric edge. If you prefer
  a `&mut Timeline` + `Precision` signature, add `aa-numeric` to aa-netcode.
* No Cargo.toml / Cargo.lock change was made by me.

## 8. Integration points for the tick driver / client
* `aa_netcode::seams::{Network, SessionSeams}` implements
  `aa_runtime::simulation::NetworkSeams` (the round-1 adapter, now compiled).
  `SessionSeams::heartbeat()` should call `aa_netcode::heartbeat::AdminNode::heartbeat`
  and `backup_clients()` its `backup_clients()` (heartbeat agent's port).
* Decree 73 / 0x75D810: `aa_netcode::speed::process_set_game_speed(driver,
  fields, authority, settings, context, path)` from your
  `GameEffects::process`; it returns the `SpeedChange` (UI message flag and
  sync-log entry are yours to apply, as in Timeline::set_game_speed).
* UDP: `aa_netcode::udp::UdpNode` (std::net, no caps), send gate
  `aa_netcode::send::wanted_packet_type`, packet bodies in
  `aa_netcode::packets`, Connect in `aa_netcode::session`. tests/loopback.rs
  is the reference wiring (two threads, real sockets, loopback only).
* Two AdminConn structs exist: `packets::AdminConn` (channels, histories,
  proven bindings) and `heartbeat::AdminConn` (timing/statistics/lag fields).
  `packets::apply_channel_op` maps heartbeat `Effect::Channel` onto the
  packet-layer channels (tests/channel_effects.rs). Merging them into one
  struct is the next integration step (mine if you give me another round).

## 9. v14 vs original in the admin layer
docs/BASELINE.txt targets v14. v14 changes the admin layer at 0x74BEC9 (send
backlog caves 0x9EFF40/0x9EFEC0: unsent decree packets beyond AVar 0x4D and a
file with unsent blocks while map-transfer status is 2 count as backlog),
0x74E370 (type-5 file budget 0x2403 instead of 0xC03) and the file terminal
length (0x76146A/0x761270). All three are ported with
`aa_netcode::NativeImage` (default V14; `Original` selectable). A v14 node and
an original node interoperate on the wire except for the file terminal-length
fix (needs both ends, see file-transfer-notes.txt).

## 10. Emulator caveat re-check (your message)
Done for every harness of mine (research/phase8-net/null_guard.py). Results
unchanged everywhere; 96 trace runs and 4 flow cases had silently read the SEH
word through missing synthetic objects ([[0xB36530]+0x4A0],
[0xB39AB0]+0x1BC/+0x1C0), which are now supplied, with byte-identical
results. The 17 + 6 native faults are genuine original null dereferences on
hostile input (details in research/phase8-net/README.txt header).

## 11. Lobby join path (session layer)
`aa_netcode::lobby` ports the private join messages (0x7531A0, 0x74C130 /
0x74DF50, 0x74C270 / 0x76D0B0 / 0x77AAC0), host join completion 0x753580 and
the PlayerJoin decree process 0x758FD0, oracle-backed (native-lobby.json,
tests/lobby.rs). Its `SnapshotHost` (ChangeToState 0x77A750, map-status sends)
and `JoinHost` (AutoAssignKingdomToPlayer 0x778D10, localization) traits need
the session/UI owners. It is not yet wired into tests/loopback.rs because of
the synchronizer-resume question in research/phase8-net/README.txt section 12;
that needs an end-to-end native oracle (next round).

## 12. Round 3 (merged AdminConn, join resume, lobby in loopback, staging, ClientLeave)
* One AdminConn: `aa_netcode::packets::AdminConn` (heartbeat re-exports it,
  `PeerBase` is shared by AdminNode and AdminConn). Section 8's "two
  AdminConn structs" note is obsolete.
* Synchronizer-resume question settled (research/phase8-net/README.txt
  section 12): AdminNode+0x200/+0x204 are the byte/bit of the decree stream
  object at AdminNode+0x1F8, captured inside the PlayerJoin process, i.e.
  the decree-stream position right after the joiner's PlayerJoin decree.
  native-join-resume.json (original 0x74D710 host dispatch + 0x74FD10 client
  replay) shows the resumed client continues the host's synchronizer A
  sequence and a packet-start replay does not; tests/join_resume.rs replays
  it bit-exactly with aa_orders (decree bytes, every A/S entry and hash).
* tests/loopback.rs now uses lobby.rs end to end (Connect -> PlayerJoin
  request -> decree -> join snapshot + admin state -> resumed client); the
  host ticks before the client connects. Loopback only (127.0.0.1, two
  threads, plus a lossy reordering relay): not an internet or GameSpy claim.
* New ports (all oracle-backed, see README section 13): `aa_netcode::staging`
  (readiness 0x765FC0/0x76C960, launch countdown 0x774A50 / 0x772350 /
  0x76CFA0 / 0x7723F0, Launch 0x77A950, ChangeToState 0x77A750 + 0x77A6E0,
  AutoAssignKingdomToPlayer 0x778D10), `heartbeat::AdminNode::client_leave`
  (ClientLeave 0x755840 with the vote-kick sweep 0x74F160, 0x74EF90, task
  check 0x751F70 / 0x74DA50 / 0x74DAA0), `lobby::destroy_link_players`
  (0x7737E0 loop), `send::select_connection` (0x763EE0).
* Integration needs from the session/UI owners (traits, native order):
  `staging::StagingHost` (UI messages, AVar LaunchCountdownTime, timer
  0x59AB60, not-ready report 0x7725A0, world loading steps 0x775DB0 /
  0x770500 / 0x776B20 / 0x775EE0, settings sync dump 0x76D6C0, timeline
  0x76DD90, ResetOrderVersion, the session-state event 0x7851C0 and
  KKC_LaunchSettings::operator= 0x779F00, which I did not port: it copies
  every map/game sub-object), `staging::KingdomHost` (0x77E430 kingdom info,
  setters 0x777F00 / 0x777CD0 / 0x777610: scenario world data), and the new
  heartbeat effects (`LeaveMessage`, `AbortCountdown` ->
  `staging::abort_countdown`, `DestroyPeerPlayers` ->
  `lobby::destroy_link_players` + `lobby::update_rated`, `InGameLeave`
  0x76EE10, `TasksComplete` 0x750940/0x750990 on AdminNode+0x238 + session
  0x776560, `GameSpyPlayerLeft` 0x788290, `TimelineFlag28Cleared`).
* PeerBase.vote_to_kick now holds peers (PeerRef), as 0x74EF90 / 0x74D320
  store and compare peer pointers (the old "SessionPlayer ids" note was
  wrong; nothing read the list before).
* `HostConnectState::session_players_in_game` renamed `launch_countdown`
  (session +0x118 is the countdown 0x7723F0 shows; serde alias keeps the
  fixture key).

## 13. Hook for the combat agent: actor commands 0x4D9B60
KKC_GActorCommand::Process 0x4D9B60 is ported in aa-orders:
`aa_orders::commands::deliver_actor_command(&mut routing, slot, actor)`
with your `aa_orders::commands::CommandRouting` implementation:
* `handler(actor, slot)` = actor+0x60+4*slot (slot = [[command+8]+0x18]);
* `accepts(actor)` = actor virtual +0x38(command type);
* `handle(handler)` = handler virtual +0x24(command), low byte returned;
* `redirect(actor)` = virtual +0x158 (None = false) then +0x15C (Some(None)
  = null target: walk ends with the GActorCommands.cpp line 0xF7 log);
* `container_member(actor)` = [[actor+0x7C]+0x44] (None ends quietly).
It returns `CommandDelivery { result, null_actor_logged }`; a walk that
revisits an actor is `Error::CommandRoutingCycle` (the original loops
forever). Oracle: research/phase8-orders/native-actor-command.json (300
graphs), tests/actor_command.rs. Your special-operation and summon effects
should call it where they currently seam 0x4D9B60; the command object's
own handler bodies (+0x24) stay yours.
