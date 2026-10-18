Settlement source dependency and shared-definition prefix, 2026-10-16

Outcome
-------
The current native-order v14 source catalog contains1028 distinct registered
kind2 ActorData definitions and NO SettlementComponent instances or registered
WallSets. All1647 input source hashes were independently checked against the
original archive's extracted bytes or the actual v14 loose overrides. The
loader report contains11 native syntax notices and0 fatal/truncating exceptions.

This is an input/registration census, not an execution of the original Windows
application's complete data load. It uses the exact registration provenance,
including identities whose own IDS was later redirected by a Translate. It
does not misclassify only the currently reachable Thing Find result. Source
hashes, all1028 registration provenances, the11 notices, native index and load
report are retained in real-source-census-v2.json. The earlier smaller census
is superseded. A synthetic translated-IDS regression keeps that distinction.

Original464BD0's optional-child branch4679D0..4679FC was separately executed:
GetChild("SettlementComponent",0) returning null skips4640F0 and leaves mask
bit14 untouched; a nonnull child sets bit14 before calling4640F0. The12 rows
cover both pinned PEs at PC24/53/64. Child lookup and4640F0 are explicit seams.

Consequently,460940 is not demonstrated as a reached dependency for this
current shipped ActorData population. The prior broad list of possible means
dependencies must not be read as proof that Settlement fitting currently
blocks startup. These routines may be unused Kohan-era engine behavior in AA.
New source/mods that actually contain SettlementComponent still require it.

What was ported
---------------
crates/aa-rules/src/settlement_completion.rs adds no parser or load changes.
The only existing-source edit is an append-only public module export.

1. native_settlement_registration_census: inspect registered kind2 definitions
   by exact original source provenance, preserving all sources and diagnostics.
   This is not a full definition-completion certificate.
2. native_wall_set_inputs: immutable current-registry source projection of
   segment/corner/gate references, canonical identities, actual inherited
   ObjectData shapes, component masks and flags.47DFF8..47E06D requires all
   three WallComponent bits, and rejects targetable only on the corner.
   IMPORTANT:47DFB9 resolves references at LOAD TIME. This immutable projection
   uses the CURRENT registry. Later Translate/registrations can change it;
   historical identity is an explicit prerequisite for future runtime binding.
   No completed mutable WallSet is claimed. The oracle-backed fixtures place
   the three ActorData registrations before WallSet and do not change their
   reference identities afterwards; only their validation outcome is compared.
3. SharedDefinitions::append_level: bounded46450B..46469A after the loaded
   Max/Find outputs. Retain the Level record on failure, validate Structure
   bit13 and existing center owner, assign+430 BEFORE unknown-WallSet failure,
   then copy the first Level's current shared shapes into later wall sets.
4. initialize_placeholders:4647B1..464909 copies all three blocking shapes
   first, then all three soft shapes, then sets the three+1A8 bytes. Cross-piece
   aliases read each current source as reached, preserving cascading writes.
   A snapshot of all source shapes at entry would be wrong. Target+68 shapes
   and unrelated fields are not changed.
5. initial_radius: only46491C..46492F diameter*binary32(0.5), with explicit
   aa_numeric precision and binary32 store. This is NOT fitted/completed+478.

Native proof and tests
---------------------
native_prefix.py executes unchanged instructions from each pinned original
and v14 image, at PC24/PC53/PC64. Page0 faults unless FS-prefixed. A failed
410C40 is an explicit stop seam, Thing Find is the declared lookup seam;
no allocator, parser, full4640F0 or460940 is substituted with fake success.

*384 shared-definition cases:360 success/alias cases plus24 ordered failures.
  Rust compares actual mutated blocking/soft words, owners and wall bytes.
*192 WallSet lookup/validation cases, including all null/component/cornerflag
  combinations. Rust compares the immutable projection's success predicate;
  full native WallSet partial writes are not represented by that source API.
*54 finite initial-radius stores, including negative zero and tiny inputs.
*12 separate optional-child gating cases in native-presence-branch.json.
*287 executed instruction pins in native-prefix.json and12 branch pins match
  both accepted PE images byte-for-byte.
*4 scoped Rust tests pass, including translated registration and real data.
  tests-accepted.log records the run. Failed exploratory fixture assertions
  were retained in tests-final.log and corrected, not hidden or weakened.
*18 aa-rules library compatibility tests pass with real-data ignored tests
  explicitly included. This is the library set, not every integration binary.
*Ordinary aa-rules library Clippy succeeds;5 existing warnings are confined
  to unchanged native_load.rs/native_display_name.rs. No allowances were added
  and strict-warning cleanliness is not claimed for the existing crate.

Hardware x87 reference
----------------------
hardware_trig.py generates a tiny independently authored Windows x64 leaf,
not SDK code. It saves/restores the caller's entire FXSAVE64 state, FNINITs,
loads an explicit CW and raw80 operand, executes exactly FSIN or FCOS, stores
raw80/CW/status and restores the state. Allocated code is switched from writable
to executable and FlushInstructionCache is checked before execution. Buffers
are aligned and all allocations are released. No exception masks or security
settings are changed outside the restored diagnostic call.

96 cases (16 actual native angle operands*3 PCs*2 operations), each repeated,
plus sin(0)/cos(0) identity anchors pass on this Intel Family6 Model167 CPU.
Native angle producer460C07..460C18 FILD/FDIVR/FLD is executed under Unicorn;
it stops BEFORE Unicorn's inaccurate transcendental. FSIN/FCOS raw80 output
comes from the physical CPU. Code bytes, disassembly and image/producer hashes
are retained in hardware-trig.json. This is an accurate LOCAL reference for
that instruction on this CPU, not a portable trig implementation, universal
cross-CPU accuracy proof, original application source-PC receipt or460940 fit.

Actual remaining CV/startup work
-------------------------------
The existing saved Organization adapter explicitly refuses unavailable
"4618B0 completed canonical ActorData+1E4". This work does not manufacture that
value or change the adapter. Existing definition_inputs binds actual Body
defense/max health, Mover speed, completed Performer ability fields, property
biases, ordered Organization slots and Denizen inventory for tested definitions.
It is still an INPUT binder. Its45E090 oracle response was a laboratory seam.

The reached source-completion work remains:10 effective numeric settings
(7 cached formula values, mean discard and2 lazy ranged settings); the actual
UVar CombatValueCSV profile/override result;4BBF90 means before4BB590 values;
the actual completed shared definition graph in native reverse registration
order; explicit source-load PC and CRT/SSE2 environment. A world restore's
PC53 must not silently replace the earlier main-thread source-load context.
Full source+1E4/+30C/+36C publication and live Performer+60 remain separate.
Immediate Init/PostCreate, world services and battle startup are not completed.

Limits and preservation
-----------------------
No engine implementation from the downloaded SDK archives was adopted. No
original EXE, recovered asset, save, profile, prior release, Cargo manifest or
lockfile changed. Full460940 fitting, CRT steps, wall-piece placement and full
4640F0 source-loader completion remain unsupported. No playable skirmish,
multiplayer, render or performance improvement is claimed by this work.
