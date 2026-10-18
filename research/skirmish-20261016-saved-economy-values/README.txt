Saved Economy value services -- 2026-10-16

API/ownership
-------------
aa_runtime::saved_economy_values::View::new(actions, focused, held, precision)
is crate-private. `actions` is the real SavedActions context; `focused` is
the actual borrowed EconomyState under570020; `held` is a mutable slice of
(ActorId, &mut EconomyState) ancestor loans. The slice borrow lifetime is
separate from its inner loans, supporting short recursive reborrows.

Methods return aa_actors::economy_values::Result:
  upkeep_blocked(actor), provides(actor) -> bool
  cost_or_upkeep(actor, VectorKind), production(actor, cache_members)
    -> Vec<aa_body::F32>
  parent_organization(actor) -> Option<ActorId>
GateWorld/World are implemented for the same View. No reference escapes a
callback. Only Economy+44 member-cache writes mutate gameplay storage.

The refresh integrator owns taking/reinserting the actual Box and validates
focused header-owner identity. View treats focused and held allocations as
present ahead of registry slots and writes their same vectors. Held explicit
storage IDs must match the actual component header. Other actors use their
actual registry components. No clone/default Economy is used; immutable
source/read-value copies required by the existing trait are not owners.
The active Organization allocation stays in OrganizationAttachContext and
is observed even while its registry record/component slot is being loaded.

Bound native fields/algorithms
-----------------------------
Reuses aa_actors::economy_values, not a second implementation of vector maths:
56F9A0 production gate;4D0C90 supply;56FA30 cost;56FB10 upkeep;
56FC20 ordered Organization production/member caches;45F4D0/45F570/45F680
definition getters;501730/501A00/5019D0 property modifier operations.
The inline570020 upkeep gate preserves Element+10, Thing flag1, then
Organization/recruitable early returns. Parent lookup reads actual Element+10.

Source identity comes from completed saved RuntimeDefinition. Catalog's
canonical-identity guard prevents a second translated ActorData lookup.
Resource vectors/recruitability use existing typed source bindings. Layout
group indices and primary-slot fields use the completed NativeLayoutData
arena and exact canonical identities; saved parameters/members remain in
their original order. Null slots are retained.

Properties: actual4CF110 presence/owner, modifier flags, scalar4C/60 and
production table. The table is read lazily by5019D0 and its mismatched-count
rule is the existing native helper, not guessed zero input.
Supply: actual state19 gate and SupplyState.stockpile(+18), including native
ordered-positive behavior. Transport: actual stored cost(+8)/upkeep(+18)
vectors, including their existing lengths and raw words.
Inverse packed reference(+548): completed native MobileDeployment arena.

Important source correction
---------------------------
ActorData+17C is a recruitment-backreference head. It is not transport_actor
(+184). A root-approved narrow shared change adds defaulted World method
recruit_backref_present.501730 invokes it only on non-Construction Cost,
after one45E5A0 recruitability test per scalar returns false. Upkeep,
Production, no modifier, Construction and recruitable cost skip it.
The saved View calls Catalog.native_recruit_backrefs(pc).present(canonicalID),
using the separately proved460310/460000 producer. Legacy snapshot word17c
remains for fixture adapters; View's placeholder is never consulted.
Root owns the corresponding live-ActorStore adapter correction and cache.

Absent Economy source fields are not invented defaults: original constructor
4628D1..462943 creates empty Cost/Upkeep/Production vectors (capacity8) and
zero supply_required(+348).6 original/v14/three-PC executions prove these
stores. The earlier original4666C0 loader fixture proves no EconomyComponent
leaves those fields untouched. Native no-Economy definitions are therefore
represented with actual empty vectors/false supply, not domain-sized zeros.

Proof/tests
-----------
New focused lazy-getter test and both existing economy_values_native tests
pass; actors-tests.log. Existing original vector/gate/cache fixture is reused,
including all3PC modes. No old expected output is modified.

Actual tutorial02 source capture uses mode1/PC53 InspectPartial, with46real
Actor records,32canonical definitions and29actual Economy components. Its
Organizations have no Economy slot, so focus/held are real member boxes from
the registry. An initial test setup assumption of a combined Org/Economy was
corrected; the failing capture-no-org-economy.log is preserved.

actual-saved-input.json exports exact source/owned component values to the
established native-value harness. native-saved-values.json executes518cases:
actual original and independently loadedv14 at PC53, all actual actor
provides/supplied/cost/upkeep/production queries, plus the inline upkeep gate
for29Economy owners. Full native value algorithms execute; allocator/free and
the pure Properties-context lookup remain declared harness seams. The upkeep
gate uses a nonnull owner fixture and stops at57008B/570102 before refresh
mutations. Constructor proof is separate, all3PC. Null-page guards are active.

The initial v14 overlay reset two explicit registry globals after inherited
fixture setup; the harness now reinstalls those pointers after image load.
The failure log is retained and all native outputs were regenerated. It was
a harness setup failure, not an observed game difference.

Mandatory Rust recompare checks captured inputs, every native output and
every real member cache. It includes Organizations without an Economy,
null/ordered members, actual Properties and held/focused allocations. Direct
cache probes additionally verify same Box addresses and actual focus/held
mutations. The resulting log, not capture mode, is runtime-tests.log.

Reproduce (workspace root):
  $env:CARGO_TARGET_DIR='target/s-saved-economy-values'
  cargo test -p aa-actors --release --locked --offline -j4 --test economy_lazy_backref --test economy_values_native -- --nocapture
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  $env:AA_SAVED_ECONOMY_CAPTURE='1'
  cargo test -p aa-runtime --release --locked --offline -j4 --lib saved_economy_values::tests -- --ignored --nocapture
  Remove-Item Env:AA_SAVED_ECONOMY_CAPTURE
  $env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
  python research/skirmish-20261016-saved-economy-values/native_saved_values.py
  cargo test -p aa-runtime --release --locked --offline -j4 --lib saved_economy_values::tests -- --ignored --nocapture
  python research/skirmish-20261016-saved-economy-values/record_findings.py

Limits
------
This is value/caching integration, not full570020 or Kingdom initialization.
Natural missing Kingdom flow arrays remain unavailable; none are created or
reset here. Full refresh, actor scheduling, SAI/publication, full saved-world
completion, playable skirmish and multiplayer are separate required stages.
The adapter has coherent valid-owned-storage semantics; exact fault/read
timing for malformed unrelated payloads and allocation exhaustion is unclaimed.
No new engine cap or dependency is introduced. No original game/save/profile
or release EXE was changed. New files and the shared trait/lib exports were
backed up before changes; full rules/metagame compatibility is root-owned.
