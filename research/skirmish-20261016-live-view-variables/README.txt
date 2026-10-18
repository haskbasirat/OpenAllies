Controlled fresh current SVar owner (bounded dependency slice)
==============================================================

load_view_variables::State is the ONE current variable owner retained by the
active controlled World/application. It has no Clone/Default/Deserialize/raw
constructor. Views, zone Effects, dispatcher fog reads and mission consumers
must borrow its current fields. Source Settings declarations are not live
variable storage. Multiple simultaneous Worlds would have to share this process
owner; a second independently mutable copy would be wrong.

Owned native words/bytes:
* FogOfWar: indexed Var66, B39AB0+108, type3 Integer/flags20/bounds0..2.
* ResourceStockpileDefault: indexed Var68,+110, type2 Fixed/flags0.
* CityVictoryRatio: indexed Var76,+130,type3 Integer/flags0.
* EconomyStockpileResourceMax: indexed Var110,+1B8,type2 Fixed/flags0.
* CheatFog: indexed Var111,+1BC,type1 Bool/flags0; preserve the raw byte.
* ZoneBuilderFastDetectionZones: actual named type1 Bool from the source manager,
  cached by540590/5406D0/540860 after real4B2E60 type lookup; not a guessed flag.

Factory/publication order
-------------------------
reset_from_fresh_source(Catalog,NativeOverrides::ControlledNone) requires the
actual completed verified native source Bank and its source FP context. Unknown
profile/console override ownership refuses. It crosschecks public Settings raw
values/spans against private registered source nodes and checks actual WorldRule
presence/absence. Source PC is immutable Bank context, not a restore/live PC.

The constructor Fog default2 is NOT the World reset value. 474C50 applies actual
registered FogOfWar enum defaultEXPLORED1 before4902B1 copies the source snapshot
toB39AD0. 4F2826 resets current variables from that snapshot. CheatFog source seed
is0; current FastDetection source Bool istrue; stockpile default500, city ratio0
and maximum99999 come from their checked source/registered default producers.
These actual values are not arbitrary fallback choices.

Caller retains State before publish_world_rules(Catalog,WorldInventory).
The latter uses the actual separate RULES chunk, validated against its source
hash, and reuses the existing native merge/type/identity/last-wins/default logic.
It preflights the admitted closed setters, then stores selected current fields
in native WorldRule manager order. Native4FC859 calls512200 beforeTERRAIN opens
at4FC867 and its virtualLoad at4FC872. Header/creator rules are not this input.
CheatFog/Fast/Maximum have no registered WorldRule in this admitted source graph,
so publication LEAVES their current values unchanged. Future positive overrides
must bind their native owners instead of being silently ignored.

Current assignment/read methods never recompute source declarations. The known
Fog setter clamps signed values to0..2. Flags0 stockpile/city setters do not use
UI slider bounds. Native40E0A0 flags0 stores the input DWORD directly, preserving
NaN payloads. Bool setters preserve every byte payload; reads use nonzero.
validate_in_bank accepts the SAME Bank after its move intoStaged; it never
reconstructs or clones a completed Bank into a stale Catalog. The maximum getter
must be called lazily AFTER4E39D0's first result store, not precomputed per batch.

Native proof
------------
native_view_variables.py executes unchanged401940/4017D0 constructors,474C50
source-default application,4B3580 current reset,512200/512160 publication and
41C670 live Bool writes; actual4B2E60 returns the same current Fast variable.
141 rows /282 separate original+v14 executions /x87 PC24,PC53,PC64;451 executed
instruction pins. Constructor metadata, source-vs-reset value, locked-byte
independence, signed clamp extrema, mutable live values and byte payloads are
checked. Full functions check native return stack/nonvolatile registers and
balanced x87 TOP. CString allocation/assignment, named variable lookup and
native diagnostics are explicit seams. WorldRule descriptors and selected
snapshot-array entries are initialized inputs, not full source TGI/process
startup proof. Static registration/name/boundary pins are separate from executed
pins. No full enclosing GWorld/Session execution is claimed by these fixtures.

The sibling load_mission_variables helper author owns additional144-row/
288-execution/435-pin proof for source-created flags0 fixed/int constructors,
defaults/reset/publication and raw assignments. Its returned values are temporary
preflight inputs only. The current words live solely in THIS State. See sibling
research/skirmish-20261017-mission-start evidence for its exact scope.

Tests/status
------------
Scoped Rust command (original local data required):
  CARGO_TARGET_DIR=target/s-borrowed-leader
  AA_GAME_ROOT=E:\Games\Axis and Alies
  cargo test -p aa-runtime load_view_variables::tests --lib --release --locked --offline -j4 -- --include-ignored

One test replays all141 native rows against the private current-state kernel.
The actual-source test verifies the sealed source completion, actual three map
RULES chunks, exact reset values, current mutation, source tamper refusal,
corrupt rule preservation and actual Bank move validation. It does not claim
that native Windows profile/current-variable memory was captured for these new
variables; ControlledNone is an explicit scope for this fresh Rust path.

This is a dependency slice for the reached fields, not the entire SVar manager,
full exception/unwind behavior, or battle readiness. Fallible validation is
preflight; selected raw stores have no outward mutable callbacks. Current UI,
console, Mind and vision consumers must use the SAME owner once live; old saved
helper functions remain source/constructor tools and cannot substitute for a
later current read. Original files, profiles, saves and release are unchanged.

Final bounded verification (2026-10-17)
--------------------------------------
Final Cargo scope passed2/2, including the actual-source ignored test; current
aa-runtime --lib --tests check passed. The same compiled binary passed all5
existing saved_action_settings tests with AA_GAME_ROOT explicitly set. Its first
direct invocation omitted that environment variable and the ignored local-data
test failed at its env unwrap; that failed log is retained separately.

Fresh native replay with lookup owner assertions passed141/282/451 and preserved
fixture hash40f1bc792e0b907eec5633de930f8a4b5daf36a5421eca4fae96e3edf5c72f11.
The explicit Settings method ECX isB39AA8; named lookup receives its+14 hash-map
subobjectB39ABC. Adding that owner assertion found direct-setter probe calls
which previously suppliedECX0 to an owner-insensitive lookup seam; they now
supply the actual Settings owner. The raw outputs/fixture did not change.
Failed assertion log native-replay-final.log remains for audit.

The tested State bytes are copied to load-view-variables-tested-before-cv.rs
(SHA44417d2391ae87be706193e0936f8e717c742187086d1096c725e4e3497fae8e).
Ownership was then released to world_startup_gap for root-approved additional
current CV fields in the SAME State. These results certify the saved pre-CV
bytes; subsequent extensions require their own checks. No playable-battle or
full consumer-migration completion is claimed here.
