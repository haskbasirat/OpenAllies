Native fresh SVars field order / exact source initialization

API: aa_rules::actors::ResolvedNode::native_svar_fields() returns read-only
SVarField tokens into SAME private Definition.fields and Provenance. Tokens
expose name/declared_type/source/raw and LAZY source_kind()/value(PC). Enumeration
does not eagerly parse values: the caller may find an existing flags2 hardcoded
Var and skip its getter. Later unsupported values cannot erase earlier stores.

Scope is one fresh ordinary childless SVars node, one layer, no template/base/
prefilled hash map, verified source hashes, unique ASCII field identifiers and
supported declarations. Unknown inferred literal types are explicit errors.
Typed values reuse the existing field getter. Real untyped decimal0.25 is
SOURCE FLOAT(kind3,A48560), converted by Settings to destination Fixed(kind2).
Untyped true/false is sourceBool5. Explicit int is sourcekind2; fixed4,float3,
Bool5,String6,Point3=8. The initial author vtable-label inference incorrectly
called the decimal Fixed and int1; actual virtual48 execution/Rust corpus caught
both before acceptance and production was corrected. The fixture is authority.
This is bounded ordinary decimal-with-dot/Bool inference, not an expression parser.

Native order is NOT lexical or authored sorting. DataNode+40 starts capacity5;
405030 uses ASCII-folded h=h*33+unit.496F60 head-prepends and grows BEFORE insertion
if count>=2*capacity and growth enabled.491F80 chooses2N+1 then increments2 until
neither divisible3 nor5, walking old buckets/chains ascending and PREPENDING into
new buckets.4245A0/4B4300 consume ascending buckets and each head chain. Rehash
reversals matter. Inherited/prefilled hash maps stay explicit unsupported scope.

native_field_order.py/json: whole5BD080 parser, actual496F60/491F80, actual
4B3C10/Add/hash/vector/setters.51paired rows/102separate original-v14/allPC runs,
6187executed pins. Counts0..231 cover collision/growth boundaries; typed/inferred
forms execute their actual virtual48 getters. Existing line/EOF/string/heap/
mutex host transports are disclosed. No registration transport.

native_full_sources.py/json: real5065D0 typed constructors through REAL4B3470,
then both ACTUAL selected source files in SAME Settings allocation.3paired rows/
6separate original-v14/allPC runs,9487executed pins.135 constructor registrations,
214+40 source fields,231 final current cells. Snapshots retain ordered index/name/
kind/flags/logical current values and scalar bounds. No captured current input.
This receipt stops BEFORE WorldRules/default reset and later current mutations.
Another agent owns the sole canonical runtime current-cell store.

Rust lazy/guard test passed1/1; actual source projection test passed1/1 with
214+40 fields and exactly3 inferred rows. Corrected helper3/3 passed,
including all51native field rows. Required fullaa-rules suite154/154PASS,0failed/
ignored, and required metagame export check1/1PASS using AA_METAGAME_EXPORT=0;
oracle source unchanged. findings.json is the canonical final author status;
source-proof-pins.json records the unchanged source freeze made while that full
suite was still pending.

Only a bounded projection/newtests and tiny exports are added, no production
parser behavior change. Local source-before-export reconstructions are labelled
honestly in backups/manifest.json. Root manifests/lockfiles, original executables,
assets/saves/releases remain unchanged by this author. No complete world,
mission, battle, AI, multiplayer or game readiness claim.
