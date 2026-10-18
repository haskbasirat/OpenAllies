ActorData43C TechnologyProperty source binding, 2026-10-17
========================================================
45ECA0 returns the real ActorData+43C list address and current head into the
iterator. It is distinct from ResearchComponent+558 research items. Structure
loader46797F..4679A7 invokes4C4CF0: typed TechnologyProperty.property_IDS source
entries resolve through canonical Property Find and are PREPENDED. Repeated
references are retained; list traversal is reverse authored order. Constructor
4624E1 sets EBX0 and462B37/3D store zero head/tail, with callee-save EBX contract;
those constructor bytes are static provenance, not claimed full ctor execution.

NEW definition_technology_properties::bind reuses that accepted source producer
under the actual completed Bank precision, canonical source/class identity and
fresh single-declaration guard (definition_fx::Source). Property identities use
the same Catalog.definition_bank as the property instance owner. They do not
claim definitions have live effects. The exact structure_properties callback in
load_owner_add_actor now calls this helper only at its native45ECA0 branch and
validates actual held Actor. No registry scan, source-alphabetic sorting, fixed
pointer value, default empty list or Research registry substitutes for43C.

Fresh paired proof, both original/v14 and PC24/53/64:
- native-technology-properties.json:15 pairs/30 executions/5167 recorded pins;
  original parser +4C4CF0 +45ECA0, empty/one/three/repeated/case-varied/missing,
  actual head/tail/iterator and retained partial stores on missing lookup.
- native-technology-use.json:48 pairs/96 executions/27 pins; reached4E6379..AE
  consumer, real45ECA0, ordered4E3AD0(property,10,currentActor,1) calls, required
  stops at every child.4E63AE is an observed cutoff pin, not executed terminal.
- native-actual-technology-properties.json:15 pairs/30 executions/4481 pins;
  pure source authored capture and SAME source-universe actual Bank capture
  agree with native source producer/45ECA0 and portable token names/order.
Total78 pairs/156 executions. No unique-instruction totals are summed.
Property Find/allocations are declared fixture seams, not full engine services.

Actual tutorial02 saved2697 is weurope_farm6x8_01 and its source43C list is
really empty. It therefore advances without4E3AD0. All4 installed positive
lists were also exercised: assembly_parts->assembly_line_tech,
deployment_crates->rapid_deployment_tech, ger/rus_infantry_supplies->
military_gearup_tech. The latter two share the same portable Bank token. The
separately compiled read-only capture helper exhausts source actor structures,
records all nonempty authored lists before head insertion, and preserves full
source identities. Tokens in captures are evidence only, not hardcoded indices.

Two scoped tests PASS using a small standalone test crate importing the real
compiled public aa_runtime APIs and reusing the actual owned test module. No
implementation source/private Catalog guard was included or made public. This
avoided a redundant runtime build after unrelated in-flight source additions:
--tests hit the existing owned_load_send trait Send requirements, and --lib hit
an application_strings fixture that its owner had not yet written at compile
snapshot. Failed logs are retained; author subsequently supplied that fixture.
No claim those full Cargo scopes were rerun successfully. Parent owns final
shared integration validation; the2 tests exercise the production helper.

Production helper and tests are frozen. Before backup is a contemporaneous
copy of load_owner_add_actor.rs. No generic aa-rules parser/loader, external
dependency, manifest, original game/save/profile or release file changes.
Positive4E3AD0 effects are assigned separately; this helper cannot authorize
property effects, source partial-object restoration, full reload or battle.
