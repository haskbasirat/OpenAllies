ACTORDATA COMBAT VALUE SOURCE INPUTS -- 2026-10-16

Delivered scope
New aa-runtime::combat_value_source provides immutable inputs only. It does
NOT expose completed ActorData+1E4, Children30C or Weighted36C, does not use the
constructor0/-1 as completion, and is not called to invent a vacant-slot value.
Saved Organization's reached bit5 vacancy must retain its missing-completion
error until the actual source producer is bound.

APIs
  definition_inputs(&Catalog,canonical_id,source_precision)->DefinitionInputs
  ability_input(&LoadedAbility)->combat_value::AbilityLocal
  actor_manager_order(&Catalog)->ActorRegistrationOrder
  ActorRegistrationOrder::require_no_source_diagnostics()
The final helper is a conservative guard, NOT a completion certificate. The
installed native-order catalog retains11 diagnostics including syntax notices;
they are not labelled11 native load failures. Full completion needs structured
load-outcome classification. The report retains every notice.

Inputs and identity
The binder consumes existing completed Catalog ActorBindings, ground Body/Mover
fields and LoadedPerformer definitions at the explicitly supplied source PC.
It never calculates these fields again from simplified effects or raw counts.
Canonical ActorData names shadowed by Translate are rejected by the existing
identity guard. Shared AbilityDefinition translation/shadow limitations also
remain explicit. A source phase's PC is separate from restore/simulation PC.

AbilityLocal maps exact native fields: definition+54 flags,+24 range,+38 area,
+3C projectile presence,+64 computed power; local+4 reload_time,+18 completed
recharge.4BE680's first-entry value is the FIRST attack length from the FIRST
animation set, not use_fraction or another clip. Ability count/order is the
completed native-equivalent Performer loader's order.

ActorData+1D4 uses existing native_actor_properties, which resolves references
at the actual historical actor load position and preserves4C4CF0 prepend order.
Final Property bias is read from the exact registered-root provenance and
performer_cv_bias(+28), without applying Translate twice. Property re-loads or
incomplete registry loads remain errors because that producer does not retain
the latest full Property field state. No zero property bias is invented.

Children2F0 uses the bound default Organization layout's slot_groups and its
canonical parameter table, retaining repeated/null slots. This is source
ActorData completion input, distinct from a live Organization's current layout.
Weighted35C is Denizen Inventory, not Transport. Reuses native_inventory_insert
to retain case-sensitive equal-name runs and duplicate entries, while deferred
Thing references resolve once to canonical targets. Signed counts are preserved;
the later runtime Denizen behavior/slot classifier is not invoked here.

Native manager order is reverse ActorData creation/registration. It is limited
to native-order catalogs, rejects syntax-only ActorCatalog::from_files, and
classifies exact canonical roots by source path/hash/span rather than Find.
That matters for installed chaparral_wall: the canonical registered name is
shadowed by a later Thing translation. Returning to Find would select a
different definition. The original4BCA00 prepend proof is in the independent
combat-source-settings evidence below. Actual retained ActorData count:1028.

New validation
10 actual source definitions at PC24/PC53/PC64 (30 rows): five nations' infantry,
USA/GBR corps, USA infantry regiment, horse herd, USA airfield. These cover
completed abilities, properties, repeated/null Organization slots and weighted
Denizen inventory. Actual input artifact:actual-source-inputs.json.

native_source_inputs.py executes both pinned original/v14 images:
-60 Ability getter rows, each executing4BE680,4BE910,4BE3F0,4BE4B0 (240 calls).
-60 original4618B0 prefixes assemble ordered ability arguments with native
 property reduction and component gates, stopping at461BA0 BEFORE Children30C.
 The45E090 body is a declared recorded leaf returning laboratory values. Those
 responses are NOT original completed combat values and are not used by runtime.
-132 executed instruction pins plus6 static loader mapping ranges, including
 Performer.cv_bias->ActorData210 and Property.performer_cv_bias->Property28.
No arithmetic, property summation or native getter body in this bounded scope
is replaced. Numeric CV formula/completion remains the existing pure port.

Three runtime tests cover actual source snapshots, original getter/prefix replay,
and refusal of an unproven syntax-only registry order. See final-tests.log.
Target:target/s-combat-value-source; release/offline/locked/-j4. No aa-rules
parser/loader or manifests changed, so the rules/metagame export compatibility
gate is not claimed or triggered by edits to those files. Only runtime lib.rs
export was appended; its pre-edit backup is in the named backups folder.

Independent source-context research
research/skirmish-20261016-combat-source-settings/ contains42 additional native
cases for4BCA00 manager order, full4BD800 controller order and empty CSV cache.
Those are separately authored and are not added to the new getter count above.
It proves: renderer initialization precedes synchronous main-thread source
completion;4BD800 inherits caller PC;4BBF90 means precedes4BB590 values. PC24
after D3D is a static expectation, NOT an observed native control-word receipt.
Source profile, CRT SSE2 environment and FP state must remain explicit.

Full completion dependencies left open
-10 effective numeric variables:7 cached formula fields,mean discard,and2lazy
 ranged values. Use actual typed source, source identity and WorldRule default
 handling/absence; hardcoded defaults differ from source values.
-CombatValueCSV is a UVar, not an SVar. Effective empty UTF16 string requires
 UVars/profile/override proof before skipping debug-stream behavior.
-Means independently reads Settlement extent45E6D0. Completed radius+478 is
 replaced by460940 DetermineWallPieces after model segment/gate/corner fitting,
 FSIN/FCOS and float stores; raw wall_diameter is NOT that radius. Positive
 levels also follow firstLevel WallSet corner ActorData+3C. This producer is
 not ported by this module.
-All reached source graph references need their actual completed owners and
 source-load order/precision, before any1E4/30C/36C arena can be published.
-Original per-record Init/PostCreate and live Performer+60 production remain
 separate lifecycle work. Input binding does not make a battle playable.

No originals, saves, existing releases or runtime Catalog/adapter files changed.
