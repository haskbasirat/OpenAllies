First-tick native music selection (4F96D0)
========================================

Scope
-----
This is the original AA first-tick SELECTOR over the retained local Player,
Kingdom and source definitions. It is not a new playlist policy or a completed
Miles/SoundManager backend. It does not establish BeginPlaying, tick readiness,
whole map restoration, audible playback or a playable battle.

Production owner/API
--------------------
crates/aa-runtime/src/live_first_tick.rs:
  Source::bind(Catalog, actual moved Bank, actual Settings, V14Sources)
  Context implements live_clocked::FirstTick over actual SessionRead/ActorStore.
  MusicRequests retains request/delivery effects; AudioOutput::play_747630 is
  REQUIRED synchronously at the actual native call point. Enqueue alone cannot
  complete the call. UnboundAudioOutput refuses the reached service.

The Source requires the same completed source State and its native source PC,
current logical source identities, actual Nation/Audio registries and the real
ObserverGameMusic declaration. Re-parsing source files rejects forged public
Settings raw text/hash/span. The controlled fresh-source AVar is supported;
later setter/profile overrides remain explicit unsupported owner requirements.
The actual Player+28 getter remains distinct from an unbound getter and from a
genuinely bound null Kingdom. FirstTick never infers a Kingdom from Player+24.
The netcode owner supplies the real completed765E10 binding at BeginPlaying.

Native selector order
--------------------
4F96D0 calls76AC60, then765E00. The latter returns null when current GWorld is
null; with current GWorld present a null Player dereferences null at765E0C.
For a nonnull Kingdom, read+234 Nation. A null Nation returns immediately and
does not fall through to observer music. A nonnull Nation selects its+38 Audio.
For a null Kingdom,410CB0 reads current ObserverGameMusic. Empty UTF16 skips
lookup/output. A nonempty string is looked up by416370 in the actual Audio
manager; missing Audio reaches the required410C40 fatal path. A selected Audio
calls747630 on B36548; the caller ignores that sound-instance ID return.

The production source binder resolves Nation game_music and Audio IDS/category/
files/leadin/fallout through the canonical registered typed source view. Every
path retains its typed provenance. The admitted clip-list domain is literal
comma-separated members, with no guessed formatting, cleanup, fallback or
playlist shuffle. Request storage is an application-owned acknowledgement
queue; it is NOT a claim to port the native internal message/voice structures.
Accepted/Declined are real callback outcomes, not evidence of audible playback.

The source-to-Nation+38 relation has its own native prefix oracle:
native_nation_music.py executes48C050 through the next placement_group_IDS
lookup (after the actual48C103 store). Thirty-six cases/56 executed pins cover
original/v14/all PCs: known Audio, empty string, unknown Audio fatal, required
game_music failure, duplicate Nation guard, and embedded-NUL prefix. The real
duplicate guard and loaded+14 byte store execute. Name/string/registry/fatal
services are explicit seams;+38 remains its seeded old word on the required
error/fatal paths. This is neither the full Nation loader nor a certificate for
arbitrary later mutation of its current field. The callback source binding is
the controlled fresh immutable Nation-definition path.

Native oracle
-------------
native_first_tick.py executes actual4F96D0,76AC60 and765E00 from both pinned
original and patched-v14 images at all three x87 precision controls. Seventy-two
rows and75 executed nonleaf instruction pins cover twelve cases per image/PC:
Nation Audio; null Nation; null Nation Audio; observer known; observer empty;
observer unknown fatal; absent GWorld/null Player; present GWorld/null Player;
required observer error; required Nation output error; required observer output
error; and embedded-NUL observer prefix.410CB0 AVar,416370 registry,410C40 fatal
and747630 output are DECLARED seams. Shared CString lifetime/allocation leaves
are bounded seams. Injected service errors prove prefix/order, not native OS
exception/unwind semantics. This oracle does not prove profile parsing, native
AudioData loading, BeginPlaying, hardware audio or the full747630 backend.

Read-only747630 investigation (not ported)
----------------------------------------
747630 asks manager virtual+58 for an instance; null returns0. Otherwise it
sets instance+38, calls7471E0, and on success queues [0,instance,0,0,0] through
746E70 then returns instance+C. Failure may clear attached Voice state, sets
instance+39, disposes through manager+5C, and returns the existing+C word.
746E70 owns critical-section+48 and appends native0x18 message nodes.
The base ctor746470 installs B36548; the actual Miles ctor749130 overwrites
the vtable. Its+58 points7482A0 (device-ready check, allocate0x74,7457D0,
vtableA8B2D0, AudioData+4);+5C points748330. The7471E0 gate additionally uses
driver/channel/time/debug/overlap services and is NOT covered by the selector.
Therefore no claim is made about native voice/channel capacity, random track
selection, fades, cannibalization or full SoundManager error equivalence.

The existing aa-client Audio::play_music(exact canonical IDS) is an available
platform adapter boundary. Audio::play_nation_music contains fallback selection
and is unsuitable for this exact selector. Actual client wiring and device
playback must be tested separately; the current Rust source test intentionally
uses a laboratory output and explicitly bound-null laboratory SessionRead.

Replay
------
From rust-rebuild, PowerShell:
  $env:PYTHONPATH='E:/Games/Axis and Alies/reconstruction/tools/python-packages'
  python research/development10/live-first-tick/native_first_tick.py
  python research/development10/live-first-tick/native_nation_music.py
The producer compares the frozen fixture by default; --write regenerates it.
  $env:CARGO_TARGET_DIR='target/s-settlement-completion'
  $env:AA_GAME_ROOT='E:/Games/Axis and Alies'
  $env:AA_FIRST_TICK_SOURCE_REPORT='E:/Games/Axis and Alies/rust-rebuild/research/development10/live-first-tick/actual-source-inputs.json'
  cargo test -p aa-runtime --release --locked --offline -j4 --lib live_first_tick::tests -- --include-ignored

Validation results and exact final hashes are recorded separately after the
final focused replay. Native input/output fixtures and superseded logs remain
retained. No user profile, credentials, original binary or original asset was
changed by this work.

Retained first replay:
tests-final-v1.log compiled the frozen production and ran three tests. Native
selection and required-callback/error storage passed. The real-source test
failed because its test-only Nation IDs used short country codes instead of
the actual declared nation_usa/nation_gbr/nation_ger/nation_rus/nation_jpn IDs.
Those test literals/expected music suffixes were corrected; production source
was unchanged. The failure is retained and is not counted as acceptance.
tests-final-v2.log is a superseded concurrent build snapshot from the mutable
ZoneMap API transition in load_world.rs130/148. Root corrected those borrows.
tests-final-v3.log then saw the in-flight load_actor_overlaps test borrow and
load_detection_activation generic Effects bound. Those are separate authors'
shared changes, not FirstTick test failures; neither snapshot is acceptance.
tests-final-v4.log compiled the final hashes and all behavior assertions passed,
but its final optional report write failed: Cargo runs unit tests from the
crate folder, so the supplied workspace-relative output path did not exist.
The command now uses an absolute report path. tests-final-v5.log reran that
SAME release binary directly with the absolute path and all three tests passed.
No source change or new compilation was needed for that reporting correction.

Final author acceptance:3/3 tests, native72 selector rows/75 pins plus36 Nation
loader-prefix rows/56 pins. Actual source bindings include all five declared
Nations and their20-member authored ambient lists, plus the six-member observer
list. Their repetitions/order are preserved as source input; no shuffle/voice/
hardware playback semantics are inferred. See findings.json/final-freeze.json.
