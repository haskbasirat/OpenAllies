Borrowed kingdom resource-flow arithmetic, 2026-10-16

Why this addition exists
FlowArithmetic is the existing tick-prefix result structure, not a way to
mutate native flow storage. Economy owns a full State and its mutation methods
clone that State, then commit on success. That laboratory behavior cannot
represent an actual SpecialShell write prefix when a later dependency fails.
The new aa-economy::borrowed_flows module therefore takes required World
getters/setters over the caller's actual storage. Existing Economy and all
prior arithmetic fixtures remain unchanged. There is no flow cloning,
replacement, resizing, zero initialization or rollback in the new module.

Native functions and exact order
4E48C0 captures resource-manager B3A660+28, then per ascending resource:
  read/spill signed input delta, remove shortages, update consumed+1B8,
  apply shortages. Even zero deltas perform both shortage passes.
4E4930 captures catalog B36530+38, reads/spills each signed input delta,
  then calls4E3940 for that index. The two loop counts are distinct inputs.
4E3940 removes shortages, adds delta to raw+1AC and stores binary32,
  reads ResourceDefinition+2C, optionally reads the signed integer limit
  through SVar cache B39AB0+1B4, compares exact FILD integer with raw,
  stores effective+1A0, then reapplies shortages. Missing limit preserves
  the preceding raw store. The unlimited branch copies the raw32 bits.
4E1F60 uses live Kingdom+1A4, rereading it after every destination and source
  iteration. Ascending source: effective[source]-consumed[source] at selected
  x87 precision; FST binary32 spill; compare the still-extended difference.
  Negative Upkeep sources iterate ascending Stockpiled destinations. Negate
  the spilled difference, multiply the source purchase coefficient, add or
  subtract the current destination consumed value, then store binary32.
The helpers never allocate or grow kingdom arrays. Empty/uninitialized
storage must not become manufactured zeros. Correct per-record Init timing
and concrete source-initialized array lengths belong to the runtime adapter.

Public API
  borrowed_flows::{shortage, adjust_provider, adjust_providers, adjust_consumption}
All accept &mut impl World and explicit X87Precision; vector methods also
accept &[F32Bits] and add:bool. The source input vector must be distinct
actor-component storage, as at the saved Economy refresh caller.
Results use EconomyError. The World trait requires all actual counts, flow
reads/writes, resource kind, purchase cost, provider flag and lazy maximum
setting. Missing entries/settings/source facts remain errors when reached.
The native code has no dependency-error rollback, and neither does this API.
Errors from a setter retain whatever prefix that setter actually performed.
Negative signed counts naturally execute no loop, matching native branches.

Numeric scope
Uses the existing integer dyadic Binary arithmetic with explicit PC24/53/64
and nearest-even rounding, including native intermediate binary32 stores.
Finite binary32 operands/results are supported. Nonfinite input or a result
that would store infinity is an explicit unsupported arithmetic boundary;
earlier flow stores remain applied, but the unsupported result is not stored.
This does NOT claim that native code throws on NaN/infinity/overflow. FPU
exception flags, alternate rounding modes and aliasing flow/input vectors
are outside this API's proven scope. No new resource/gameplay cap or RNG is
introduced; the existing limited-provider rule is preserved as supplied.

Evidence and results
native_borrowed_flows.py independently loads original and v14 and executes
all four unchanged functions at PC24/53/64. Cases reuse the preserved phase2
native fixture's prior state for each relevant operation, so earlier rounding
history is retained separately per precision. Results are also checked against
that fixture when applicable. New cases cover distinct/zero/negative counts,
missing second input, missing consumed storage after an empty shortage pass,
missing cap after raw update, missing coefficients and pre-store failures.
Injected failures stop BEFORE native memory instructions; they are explicit
adapter-dependency probes, not claimed native exceptions.
The fixture contains1026 rows and records every write to all three flow
arrays, including writes that leave bits unchanged. Native function windows,
both full image hashes and producer/base-fixture hashes are pinned. Default
oracle replay compares successfully; --write is the only export switch.
Three Rust tests pass:1026-row state/store-prefix replay, unreached invalid
input-tail handling, and finite-domain overflow preserving earlier writes.
The replay world's arrays are borrowed slices; allocation identity is checked.
Existing aa-economy native_parity regression results are in their separate log.
Actual SpecialShell source binding and Kingdom Init scheduling are owned by
the runtime integrator; no live match or completed restoration claim is made.

Commands
  CARGO_TARGET_DIR=target/s-borrowed-flows cargo test -p aa-economy --test native_borrowed_flows --release --locked --offline -j4
  CARGO_TARGET_DIR=target/s-borrowed-flows cargo test -p aa-economy --test native_parity --release --locked --offline -j4
  PYTHONPATH=<game root>/reconstruction/tools/python-packages python research/skirmish-20261016-borrowed-flows/native_borrowed_flows.py
See pure-tests.log, existing-arithmetic-tests.log, native-run.log,
native-replay.log, native.asm.txt and findings.json.

Preservation
Backup: backups/source-before-borrowed-flows-20261016.zip.
Changes: one new aa-economy module, one appended export, one test file and
this evidence folder. No manifests/dependencies, runtime adapter, generic
source parser, original assets/saves/profiles or Development08 were edited.
