Saved Supply InitExisting58B0E0 / Tick58AE10(0) / After4B2830

The actual strict Tutorial45 load stopped on usa_corps4097 Supply after the
new Denizen InitExisting completed. Denizen AfterInit was not reached because
the InitExisting component loop stopped first. No full battle result is claimed.

Production uses the SAME current Supply allocation, actual Queues intrusive
head and RNG, Actor+148 vector, ZoneMap, zone identity arena, source Bank PC,
and retained mutable current SVar State. There is no SupplyActor snapshot or
copyback. Scheduling has no World-version guard for this native caller.

58B0E0 order: pending check / optional queue1(data0), current first-zone check,
actual4D69A0 Zone0 construction if needed, then58AE10(0). An allocated inactive
Zone0 survives every later failure. AfterInit is the actual4B2830 RET.

58AE10 preserves each stockpile binary32 store/read, reloads relay/source
fields at native boundaries, clears1C BEFORE an unsupported relay collector,
and clears dirty14 only AFTER the real zone move succeeds.58ACE0 reads the
owner twice and both high/low state words before branching on the high bit.
The primary non-relay4D0A80 immediate true return is concretely bound; the
unchanged4D7CE0 branch completes. A reached state transition uses the existing
native state coordinator and retains its real error prefix, never a raw bit flip.

New source-reset cells in the existing single State:
 SupplyProvideUnderSiege: native507C2C..67,4017D0 Bool/type1/flags0/default0,
 slot63 -> cache+FC; shipped Game/SVars.tgi flag=false.
 SupplyStockpileRechargeRate: native507C6C..AA,401710 Fixed/type2/flags0/
 default1.0, slot64 -> cache+100; shipped fixed=1.0.
Both declarations are validated against private resolved source values/spans at
the immutable source PC; WorldRule registration is explicitly refused because
neither is registered in the admitted current source universe. Later RULES
publication preserves these SAME current cells. Setters preserve raw bytes and
DWORDs, including signalling-NaN words; evaluating unsupported numeric input is
a separate explicit error. No profile/console override ownership was invented.

Native proofs, both original/v14 at PC24/53/64:
 native_supply_init.py ->1506 paired rows /3012 separate executions /112
 executed block pins. Full Init executes actual4D69A0/516640. Tick coordinator
declares update/provides/positive zone solvers as children. Separate58ACE0 and
primary4D0A80/4D7CE0 probes execute the actual leaves. Every labelled stop is
before a read/store instruction or child entry, never a MEM_WRITE exception.
 native_supply_variables.py ->12 paired rows /24 executions /81 block pins.
It executes the isolated native SVar declaration fragment, constructors and
raw setters;4B3470 registration and named lookup are declared transports. It
does not claim a complete native Settings registry or full TGI parser run.

Rust fixture replay source: load_supply_init_world::tests (2 tests), includes
all1506 rows and completed error prefixes. Actual SVar ownership/reset/3-map
RULES preservation is added to the existing ignored load_view_variables test.
Root requested one batched runtime link, so the author did NOT start another
full runtime link. See findings-pending-rust.json for the exact current status.
Production --lib check passed; the --lib --tests check had no Supply errors but
failed a foreign mission_owner_sources path-included integration harness.

Explicit remaining reached boundaries: actual relay-list collection, nonprimary
Element/Kingdom/territory supply calculation, positive state/action effects,
stockpile-depletion Economy refresh, solver callbacks still required by the
existing shared zone adapter, and unexpected dead-Actor diagnostic logging.
Native Tick nonfinite arithmetic is outside the numeric proven domain; the
range predicate's signed-zero/NaN/infinity results are covered separately.

Historical failed author logs remain: the SVar proof's initial wrong helper
name and duplicate named-lookup hook were transport harness bugs, corrected
before producing native-supply-variables.json. Production values were unchanged.
Original game, Cargo manifests/lock, release files and SDK sources were untouched.

SOURCE ORDER CORRECTION (actual attempt09 regression)
The declaration/default proof above correctly executes native false default,
but that is not the final source current. Data/Game/SVars.tgi declares flag
false; Data/Game/SVars_AA.tgi assigns untyped true to the SAME registered Bool.
The old one-declaration assertion caused a genuine early-load regression.
The new exact two-file producer validates both public/private paths/spans and
the original typed declaration plus untyped literal. Full native4B3C10 proof
executes96 paired rows/192 original-v14 all-PC calls with1062 block pins;
all second-call error prefixes preserve the prior current byte. The final
current source value is true. No Settings/parser/default relaxation occurred.
Historical source snapshots/report/logs remain. See source-order-findings.json
for current pins and actual State/attempt10 test status.

RUNTIME45 NaN RANGE CORRECTION
The first root replay found a genuine unordered-compare bug:58AD79..84 uses
FCOMP/FNSTSW plus TEST AH,5/JNP, so unordered NaN continues to the City read.
Rust had incorrectly rejected it via !(range>=0). The shared actual core now
rejects only ordered range<0. All1506 native rows and relay prefix test pass
in the focused extraction replay2/2. Fixture bytes remain unchanged. See
nan-range-findings.json; the runtime45 failure is retained.
