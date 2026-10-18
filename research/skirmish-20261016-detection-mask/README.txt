Detection component-mask correction, 2026-10-16

Problem and change
------------------
Actual Windows/v14 capture03 reported ActorData+174 = 1090145 for minefield
and beach_mine_pole. The previous binder produced 5284449, retaining Detection
bit22 (0x400000). This is a producer-condition error, not an authored exception.

Original464BD0 reaches Detection at465011. It sets bit22 when the node exists,
loads detection_range, loads remote_detection_range, then clears bit22 exactly
when BOTH current ActorData+218 and the retained remote value are ordered <=0.
The FNSTSW/TEST AH,41/JP sequence at465061..465079 includes equality; greater
or unordered retains the bit. Both positive and negative zero therefore clear
on a fresh object with remote zero. Constructor4624B0 has xorEBX,EBX at4624E1
and stores EBX to+218 at462677. Detection loads before Defender writes the
authored control_range. Later Defender control_range=5 cannot restore bit22.

native.rs now exposes native_component_mask_at(id,count,precision), retaining
the existing method as a documented PC53 target wrapper. The mask projection
reads the assigned typed Fixed remote value only when Detection exists. It
keeps authored component provenance even when the effective bit is cleared.
Missing/unassigned/wrong-type remote values remain explicit errors. Fresh
actor creation and callers already holding precision propagate that value.

Proof and scope
---------------
native_detection_mask.py executes original465011..465089 at PC24/53/64 with
explicit component-lookup and binary32 Fixed-getter seams. No full464BD0 load
is claimed. It covers2331 rows:2304 present-node branch probes and27 absent
node probes. Inputs include negative/positive finite values, subnormals, both
zeros, infinities and NaNs, with3 initial masks. General nonzero+218 rows are
isolated existing-object branch probes, not fresh-load source contexts.
All reached native instructions and the constructor span match original/v14.
Page zero is unmapped; lookup/getter stubs do not execute upstream tools.

NaN rows establish the unordered mask decision only. They do not establish
signaling-NaN quieting, payload, exception, FPU-status or getter-type parity.
The helper's numeric domain is binary32 typed Fixed getter returns. Component
mask binding remains a fresh-definition projection, not a complete loader:
it does not certify unrelated field reads, native partial error writes,
arbitrary reload side effects or all definition-derived classes.

Rust native_detection_mask.rs replays every row and checks source binding,
absence/lazy lookup, wrong types, provenance, and fresh constructor ordering.
Its real-data test loads original Data.rwd plus accepted v14 overrides and
checks both mine masks at all3 PCs. The exact hardware observation itself is
PC53; the allPC generalization additionally rests on the native fragment.

Commands (PowerShell, from rust-rebuild)
---------------------------------------
$env:PYTHONPATH='E:/Games/Axis and Alies/reconstruction/tools/python-packages'
python -B research/skirmish-20261016-detection-mask/native_detection_mask.py
$env:AA_GAME_ROOT='E:/Games/Axis and Alies'
$env:CARGO_TARGET_DIR='target/s-detection-mask'
cargo test -p aa-rules --test native_detection_mask --release --locked --offline -j4 -- --include-ignored
cargo test -p aa-rules --release --locked --offline -j4 --config profile.release.lto=false --config profile.release.codegen-units=16 -- --include-ignored
cargo test -p aa-metagame --release --locked --offline -j4 --config profile.release.lto=false --config profile.release.codegen-units=16 -- --include-ignored

See focused-tests.log, all-rules-tests.log and all-metagame-tests.log for the
actual final outcomes; findings.json pins completed evidence. The successful
Windows receipt remains unchanged at
../development10/native-source-capture-03/capture.json. Older full combat
source snapshots with the two wrong masks remain historical; the CV author
owns a new v3 source capture and replay rather than silently replacing them.

The first full rules build used default release thinLTO/codegen-units1, but
another source owner hardened source_requirements while it compiled. It was
stopped using its verified owned Cargo process tree; its incomplete log and
aborted-before-source-freeze.json are preserved, with no success claim. The
next full rules run completed successfully, but another coordinated complete
source-root guard landed after its compilation. That successful older-revision
log is preserved as all-rules-tests.earlier-source-root-guard.log, with its
earlier source snapshot. It does not certify the final guard. The final full
checks explicitly use optimized release with LTO disabled and16
codegen units to shorten test linking. This changes no manifest or source.
Focused Detection tests passed the default release profile. The final game
executable's release profile remains the parent's separate check. All rules
and metagame Rust source hashes before/after the checks must agree. No separate independent
reviewer acceptance is claimed for this bounded Detection correction.

No game assets, original executable, release, Cargo manifests or lock changed.
Backups preserve the two owned pre-edit source files. This fixes a proven
source-mask discrepancy; it does not publish a playable battle or multiplayer.
