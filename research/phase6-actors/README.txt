PHASE 6: TYPED DEFINITION INPUTS AND ACTOR CONSTRUCTION
=====================================================

Production interfaces
---------------------
aa-rules ActorCatalog now retains its verified source/template representation.
Existing raw inspector fields and their behavior remain intact. New interfaces:

native_field(id, dotted_path, precision) resolves inherited fixed/float/boolean/
string declarations separately from the winning authored value. Both declaration
and value have exact source path/hash/span. Native GetFixed rejects Float despite
both representations storing binary32. Optional-only child declarations remain
absent; mandatory declarations without values, ambiguous/cyclic templates,
duplicate fields, unsupported headers/types and expressions produce errors.
This remains the existing proven ordinary/missing-field inheritance subset,
not the complete native template/list/language/type-replacement evaluator.

scalar_literal executes the proved canonical ASCII decimal subset: optional
sign, decimal point, f/d/k suffix or percent. Decimal parsing produces binary64,
then a binary32 store, then (for percent) multiplication by original binary32
0x3C23D70A with explicit precision and another binary32 store. Exponents and
general arithmetic/reference expressions are not silently accepted as numbers.
786 actual original classifier/assignment cases cover all three precision modes,
including native CRT parsing and both native fixed/float constructors.

native_attributes resolves the actual Body health, defense, rot/sink times,
captureable/destructible booleans and Mover speed/acceleration/radius/turn-type.
These are definition inputs, not current live component or restored state.
The pinned local catalog resolves this bounded projection for 1,627 of 1,645
Thing rows;18 rows have explicit ambiguity/errors. Many Thing rows are scenery
with no Body/Mover. This figure is NOT 1,627 fully implemented actor types.

native_category/native_class execute the built-in43-category table recovered
from whole476700, real474920 constructors,4B4E60 ordinal assignment and766DE0
array append. Descriptor runtime class is independent of an untrusted serialized
class. Native kind+18, runtime class+1C, view type+20 and registry index+C are
distinct. User category registry overrides are not implemented.

native_base_flags ports the complete4BB310 base flag loop using actual18-entry
native flag table. native_component_mask uses the30 actual child-presence
branches (Mind has no node bit), preserves the final Performer filter, and
requires the caller's completed Performer Ability count where that filter needs
it. Raw list declarations are never counted as completed abilities. This API is
for fresh ActorData construction: constructor+218 is0 before Detection loads,
so the original negative/negative detection-clear condition cannot fire there.
Research requires Structure. Complete validation/loading of all component data
is still separate; producing a mask does not instantiate a component.

native_final_flags handles definition kinds0/1/2: ThingData/ObjectData/standard
ActorData. The4BCA00 definition factory selects descriptor+18 native kind, not
descriptor+1C runtime class. RANDOM_ACTOR and RANDOM_OBJECT share runtime
classes with standard objects but have distinct unported data subclasses.
Actor final
flag adjustments at468995..468A43 are covered by162 original executions. Other
derived classes, including DynamicDoodad9, return an explicit unsupported
finalizer error. Do not feed native_base_flags to a caller requiring final flags.

aa-actors loading::construct_shell(Thing/Object/Actor, definition) ports the
separate saved-load constructors4EF020 ->4EA690 ->4D6D70. Position/facing/name
begin empty, while Thing+1C is explicitly None because native leaves it
uninitialized until Load. Object fields and Actor's31 empty component slots,
state words and remaining initialized class words are preserved. It consumes
no RNG, normalizes no saved facing and invents no owner/component codec.
Nine whole original constructor cases cover three kinds and three saved GTIDs.

IdentityRegistry::from_load_metadata accepts next slot/highest slot and exactly
the serialized generation prefix. It retains a positive highest slot with no
live entries, reproducing the intermediate IDMGR state. It pads the unstored
generation tail with Reset zeros. Saved objects then call register_saved directly
without reseeding. Six native metadata cases from phase6-mapworld compare every
generation slot. The normal completed-world checkpoint validator remains strict.

creation::construct_actor ports new construction4EF2A0 ->4EA5E0 ->4D6BB0.
It preserves facing RNG then aspect-variant RNG, explicit variant behavior,
record owner mutation when not ownable, native initialized Actor class fields,
and the creation record's word30 copy. initialize_object ports4E9FC0: identity
registration precedes a random scale draw and scale write. Weighted selection
49B900 and scale49B440 retain extended intermediate arithmetic; the scale range
subtraction has its original binary32 store. The native precomputed total
variant weight is an explicit input, not recomputed in a different order.
81 whole original constructor/InitCreate cases cover PC24/53/64, ownership,
random orientation, random/explicit/disabled variants and multiple RNG states.

Boundaries that remain
----------------------
- New-creation record+52 requests the unported blocking/spatial callback branch;
  this is rejected before effects as an explicit unsupported precondition.
- ObjectDefinition aspect lists/total weights and scale fields must be resolved
  by the native-equivalent caller. No unknown variation or numeric defaults.
- ActorInitCreate4D6F10, owners/required objects, component factories/codecs,
  initial abilities, full deployment/publication/destruction are not completed.
- Saved actor component tails remain separate decoder work. Restoring headers
  and registering saved IDs does not mean a live playable match is restored.
- Invalid native pointer/index cases return safe errors. The weighted selector
  rejects running beyond supplied entries rather than emulating an out-of-bounds
  read. Registration before a later scale error remains visible.
- The native thread precision/lifecycle is explicit, not inferred globally.

Evidence and reproduction
-------------------------
typed_scalar_oracle.py: original5BD6E0,5B9E20/5BB9F0, CRT96D3E8 and native value
constructors. C-locale TLS and allocation/insertion are declared leaves.
definition_flags_oracle.py: fullbase flag loop, scoped component presence/late
filter/finalflag fragments; resolved node/getter inputs are explicit boundaries.
category_oracle.py: complete built-in initializer/ctors and ordered registration;
duplicate/hash bucket operations are declared leaves.
load_constructors_oracle.py and created_actor_oracle.py: complete original ctor
chains with successful CString-pool allocation leaves. Native/v14 bytes are pinned.

Run each Python producer, then:
  cargo test -p aa-rules -p aa-actors
  cargo test -p aa-rules --test phase3_local -- --ignored
  cargo clippy -p aa-rules -p aa-actors --all-targets -- -D warnings
  cargo run -p aa-rules --example native_actor_fields -- ../Data.rwd ../reconstruction/native-game/runtime-expanded-v14

Original binaries/assets/profiles/saves and prior releases remain unchanged.
