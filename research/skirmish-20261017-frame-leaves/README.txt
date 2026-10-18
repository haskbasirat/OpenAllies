Frame leaves:75B820 heartbeat,41C180 timing,752AE0 duration,76F600 guard
===================================================================

Production ownership
--------------------
The application retains one presentation_timing::State and one process-global
B40934 heartbeat Counter in restore_lifecycle::Sai, moving both into the SAME
StartupOwner. Frame::with_process_effects lends those owners; neither is made
per frame, mission or Node. The direct owner accessor grants no activation.

The actual local heartbeat borrows the existing Node backup request/decree
histories, cursor, flags, vote, ratio60, real player/host links and process
counter. It reuses the existing heartbeat coordinator, status codec and decree
cursor algorithm. No second mutable heartbeat Node or copied PeerBase is made.
The pre-existing packet laboratory keeps its historical diagnostic omission;
the concrete application requires the diagnostic service at native404F90.

The whole752AE0 wrapper samples the SAME retained NodeTime/QPC timer before
and after76F790. Its duration appends to the existing raw DWORD TaskHistory238,
which task315 also clears/retires. It updates the same Node ratio60 after at
least10 samples and keeps the native16-sample window. There is no second ring
or timing ratio. The history can grow; no native fixed allocation is a cap.
Unsupported unretired negative/overflow windows return a required error.

The borrowed frame kernel brackets76F790 after recorder/playback effects and
before AVI/logging/smoothing, and commits duration only after World update
and end-condition success. The old laboratory/direct Driver fixtures explicitly
exclude this outer wrapper; their Host adapter returns None for that scope.
The live Host always samples a real clock. An unrelated ClockHost refuses it.

The concrete end-condition adapter reads actual Session16D lazily. False
returns without reading16E/World/player state. True stops at required76BA90;
it does not invent human opposition or a UI notification.

Native proofs (original and accepted v14, PC24/53/64)
----------------------------------------------------
native_presentation_timing.py:3048 finite arithmetic/error rows,6 whole41C1F0
constructor executions,51 genuine executed instruction pins.866060 is an
explicit binary64 ST0 input seam. Time failure precedes writes; factor failure
preserves the15C time store. The comparison uses the unspilled smoothed value.
The constructor also checks the original process B40934 word is zero.

native_local_heartbeat.py:120 input rows,330 actual75B820 pulses,1045 genuine
executed pins. Includes36 consecutive pulses/two status publications, raw
noncanonical status bytes, lazy diagnostics, and diagnostic failure after the
packet/counter prefix. Initial current fields are typed laboratory inputs;
this is not a whole Host/session construction certificate. No remote peer,
takeover, request-packet or actual frontend diagnostic implementation is claimed.

native_frame_duration.py:792 rows,1002 actual752AE0 wrappers,313 genuine pins.
Two59ABA0 results and the76F790 child are explicit typed seams. Covers unsigned
millisecond conversion/wrap, unspilled second timestamp, per-add binary32 sum
stores,10-sample threshold,16-sample retirement, growth and clamping. Failure
at either timer or World child retains the exact earlier owner state. No whole
World update or OS QPC invocation is certified by this isolated wrapper proof.

native_frame_end_condition.py:36 rows,17 genuine pins. Actual76F600 false path
and precise positive76BA90 entry only. The supplied later16E byte is untouched.

Each script defaults to immutable replay. --write is an explicit author-only
regeneration mode. Replaced seam entries are excluded from executed pins.
No original executable, game profile or save was changed.

Rust verification
-----------------
Exact presentation_timing production source isolated replay:2/2 passed,
3048 rows+6 constructors, timing-isolated-tests.log. No runtime unit relink.
Current aa-netcode production+all test typecheck passed37.25s in
check-frame-leaves-final-v2.log. Earlier failed logs preserve shared in-flight
ActorAccess/Economy compiler snapshots; they are not successful checks.
Root's shared cohort is responsible for executing the new5 local tests and
the existing external heartbeat/borrowed_driver regressions at frozen source.
Do not promote typecheck or this isolated native proof into whole-frame success.

Remaining reached services
--------------------------
The live frontend must loan SAME Frame timing/counter into its hooks and bind
an actual retained404F90 diagnostics sink. Every17th server pulse publishes a
status packet; this must not be skipped. Remote status/chat/file/takeover and
positive backup requests remain required where reached.

41C1F0 initializes smoothing byte4 disabled. Disabled updates need no time or
AVar. A future positive enabling writer needs proof and actual866060 Ni timer
plus current AVar68 ownership. The Node timer is not a replacement for866060.
The enabled76F600 branch still needs real HumanOpposition/World48/localization/
UI/GameSpy owners. None of these proofs grants activation, rendering readiness
or a controllable battle certificate.
