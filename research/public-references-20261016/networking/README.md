# Public networking references — October 16, 2026

## Result

These sources can reduce protocol investigation time. They do not supply
TimeGate's session/gameplay implementation or make the current Rust build
Internet-playable. No downloaded source was built, installed, executed, or
integrated. There were no account, host advertisement, system, or server changes.

Downloaded material is isolated in
`E:\Games\Axis and Alies\reconstruction\references\public-rewrite-20261016\networking`.
Its `download-manifest.json` pins 44 selected source/text files to exact commits,
Git blob SHA1 identities and SHA256 hashes. Repository/commit/tree and issue API
snapshots are separate metadata, not additional engine/source packages.

## Versions and notices

| Source | Pinned revision | What this establishes |
|---|---|---|
| [nitrocaster/GameSpy](https://github.com/nitrocaster/GameSpy/tree/d1deb2d1a951cf77933dda040b8d311cc09815a7) | `d1deb2d1a951cf77933dda040b8d311cc09815a7`, committed January 29, 2016 | Public SDK source mirror; the Common changelog ends at 2.06.00 and Peer at 2.07.00, both December 12, 2007. Later than AA 2004. |
| [OpenSpy Core](https://github.com/openspy/openspy-core/tree/221c6a7931914362a04ffe6f374957eff2fe1bac) | `221c6a7931914362a04ffe6f374957eff2fe1bac`, committed January 4, 2026 | Server protocol implementation at a fixed revision; it does not establish the public deployment's exact binaries/configuration. |

The GameSpy root `license.txt` grants source/binary redistribution and modification
subject to notice retention and non-endorsement conditions (BSD-3-Clause text).
Selected original file copyright headers are retained. This is separate from
the agreement-restricted Gamebryo archives. Only selected files were downloaded;
this is not a complete, buildable SDK or a certification of every dependency.

OpenSpy Core's current tree has no project LICENSE/COPYING file; the captured
GitHub repository API reports `license: null`. A project-wide reuse/distribution
grant was not verified. Public availability is useful for protocol inspection;
do not describe its code as license-cleared for incorporation. Neither source
was copied into production Rust.

The exact GameSpy SDK build linked into AA remains unverified. No general
drop-in or binary compatibility claim follows from similar API names. In
particular the mirrored SDK uses `NN_PROTVER 3`, while OpenSpy's current header
declares 4 and its handlers inspect request versions. The shared six-byte magic
does not prove AA's version, helper behavior, retry timing, or interoperability.

## Useful source paths and the precise rewrite gaps

Paths below are relative to the pinned repositories and also preserved locally.

| Area | Useful public files | Existing Rust and remaining work |
|---|---|---|
| Presence/login | GameSpy `src/GameSpy/GP/gpiConnect.c`, `GP/gp.h`; OpenSpy `code/GP/server/GPPeer.cpp` | GP namespace/product fields, challenge and connect framing can be studied. `aa-netcode` has no actual GP account/login connection. Match AA's unique-nick path and callbacks against original code; no account login was attempted here. |
| Lobby/chat | GameSpy `Peer/peerMain.c`, `peerRooms.c`, `Chat/chatHandlers.c`, `chatCrypt.c`; OpenSpy `peerchat/server/commands/handle_crypt.cpp` | IRC-derived GameSpy chat and title/group/staging flows are reference leads. Existing `aa-netcode::lobby` is TimeGate session snapshot/join logic, not a peerchat client. Connection state, crypt, room membership, callbacks and UI integration remain separate work. |
| Browse/group lists | GameSpy `serverbrowsing/sb_serverlist.c`, `sb_crypt.c`, `Peer/peerSB.c`; OpenSpy `serverbrowsing/server/V2Peer.cpp`, `tasks/GetGroups.cpp` | Documents `%s.ms%d` host selection, enctypex list parsing/encryption and groups. The Rust UDP admin layer does not implement the SBv2 master connection or its listing lifecycle. |
| Advertise a host | GameSpy `qr2/qr2.c`, `Peer/peerQR.c`; OpenSpy `qr/tasks/Heartbeat.cpp`, `GetGameInfo.cpp` | QR heartbeat/challenge, query keys, game/query address and group association need an actual game-side adapter. Successfully seeing group names is not advertisement or remote join. Preserve AA's reported query port 8850 and host port 7290. |
| NAT traversal | GameSpy `natneg/natneg.c`, `nninternal.h`; OpenSpy `natneg/server/structs.h`, `handlers/handle_init.cpp`, `handle_connect_ack.cpp` | Cookie, partner roles, packets and retries are useful leads. `aa-netcode` currently has no NAT-negotiation client. Observe AA's actual protocol version/call order before implementing; do not copy the later SDK's retry policy as game behavior. |
| HTTP catalog/ranks | GameSpy `ghttp/ghttpMain.c`, `ghttpProcess.c` | HTTP callbacks/pumping can inform a service adapter. AA-specific completion/parsers are TimeGate code; the SDK does not provide the approved AA map list or historical rank/scoring service. |
| Match, save transfer and deterministic orders | No replacement supplied by these SDK files | `aa-netcode::{session,lobby,staging,lobby_staging,launch_assignment,file,packets,channel,heartbeat,seams,udp}` already contain native-backed portions. The public SDK is not the authority for TimeGate's wire codecs, save bytes, synchronization or v14 exact-block repair. |

The current `lobby_staging::Effects` still requires real world loading,
transport/UI effects, timers, launch steps and state notifications. `MapStatus`
requires actual map/save status, checksum and I/O. `SessionSeams` requires the
real network/command pump, clock, rate/heartbeat and backup/film services. Those
traits have no permissive success defaults. `UdpNode` provides sockets and
native datagram sequencing; a complete game/session coordinator remains needed.

The historical network evidence includes local UDP host/client and lossy relay
composition, not a playable world or two real Internet peers. Later settings
and staging adapters have progressed beyond some historical README gap entries;
the current source is authoritative. No networking tests were rerun in this
source-reference-only task.

## Issue 33: the official-map gate is not an SB map-loop feature

[Issue 33](https://github.com/openspy/openspy-core/issues/33) remains open with
zero comments in the fresh API snapshot. It proposes missing game registration
and infers a non-HTTP map request from a failed older URL experiment. Those are
the reporter's hypotheses, not maintainer-confirmed findings.

The original `AVars_AA.tgi` lists `aarts`, game 868, product 10318, namespace 1
and query port 8850. `AVars_version.tgi` overrides the older SDK-development URL
with `http://gamestats.gamespy.com/aarts/maps.txt`. Native `78A0F0` selects
`GameSpyRatedMapsURL` for request kind 2 and queues the URL request; `79CE10`
parses the completed content. Existing native patch/evidence already traced the
GHTTP callback and completion gate. A request to a different overridden hostname
cannot exclude HTTP.

OpenSpy `V2Peer.cpp` separately recognizes `MAPLOOP_REQUEST` without implementing
its response. That does not demonstrate that AA's official-map gate needs that
command. Game registration/group retrieval also differs from HTTP content.
The October 11 preserved read-only SBv2 evidence successfully decoded `aarts`
and `aartsd` groups, with a separate valid empty server list. Therefore a new
public game-registration row was not demonstrated as necessary. Those probes
were not repeated here and do not prove today's service health or host/join.

Fresh credential-free GET checks at 20:31 EDT, October 16, returned **HTTP 404,
empty body**, for both `gamestats.openspy.net/aarts/maps.txt` and `/aarts/ranks.txt`.
See the new `read-only-http-probes.json`. No login, QR heartbeat, or NAT request
was sent by this review.

## Exact server requirement, if original rated services are wanted

1. Serve successful plain-text HTTP at the configured map URL. Native records
   are separated by semicolons; each record contains map identity, comma,
   signed decimal checksum. The native parser trims tokens, takes the absolute
   signed value, and ignores zero checksums. Identities/checksums must match
   AA's `79CB60` comparison. EOF completes the catalog even when empty. An empty
   catalog marks no map approved for rated play; it is an unranked fallback.
2. If rank retrieval is enabled, supply the exact loaded number of
   nondecreasing comma-separated thresholds. This installation has five ranks;
   shipped local thresholds are `1,50,250,500,1000`. They are not established as
   the historical online table. An empty rank response fails `49FA30`'s count
   check and is not a safe substitute.
3. Rated play also requires the AA-specific stats/session/persistence schema,
   validation and scoring/aggregation pipeline, followed by actual submission
   and retrieval tests. Serving two text files does not provide ranked service.
4. If operating a separate deployment without AA records, populate the game's
   recognized name/ID/key and metadata, group data and corresponding Redis game
   and group caches. `OpenSpy.cpp::GetGameByName` reads the lowercased gamename
   mapping and `gameid/secretkey/description/gamename/disabled_services/queryport/backendflags`.
   `V2Peer::OnRecievedGameInfoPair` rejects a missing source/target key. This is
   conditional setup for a separate deployment, not evidence that public
   OpenSpy needs new `aarts` registration.

The accepted native v14 work already provides a reviewed **unranked empty-map
completion lifecycle**, preserving auth/readiness and disabling fabricated
ranked results. That native patch has not thereby been ported into the current
Rust rewrite. Existing game/save protocols and catalog approval checks must
remain intact. The detailed prior contract is in
`reconstruction/native-game/mods/openspy/research/server/SERVER-REQUIREMENTS.txt`,
with byte-level evidence pinned in this report's findings.

## Internet and free-traffic limits

OpenSpy's NAT service is IPv4 rendezvous, not evidence of a universal gameplay
relay. Its documented full deployment uses three public IPs plus helper/AMQP
coordination. It cannot guarantee every symmetric-NAT/CGNAT/firewall pair can
connect without forwarding. Free public matchmaking does not imply free
unbounded relay bandwidth. A relay fallback would require its own explicit
protocol/hosting choice and real reachability tests; none was deployed here.

Remaining acceptance requires original native/client packet comparisons,
then two Windows PCs on separate Internet connections: profile/login, chat and
lobby listing, QR advertisement, NAT negotiation, remote join, map/save transfer
with hashes, complete deterministic match, disconnect/reconnect/migration and
error handling. The independent connection is unavailable in the user-provided
test setup, so those tests remain blocked. Same-Wi-Fi direct-room joining was a
historical user report, not proof of WAN traversal or transfer performance.
