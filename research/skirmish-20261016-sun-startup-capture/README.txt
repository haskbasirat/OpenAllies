Sun constructor startup input, 2026-10-16

The fresh isolated Windows/v14 capture observed B2814C/B28150/B28154 as
[0,0,0] from CREATE_PROCESS through source_return and the actual Session
constructor return. These are initialized writable .data bytes in both pinned
images, not the PE zero-fill tail. The capture itself did NOT reach Sun
constructor4EE0A0, a map or a battle. Sun's constructor/read-only exact-address
reference audit is separately owned by the sun_state implementation.

All62 created threads received three aligned4-byte WRITE watches, verified by
reading back DR registers. No write to those12 bytes occurred before the sealed
observation cutoff. Three same-value isolated native MOV instructions calibrate
the three slots AFTER that cutoff, with other threads suspended; they are tracer
probes, not game producers. No FP/SSE context was written. The owned PID50308
was terminated/drained normally (wait0,exit0,EXIT_PROCESS,errors[]), and no
AA.exe remained. Original archives and all54 independently copied support
files remain byte-identical. All1028 ActorData rows/CV/masks, eight means,
source FP controls/status and cached numeric words match frozen capture03.

capture_sun_startup.py is a distinct copy of the successful, frozen point
tracer; prepare_tracer.py records its narrow changes. Old capture03 and both
point captures were not changed. Like the base, its two original CreateProcess
handles live until the bounded Python exits; owned duplicates close explicitly.
Inherited setup checks precede the main cleanup try. Successful receipt claims
require normal final cleanup. The optional Sun constructor breakpoint was not
reached; a guessed constructor observation is never inferred from its presence.

seal_sun_reference.py independently verifies the trace, source/FP comparison,
all54 support copies, image bytes and scope before writing a NEW compact
sun-startup-reference.json (exclusive creation prevents replacement).

aa-runtime::load_sun_reference::ConstructorInput has private observed words and
source FP context. from_verified_startup_reference checks the sealed source
receipt, source owner/fingerprint, current numeric target and compact Sun
reference. create_sun(self,&Catalog) checks the current destination again and
consumes the input to call the separate proven sun_state::State::new. There is
no raw seed getter, Default, Clone or zero fallback. Root retains the returned
actual Sun and applies saved Load to that same object. This is a target-bound
startup seed, without an arbitrary future native immutability claim.

Commands from rust-rebuild:
python -B research/skirmish-20261016-sun-startup-capture/prepare_tracer.py
python -B research/skirmish-20261016-sun-startup-capture/capture_sun_startup.py --folder research/skirmish-20261016-sun-startup-capture/capture-01 --seconds 120
python -B research/skirmish-20261016-sun-startup-capture/seal_sun_reference.py
$env:AA_GAME_ROOT='E:/Games/Axis and Alies'
$env:CARGO_TARGET_DIR='target/s-detection-mask'
cargo test -p aa-runtime --test load_sun_reference --release --locked --offline -j4 --config profile.release.lto=false --config profile.release.codegen-units=16 -- --include-ignored

See runtime-tests.log and findings.json for completed focused test results and
pins. The release overrides are explicit; this is not a full-workspace or
final executable validation. No original game, profile or release package changed.
