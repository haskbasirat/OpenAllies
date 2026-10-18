PHASE 8 - ORIGINAL MULTIPLAYER PROTOCOL OF AA.EXE (TGC NETWORKING)
=================================================================

Source: original E:\Games\Axis and Alies\AA.exe (image base 0x400000,
SHA256 5d2548a2...0d26) and the accepted v14 image (3bda77d7...c5c1, the
rewrite's target per docs/BASELINE.txt). Every oracle records its executed
byte windows and whether they are identical in the other image. In the admin
layer v14 differs at exactly four places (all ported, NativeImage::V14 is the
default in aa-netcode): 0x74BEC9 (send-gate backlog test -> caves 0x9EFF40 /
0x9EFEC0), 0x74E370 (type-5 file budget target 0xC03 -> 0x2403), 0x76146A
and 0x761270 (file terminal-block length, crates/aa-netcode/src/file.rs).
Nothing was modified. Rust port: crates/aa-orders (order payloads, request/
decree flow, transport) and crates/aa-netcode (round 2: channels, admin
packets, connect, send gate, UDP, heartbeat, file transfer, game speed,
NetworkSeams adapter).

Emulator caveats (docs/NATIVE-PORT-RULES.md): every harness here installs
null_guard.py (page 0 faults unless an FS-prefixed access of [0..3], the SEH
link). Re-running all fixtures with it: transport, sync, channels, packets,
connect, speed, file transfer, send gate unchanged; order traces found 96
runs (12 RMC/session-load descriptors) that had read the SEH word through a
missing synthetic [[0xB36530]+0x4A0], and the flow harness 4 runs (cheat
decrees 30/35) through missing [0xB39AB0]+0x1BC/+0x1C0; both objects are now
supplied and every recorded result is byte-identical to before. The 17 trace
and 6 flow native faults were re-checked: all are genuine original null
dereferences on hostile payloads (0x511E50 reads a WorldRule index equal to
the null sentinel and dereferences it at 0x511EC9; 0x76A9C0 stores a null
for an unknown RMC name that 0x76A8C0 dereferences at 0x76A8E0 when the
request is re-serialized); the allocator seam was present in every run.

Evidence tags used below
  [ORACLE]  original instructions executed under Unicorn, fixture replayed by
            a Rust test (file named next to the claim).
  [MC]      read in the original machine code (capstone) during this phase.
  [PC]      Ghidra pseudocode only (reconstruction/research/native-aa/pseudocode).
  [INF]     inference; not proven.
Static reading was split across three read-only research passes plus my own;
everything marked [MC]/[PC] still needs an oracle before it is a parity claim.

-----------------------------------------------------------------------
1. LAYERS AND OBJECTS
-----------------------------------------------------------------------
  UDP socket (TGC_WinSock, global [0xB40B80]; Update 0x783D40, recvfrom
    0x783BB0, sendto 0x781FC0)
  -> optional simulated latency/loss (TGC_SimulatedNetEffects [0xB40B74],
     0x77C210 send / 0x77C3F0 receive / 0x77C5B0 drain; enabled only when one
     of AVars SPing/SPL/RPing/RPL/SPingV/RPingV (index 0xB9..0xBE) != 0.0;
     all default 0.0)                                                  [MC]
  -> TGC_NetNode ([0xB36524]; Update 0x765410, SendPackets 0x764FA0) and one
     TGC_NetConn per remote IPv4:port (0xA4 bytes, created on demand by
     0x764970 BEFORE any validation, freed only at node shutdown)      [MC]
  -> TGC_AdminNode ([0xB36558], ctor 0x75CE80; frame update 0x75CA70) and one
     TGC_AdminConn per NetConn (0x424 bytes at NetConn+0xA0, ctor 0x756BC0)
  -> reliable channels (BlockSender/BlockReceiver + MessageHistory) carrying
     order REQUEST packets client->host and DECREE packets host->clients,
     private join messages, backup traffic and the file transfer
  -> order descriptors (357, CRT registration, phase4) and their payloads
     (research/phase8-orders).

-----------------------------------------------------------------------
2. DATAGRAM (TRANSPORT) FORMAT                         [ORACLE native-transport.json,
                                                        tests/net_transport.rs]
-----------------------------------------------------------------------
Bit packing: every field uses the original 0x40D420 writer / 0x40D5D0 reader
(proved in phase3/phase5 and again here). Fields wider than the bits left in
the current byte are NOT a plain LSB-first stream: the low bits of source byte
k fill the current byte, bytes 0..k-1 are copied byte-aligned, then the rest
of byte k. Any reimplementation must use aa_orders::Cursor / WireWriter.

Position starts at byte 0 bit 3 (0x74AF40 reader ctor, 0x7652C9 send).
Writer 0x764DA0 -> 0x764CA0, reader 0x764BB0 -> 0x764600 -> 0x764550:
  bits 0..2  number of unused bits in the last byte, ORed in by the finalizer
             0x74D0A0: bytes = ceil(pos/8), pad = (8 - pos%8) & 7.
  1          session ids present (always 1). Receiver drops 0 (log 0x391).
  32         sender session id = NetNode+0xB0 = trunc(NetNode clock * 10.0)
             at Init 0x764190 (time derived, not random) [MC]. Receiver drops
             -1 (log 0x377).
  32         target session id = NetConn+0 (peer id last seen, -1 initially).
             Receiver accepts -1 or its own id (else log 0x37E) and stores the
             sender id into NetConn+0.
  32         sequence = ++NetConn+0x30 (first datagram 0, signed, no wrap
             handling; consumed even if sendto fails).
  32         ack = NetConn+0x34 = highest sequence received, -1 = none.
  N x 1      only if ack != -1: received flag of every sequence in
             [s, ack) with s = ack-32 when ack-32 >= 1 else 0, oldest first.
  4          admin packet type (0x763A60 writes 4 bits; reader 0x764950 reads
             0x411FA0(13) = 4 bits).
  ...        type body (section 3), appended without length or alignment.
No magic, game id, version, checksum or length field exists [MC].

Receive rules (all [ORACLE]):
  * ack == -1: "Accepting first packet" (log 0x3E6): mark received with RTT
    sample 0, no reorder check, no ack bits; dispatch.
  * otherwise RTT sample = now - sent_time[ack] when ack is inside the
    sent-time window, else 3.0 (0xA32690); a difference > 3.0 also -> 3.0
    (fst + fcomp, C0|C3 keeps the stored value).
  * sequence < highest - 4 -> rejected (log 0x3FD), nothing else changes
    (but the sender id was already stored).
  * mark received; when sequence > highest: highest = sequence,
    rtt = f32(0.75*rtt + 0.25*sample) (x87 statement, precision explicit),
    liveness time NetConn+0x2C = now. First packet ever: receive window
    restarts at that sequence (0x7644B0).
  * each ack bit is reported to AdminConn 0x74B4F0 as (sequence, bit==1), then
    (ack, true); 0x74B4F0 forwards to every BlockSender's 0x7608E0. A 0 bit
    counts as a loss immediately. Duplicates inside the window are dispatched
    again.
  * admin dispatch 0x75CD20(stream, sequence) (skipped while the admin node
    is leaving, AdminNode+0x2CD [PC]).
Sent-time history keeps sequences >= latest-200 (0x764AFF/0x764B16); the
received-flag window grows by one entry per received sequence and is never
trimmed for the life of the session [MC].

Packet types (writer case 0x7521F0 / reader case 0x75CD20) [PC, cross-checked]:
   2 Connect                0x750330 / 0x75BF90  (section 6)
   3 Reconnect              0x74C2D0 / 0x753820  32-bit admin+0x218-1
   4 ClientToServer         0x74E0F0 / 0x74E170
   5 ServerToClient         0x74E280 / 0x75A350
   6 ClientToBackupServer   0x74E3C0 / 0x74E430
   7 BackupServerToClient   0x74C310 / 0x75A5C0
   8 ServerDisconnect       0x74B680 / 0x75A730  empty
   9 (no case: header only, ignored)
  10 ClientDisconnectRequest 0x74B6B0 / 0x74F7C0 empty
  11 ClientDisconnectDecree 0x74C380 / 0x75A780  4-bit reason (13 values):
     0 game_full, 1 password, 2 wrong_version, 3 banned, 4 kicked, 5 quit,
     6 bad_depot_checksums, 7 bad_data_checksums, 8 wrong_mods,
     9 internet_lan_mismatch, 10 game_in_progress, 11 cdkey_auth_failed,
     12 prompt_not_original_host.

-----------------------------------------------------------------------
3. ADMIN PACKET BODIES AND RELIABLE CHANNELS                  [MC/PC]
-----------------------------------------------------------------------
AdminConn channels (ctor 0x756BC0; bindings 0x752490/0x7525B0/0x752300/
0x7523A0):
  +0xCC/+0x114  broadcast decree   sender/receiver (bound to admin+0x214)
  +0x144/+0x18C reverse decree     (host migration)
  +0x1BC/+0x204 broadcast request  (client admin+0xC -> host conn+0xC)
  +0x234/+0x27C private decree     (host conn+0xAC -> client; join snapshot)
  +0x2AC/+0x2F4 backup decree      (admin+0x224)
  +0x324/+0x36C backup request
  +0x39C/+0x3E8 FileSender 0x7611B0 / FileReceiver 0x761870
Generic senders: MessageHistorySender 0x763520 -> BlockSender 0x760390 with
span 32 and at most 2 in-flight copies per block; receivers span 32.

Body of type 4 (client -> host, 0x74E0F0):
  broadcast-request channel up to absolute bit 0x403; 32-bit backup index
  (conn+0xA0); status block 0x74C6D0 (1 bit request backlog, 1 bit
  NetNode+0x64 falling behind, 32-bit conn+0x60); backup-request channel up
  to bit 0x603.
Body of type 5 (host -> client, 0x74E280):
  1 bit backup mode; 32-bit backup index; private-decree channel to 0x803;
  backup-decree channel to 0xC03; broadcast-decree channel to 0xC03; 32-bit
  file-transfer generation (conn+0x418, NOT charged to the file budget, which
  was computed before it); file channel to 0xC03.
  Reader 0x75A350 applies status/backup changes only from the most recent
  sequence and drops an older packet's payload ("ignoring not most recent",
  0x75A387..0x75A3B8) although it was already acknowledged [MC]. With one
  in-flight copy for files this can lose file blocks permanently [INF].
Types 6/7 carry backup index, status, backup channels with fixed budget 0x400.

Channel section (BlockSender 0x760A80 / BlockReceiver 0x760F20, skip 0x75FEA0):
  1 bit present (0 = empty section)
  32 bits first block index F
  span presence bits for F..F+span-1, each followed by its block when 1
Selection budget = limit - span - 33; retire packet records older than
sequence-100 as lost (0x760A40); passes p = 0..copies-1 pick blocks with
in-flight count p and estimated size < remaining budget, all within 32
indices; record {state 1, first, end, 32-bit mask} per sequence. ACK 0x7608E0:
state 2 and blocks acked (-1); NACK: state 3 and in-flight-- ; then retire
records older than ack-32 and drop acked blocks. No wall-clock retransmit
timer exists: loss is inferred from ack bits and sequence age only.

MessageHistory blocks (0x762D30 / 0x761FA0 / 0x7633C0 / reassembly 0x7632D0):
each history packet (one sealed request or decree packet) is cut into blocks
of at most 255 bits: 1 bit isLast, if isLast an 8-bit bit count, then the
bits; block 0 is a header carrying the 32-bit start history index (as a
32-bit byte count + bytes payload, 0x75FFA0). The history index of a packet
is implicit (start + order); no per-packet index travels on the wire.

-----------------------------------------------------------------------
4. ORDERS: REQUEST -> DECREE (VERIFIED SEPARATELY)
-----------------------------------------------------------------------
See research/phase8-orders/README.txt. Summary:
  * Every order is a 9-bit descriptor id (0x766D00, width 0x40C7C0(357))
    followed by its payload. All 357 payload encodings are ported and
    proven against the original readers/writers [ORACLE order-traces.json].
  * Client emitters (one per request, e.g. 0x61E960 for 84) write into the
    request stream admin+0x254, preceded by a TGC_SetPlayerOrderRequest (320,
    32-bit player id) whenever the issuing player differs from admin+0x210
    (0x74B360).
  * Each client admin tick seals +0x254 into one request packet appended to
    admin+0xC (0x752DA0, history keeps 200) and the broadcast-request channel
    carries it to the host.
  * Host admin tick (0x755290): 0x752E60 visits every peer in admin+0x2AC (the
    host itself first) and runs its queued request packets through 0x751B60:
    the first packet of a peer this tick emits TGC_SetPlayerOrderDecree (298)
    for the peer's ordering player (0x7509F0), then 0x74D710 dispatches every
    descriptor; each request's ProcessRequest validates and writes the decree
    into admin+0x1F8 and EXECUTES it immediately [ORACLE order-flow.json].
    0x752EA0 seals admin+0x1F8 as decree packet #admin+0x218 (empty packets
    included; host keeps 1000, 0x752F14).
  * Clients execute decree packet N at their admin tick N (0x74FD10) only when
    it has arrived (admin+0x20C < admin+0x218): this is the lockstep gate.
    The tick driver (aa-runtime simulation.rs, phase8-tick) owns the timing.

-----------------------------------------------------------------------
5. LOCKSTEP, TICKS, LATENCY                                   [MC/PC]
-----------------------------------------------------------------------
Admin tick = 0.0625 s of admin time (constant 0xA8C084). Host clock caps
catch-up at 0.1 s (0x75FD80). Clients run in real time up to the newest
received decree packet (0x74C850 -> 0x75FB40). There is no fixed "execute at
tick N+k" constant: command latency is the sum of request sealing (<= 1 client
tick), ClientToServer cadence (0.1 s), transit, host tick, host send cadence
(0.1 s, or bursts while backlogged), transit back [INF].
Async heartbeat at 16 Hz (AVar NetworkAdminAsyncTicksPerSecond).

-----------------------------------------------------------------------
6. CONNECT, JOIN, LOBBY, LAUNCH                               [MC/PC]
-----------------------------------------------------------------------
Connect (type 2, sent every > 0.25 s while connecting): CString name,
password, version DAT_00B3888C, version2 DAT_00B388A0, 32-bit data checksum,
CString depot checksums (0x5AF810), CString mod checksums (0x5AF970), identity
block (32, CString player id, 3 x 32), 1 bit internet flag. Host checks in
order (0x75BF90): migrated host (12), internet/LAN mismatch (9), game in
progress state 2/4 (10), version (2), version2 (2), password (1), ban list in
internet mode (3), depot checksums (6), mod checksums (8), data checksum (7);
on success issues TGC_PlayerJoinOrderRequest itself (0x74C9C0).
PlayerJoin decree 317 (0x758FD0 on every node): session-full check 0x76CCA0
(humans = non-AI, non-film players, compared with session+0xD4; observers
count), new SessionPlayer id = ++session+0xE8, kingdom auto-assignment, host
queues the private join snapshot (session 0x74C270 + admin state 0x74C130)
and in internet mode a Challenge (300/312, 301/313 gcd CD-key, then
MarkPlayerAuthenticated 310/319). PostPlayerJoin (307/318) executes a
`ret 4` in AA (no effect) [MC].
Launch: StartLaunchCountdown 96/97 (host-ordered, all ready) -> host counts
LaunchCountdownTime (AVar, 5) and sends LaunchCountdownProgress 325/329 each
second -> TGC_Launch 0/5 -> KKC_Session::Launch 0x77A950 (ChangeToState(state
from launch settings), ResetOrderVersion). Session states (+0xE4): 0 reset,
1 staging, 2 in game, 3 editor, 4 film, 5 metagame.
Ports: host binds UDP 0.0.0.0:HostPortDefault (7290) .. +NetworkNumPortsToTry
(10)-1, first success; clients use an ephemeral port. Internet games use
GameSpy QR2 (query port 8850, gamename "aarts") and NAT negotiation; NatNeg
datagrams (magic FD FC 1E 66 6A B2) share the game socket and are diverted
before transport parsing (0x764E6E).

-----------------------------------------------------------------------
7. DESYNC DETECTION   [hash/retention/validate: ORACLE native-sync.json,
                       tests/net_sync.rs; cadences and routing: MC/PC]
-----------------------------------------------------------------------
Two rolling hashes (TGC_Synchronizer, A=[0xB3655C], S=[0xB36560], created by
0x77CCB0(0x10000)): h = (h*37 + v) % 65536 (MSVC signed remainder, 32-bit
wrapping product), v = the int, raw float bits, 0/1 or every UTF-16 code unit
of a string; labels are not hashed (0x77DA70 int, 0x77D930 float, 0x77D9D0
bool, 0x77DB10 string). Logged only on the main simulation thread.
Order flow contributes "o.GetBitSize()" (decree-stream bits) before and after
each host decree, the decree name and id to A and (except SetPlayer, and only
in instantiations that have it) to S [ORACLE order-flow.json]. Clients send
TGC_SetSyncValueOrderRequest (302) {A index, A hash} every 17th admin tick;
the host validates against its own A log (0x77D050: unknown future index ->
mismatch, forgotten index -> accepted) and on mismatch emits
TGC_SyncFailureOrderDecree (303) once; every node then dumps its log. There is
no resynchronisation.

-----------------------------------------------------------------------
8. FILE TRANSFER (MAP/SAVE IN LOBBY)                          [MC/PC]
-----------------------------------------------------------------------
Only inside type 5. Generation word (conn+0x418): older -> skip, equal ->
decode, newer -> reset receiver then decode. Block payload: 32-bit byte length
then that many 8-bit values (0x75FFA0 / 0x7610A0 with 0x411FA0(256)).
Block 0 = header bitstream: 3-bit pad, 16-bit name length, UTF-16 name,
32-bit file size (built 0x761540, parsed 0x7619A0). Blocks k>=1 = file bytes
[(k-1)*BS, k*BS). Terminal block index L = ceil(size/BS); its length is
size % BS -> a non-zero exact multiple of BS loses its last BS bytes
(0x76144D.., 0x76124A..) [MC arithmetic, INF effect]; size 0 never closes the
file [INF]. Receiver drains contiguous blocks immediately (0x761CE0); the only
path check rejects names containing ".." (0x7619A0); the destination is
profile directory + name by plain concatenation. AVars:
NetworkFileTransferBlocksPerFrame 20 (span, must match on both ends),
BlockSize 64, MaxPendingBlocks 256.

-----------------------------------------------------------------------
9. TIMEOUTS                                                   [MC/PC]
-----------------------------------------------------------------------
Liveness (AdminConn::Update 0x75BD70, only with an admin clock and a
connected conn): 3.0 s since NetConn+0x2C for the upstream server conn
(AdminNode+0x2C8), 2.0 s for every other conn -> 0x75A250 clears connected
and fails over when it was the server. Disconnecting conn reset after 2.0 s;
leaving node shuts down after 2.5 s (0xA874AC). Lag "patience" per conn
starts at 15.0 s, +0.1/s without lag flags (max 15), -1/s with flags; lag
flags (every ~1.06 s) raise backup mode, which stops host admin ticks; peers
are dropped by vote when patience <= 0.1 (0x75A2C0). No connect/join timeout
was found.

-----------------------------------------------------------------------
10. NATIVE CAPS: ORIGINAL VALUES AND THE PORT POLICY
-----------------------------------------------------------------------
"Removed" = the Rust structure is growable / has no such limit. "Policy" =
not yet implemented because the socket/channel layer is not ported this
round; the cap must not be reintroduced when it is.
Removed in code this round:
  sent-time window 256 initial (0x7647C8)             -> Vec (retention rule 200 kept)
  received-flag window 256 initial (0x7647F1), grows    -> Vec<bool>, fallible
  request history 0x400 / decree history 0x100 initial
            (0x752A60 / 0x74B790), grow by doubling     -> History (growable)
  synchronizer ring capacity = max entries (0x77CC30)   -> VecDeque
  order request/decree/mirror bitstreams, 0x800-byte
            initial request buffer, +0x6B-bit growth slack -> WireWriter (growable)
  parameter-set and every native vector in payloads     -> Vec with explicit
            bounds only from the packet length (hostile-input safety, not a
            native cap) and a caller budget for zero-width lists
Removed in code in round 2 (crates/aa-netcode/src/udp.rs, channel.rs,
history.rs, file.rs, heartbeat.rs):
  0x783D8F  64-entry fd_set / select(0x40)                 -> one socket per
            node, no socket-count limit
  0x783DC7  >= 32 receive dispatches per WinSock::Update   -> receive_all
            drains every pending datagram
  0x783BE5  4096-byte static receive buffer                -> 65,535 (UDP max)
  0x76526D/0x765133 4096-byte send build buffers           -> WireWriter
  AVars NetLanBandwidth 200000.0 (registered 0x4230BA),
            NetMaxBandwidthToUnknown 4000.0 (0x423102),
            connection_types.tgi bandwidth 100000 (Broadband) / 4000 (Modem):
            aggregate + per-connection token buckets, cap = rate/4 bytes,
            +16 bytes accounting per datagram (0x765324), refill clamp
            100 s (0x764FD7) in 0x764FA0/0x763E30      -> not ported: no
            application bandwidth cap (OS/network still limit)
  BlockSender/MessageHistory initial capacities 0x100 (0x760390, 0x7611B0,
            0x761870), selection scratch 0x80 (0xB28A44), file block rings
            256 (0x7611FA/0x7618B4), header buffer 0x100 (0x761648),
            static build streams 0x400 (0x767110)            -> growable
  NetConn table (0x764970)                                 -> Vec
Kept as protocol pacing rules (not caps; see send.rs): 0x74BED7 / 0x74BF16
  8 / 9 datagrams per SendPackets pass while backlogged / joining. They are
  the only termination condition of a backlogged connection inside one pass
  of the original; removing them would need an invented rule. They are
  constants of crates/aa-netcode/src/send.rs and can be raised for
  Rust-only sessions without other changes.
Kept on the wire for compatibility with original peers (changing them breaks
interoperability; a Rust-only session may raise them):
  packet bit targets 0x403/0x603/0x803/0xC03 (0x74E0F0/0x74E280), 0x400
  (types 6/7); NetworkFileTransferBlocksPerFrame 20 / BlockSize 64 /
  MaxPendingBlocks 256 (AVar index 0x48..0x4A; both ends must agree).
Kept because they are protocol rules (changing them changes behaviour):
  reorder window 4, ack bits 32, sent-time retention 200, BlockSender
  retirement 100/32 sequences, 255-bit message blocks, request history 200,
  decree history 1000, synchronizer retention 65536 entries (forgotten index
  -> accepted), session max players (MultiplayerMaxNumPlayers 12; 16 is the
  audited candidate), all timeouts and cadences.

-----------------------------------------------------------------------
11. HAZARDS FOUND IN THE ORIGINAL (kept visible, not silently fixed)
-----------------------------------------------------------------------
  * A datagram with a huge sequence makes the receiver grow its received
    window to that size (0x763FD0/0x763D00): the oracle harness had to avoid
    such inputs; the Rust port uses fallible allocation and reports
    Error::Allocation.
  * Any datagram from a new address allocates a NetConn + AdminConn before
    validation and is never freed until shutdown [MC].
  * Packets 1..4 behind the newest are acked but their type-5 payload is
    dropped (file blocks lost with one copy in flight) [MC/INF].
  * Exact-multiple file sizes truncated; zero-size files never closed [INF].
  * File budget overshoot by the uncharged 32-bit generation word [MC].
  * The history receiver skip 0x7638B0 parses every skipped block as a bit
    block (0x7633C0), including the byte-buffer header block 0: skipping a
    section that carries a header (type 7 with a backup-index mismatch on a
    not-most-recent datagram, 0x75A5C0; same skip in 0x75A350/0x74E170)
    desynchronizes the stream and reads past the datagram into whatever
    follows in the receive buffer [ORACLE native-packets-misc.json: native
    read ends beyond the datagram]. The port reproduces the misparse and
    stops with a typed error at the datagram end.
  * Receiving history blocks on an unbound MessageHistoryReceiver
    dereferences a null history (header 0x76322C) [ORACLE guard]; typed
    error in the port.
  * HistorySender::GetOldestUnsent 0x762640 dereferences the bound history
    without a check when unbound and empty [MC]; typed error in the port.
  * Connect is accepted with the depot/mod checksum strings compared
    case-insensitively (C-locale _wcsicmp 0x96CAD6, ASCII fold only; AA only
    ever sets LC_NUMERIC) but versions and password case-sensitively [ORACLE].
  * A ClientDisconnectDecree stops the admin node (0x74B980) before reading
    the reason [MC].

-----------------------------------------------------------------------
12. STATUS OF THE RUST PORT (round 2: crates/aa-netcode)
-----------------------------------------------------------------------
Proven by original-code oracles, replayed by Rust tests (all in
crates/aa-netcode/tests; command: CARGO_TARGET_DIR=target/s-orders cargo test
-p aa-netcode --release --locked --offline -j 4):
  * message-history channels over TGC_BlockSender/Receiver (0x760A80,
    0x7608E0, 0x760F20, 0x75FEA0, 0x763520/0x763600, 0x7636B0/0x763130,
    0x762D30, 0x7632D0/0x7631D0/0x7626F0) - native-channels.json, 6
    scenarios / 103 steps, tests/channels.rs;
  * admin packet types 4 and 5 end to end on two native nodes (AdminConn
    ctor 0x756BC0, bindings 0x752300/0x752490/0x7525B0, 0x7523A0 via
    NewBackupIndex, ack dispatcher 0x74B4F0, status block 0x74C6D0/0x74C7B0,
    file generation 0x74C8B0, backlog test 0x74BCB0 and the v14 caves
    0x9EFF40/0x9EFEC0), losses, late datagrams, decree bursts, private
    messages, both images - native-packets.json, 2 x 5 scenarios / 292 steps,
    tests/packets.rs;
  * types 3, 6, 7, 8, 10, 11 (writers and reader gates, bindings, status,
    backup channels, take-over branch of 0x75A780) - native-packets-misc.json,
    204 cases, tests/packets_misc.rs;
  * Connect type 2 writer 0x750330 and validation 0x75BF90 (all ten reasons
    in native order, gates, case rules, ban lookup) and the PlayerJoin
    request payload 0x77B680 - native-connect.json, 31 cases,
    tests/connect.rs;
  * send gate GetWantedPacketType 0x74BDE0 with 0x74B600 and 0x74BD80, PC24
    and PC53, both images - native-send-gate.json, 4,096 cases,
    tests/send_gate.rs;
  * SetGameSpeed decree process 0x75D810 (with 0x76AD90, 0x74B3B0,
    0x74B900) bound to aa-runtime Timeline::set_game_speed (0x76F370, proven
    in phase8-tick) - native-speed.json, 48 cases, tests/speed.rs;
  * file transfer (FileSender/FileReceiver, header, payloads, terminal rule
    original and v14, drain, skip, generation) - native-file-transfer.json,
    tests/file_transfer.rs, research/phase8-net/file-transfer-notes.txt;
  * heartbeat 0x75B820, AdminConn::Update 0x75BD70, lag refresh 0x74F1A0,
    DisconnectLostPlayers 0x75A2C0, SetServer 0x759790, SetHost 0x757010 -
    native-heartbeat.json, 1,140 cases, tests/heartbeat.rs,
    research/phase8-net/heartbeat-notes.txt;
  * lobby join path (src/lobby.rs): private history dispatcher 0x7531A0,
    admin-state message 0x74C130 / 0x74DF50 (synchronizer A resume, client
    clock), session snapshot 0x74C270 -> 0x76D0B0 / 0x77AAC0 with every
    sub-codec, host join completion 0x753580, PlayerJoin decree process
    0x75DA80 -> 0x758FD0 (session full 0x76CCA0, CreatePlayer 0x773710,
    rated flag 0x770280, PostPlayerJoin request 0x74CA80) - native-lobby.json,
    44 join-message + 27 PlayerJoin + 9 address cases on both images,
    tests/lobby.rs, research/phase8-net/lobby-notes.txt. Three PlayerJoin
    cases (duplicate / empty / NUL names, rule 0x766560) have no native
    evidence: the oracle harness stopped on its own player objects; they are
    listed in the test and unproven.
  * synchronizer resume (round 3): native-join-resume.json, the original
    host request dispatch 0x74D710 over a request packet holding a
    PlayerJoin request among speed requests, and the original client replay
    0x74FD10 of the decree packet, resumed at the captured AdminNode+0x200 /
    +0x204 and from the packet start (4 cases); tests/join_resume.rs replays
    it with aa_orders bit-exactly (decree bytes, every A and S entry with its
    hash, processes, capture);
  * send selector 0x763EE0 (src/send.rs select_connection) -
    native-send-selector.json, 400 cases, tests/send_selector.rs;
  * staging (src/staging.rs): readiness 0x765FC0 / 0x76C960, the launch
    countdown processes 0x774A50 / 0x772350 -> 0x76CFA0 / 0x7723F0 with the
    rejection 0x76DA60 and request 0x76CBF0 (descriptor 325 bytes compared),
    KKC_Session::Launch 0x77A950, ChangeToState 0x77A750 with 0x77A6E0 and
    the event construction, AutoAssignKingdomToPlayer 0x778D10 with
    0x7664D0 - native-launch.json, 700 cases, tests/staging.rs;
  * ClientLeave 0x755840 (heartbeat::AdminNode::client_leave) with the
    vote-kick sweep 0x74F160 (0x74D380 / 0x74D320, recursion), vote removal
    0x74EF90, task check 0x751F70 / 0x74DA50 / ClearAllTasks 0x74DAA0, list
    removal 0x4F1430, SetDisconnecting stores and the session player loop
    with DestroyPlayer 0x7737E0 (lobby::destroy_link_players) -
    native-leave.json, 400 cases (352 complete, 48 native null reads that
    the port reports as typed errors: a voter peer without a player
    0x74D3A9, a "player left" message for a connection without a player
    0x7667D0), tests/heartbeat.rs native_client_leave. 0x74F160 reads the
    next link of a list node that the recursive ClientLeave just freed; the
    engine pool free 0x85C2B0 only clears a bitmap bit, so the port follows
    the link the node had when it was unlinked (AdminNode::removed_links;
    the oracle's free is a no-op seam with the same effect).
Compiled and composition-tested:
  * NetworkSeams adapter (src/seams.rs; aa-runtime trait, aa-orders core);
  * UDP layer src/udp.rs (std::net, transport port, no caps);
  * heartbeat channel effects mapped onto packets.rs channels
    (tests/channel_effects.rs: same channel state as the proven bindings);
  * LOOPBACK ONLY (tests/loopback.rs): host and client in two threads with
    two real UDP sockets on 127.0.0.1, once directly and once through a
    relay that drops every fifth and delays every eleventh datagram per
    direction: Connect -> PlayerJoin request/decree -> type 5/4 channels ->
    the client replays every host decree packet with identical decrees,
    fields and order, and every TGC_SetSyncValueOrderRequest the client
    sends is validated by the host's synchronizer A (0x751D00 decision with
    0x77D050) - all valid; a deliberately offset client synchronizer makes
    every validation fail. Round 3: the join runs through lobby.rs (PlayerJoin
    process 0x758FD0, host join completion 0x753580 with the session
    snapshot and the admin-state message, client 0x7531A0 -> 0x74DF50 /
    0x77AAC0); the host ticks (and sends decrees) before the client
    connects, and the client resumes at the admin-state pair inside the
    packet holding its own PlayerJoin decree. Remaining stand-ins: the order
    payload context (tests' Ctx), the SnapshotHost / JoinHost traits (no
    ChangeToState / kingdom data), no file transfer content. This is not a
    two-machine test.
Synchronizer resume (round 3, settled by native-join-resume.json):
AdminNode+0x1F8 is the decree stream object {vtable, buffer, byte, bit};
+0x200 / +0x204 are its byte and bit fields (not separate members), so the
admin-state message 0x74C130 sends the decree-stream position at the moment
0x753580 runs inside the PlayerJoin process: right after the joiner's
PlayerJoin decree. Synchronizer A (index, hash) is captured at the same
point. The PlayerJoin request handler omits the trailing "o.GetBitSize()"
entry on the host (aa_orders flow.rs, `trailing_bits_log`) and the client
resumes 0x74D710 at (+0x248, +0x24C)
skipping its "i.GetBitSize()", so the client's entries from there on are
exactly the host's. The earlier "(0, 3) only" reading was the stream reset
of the same fields.
Not ported: the disconnect dialog, GameSpy/NAT negotiation (out of scope),
SimulatedNetEffects, KKC_LaunchSettings::operator= 0x779F00, the kingdom
setters 0x777F00 / 0x777CD0 / 0x777610, the not-ready report 0x7725A0, the
session task handlers 0x775BF0 / 0x773AE0 / 0x76EE10 (traits / effects).
x87: network/simulation threads at PC53 (lead policy); every float
statement ported here takes the precision explicitly.
Untested: any real two-machine session. No claim of working multiplayer.
