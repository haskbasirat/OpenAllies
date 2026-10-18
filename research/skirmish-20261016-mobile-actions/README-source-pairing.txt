MobileDeployment source identities and pairing, 2026-10-16

Approved extension: NEW aa-rules/src/native_mobile_deployment.rs, its single
lib.rs export, and tests/native_mobile_deployment.rs. Existing generic parsing,
source resolution and runtime/catalog files were not changed by this work.

API
  ActorCatalog::native_mobile_deployment_data(precision)
    -> NativeMobileDeploymentData
  data.definition(canonical_registered_id)
    -> Definition { unpack_to, pack_to, pack_health_min_bits, ... }
References use canonical registered Thing identities, not PE pointer values.
The lookup here is identity lookup: do not apply a second translation to an
already canonical ID. Source fields are matched by registered provenance, so
a registered owner's IDS later shadowed by a translation still loads that
owner's own definition. Authored target references use the native Thing Find
translation behavior before pairing. Unknown/non-ActorData targets error.

Source-field scope
ActorData constructor4624B0 initializes +534/+548/+54C to zero. The relevant
464BD0 block is468536..4686D9 (actor-loader-assembly.txt):
- MobileDeployment child sets bit19;
- empty unpack_to_actor_ids stores fixed pack_health_min at+54C (468590..59C);
- nonempty IDS queues a473AD0 reference and stores packed_position_offset
  at+538/+53C, leaving constructor+54C unchanged.
The provider projects these fields and the Body/MobileDeployment/Element
presence bits needed by4605B0. It does not claim to repeat every ActorData
LoadData validation, including packing durations and orientation validation.
Performer ability counts are not invented to obtain the unrelated mask bits.
ObjectData+54 comes from the existing source-bound native_object_definition
shape provider only for targets that reach shape validation.

Pairing and order
473AD0 prepends a deferred reference at GameData+824.475230 walks head first
after the data files load, so mobile callbacks execute in reverse creation
order. The provider requires the native registry loader and uses its registered
Thing order; an arbitrary filesystem or from_files scan is rejected.
4605B0 runs these steps:
1.4BDBA0 must return ActorData; target must have Body. Errors here leave+534.
2.Store source+534=target, then require target MobileDeployment and matching
  Element presence on source/target. These failures preserve the forward write.
3.Compare sqrt(x*x+y*y) with target soft radius+54. Ordered greater fails;
  less/equal/unordered succeed. Operations retain explicit x87 PC24/53/64.
4.Store target+548=source. A later callback replaces an earlier inverse value.
The public resolve_reference method preserves these partial-write semantics;
the convenience whole-source projection returns an error without installing
an incomplete projection into any runtime.

Proof
native_mobile_pairing.py executes ORIGINAL473AD0/475230, the actual callback
wrapper461840 and4605B0 under Unicorn. Only4BDBA0 identity lookup is a fixture
seam;410C40 stops on native data errors and leaves prior writes observable.
Allocator/CString helpers reuse the pinned conversion machine.49 native cases
cover all validation gates, PC24/53/64 shape boundaries, NaN/infinity outcomes,
two sources overwriting one target, aliases, ordered failures and chained links.
All167 observed instruction sequences match original and accepted v14 bytes.
The source-field projection and its constructor initialization are additionally
anchored to the original loader/constructor instructions above; the whole
464BD0 routine is not executed by this narrow pairing oracle.

Rust validation
-49 native pairing cases replay output references, failures, partial writes
 and deferred callback order.
-Synthetic native-registry source test deliberately shadows an existing
 owner's name, aliases the target, and checks the owner is not redirected;
 it checks inverse overwrite order and the conditional+54C assignment.
-Real original/v14 source check succeeds for1028 ActorData definitions and
 all75 forward MobileDeployment callbacks/pairs. The source snapshot is
 shipped-mobile-source.json; this is source evidence, not a native save.
-Existing generic parser/definition semantics are unchanged. Root owns the
 final full aa-rules/aa-metagame compatibility runs after all providers freeze.

Reproduce with AA_GAME_ROOT=E:\Games\Axis and Alies,
CARGO_TARGET_DIR=target/s-actorcreate:
  cargo test -p aa-rules --test native_mobile_deployment --release --locked --offline -j4 -- --include-ignored --nocapture
Set PYTHONPATH to reconstruction/tools/python-packages:
  python research/skirmish-20261016-mobile-actions/native_mobile_pairing.py
Default compares the fixture. --write explicitly regenerates it. Optional
AA_EXPORT_MOBILE_SOURCE=<new path> exports the source snapshot with create_new;
no original input or existing evidence file is overwritten by that diagnostic.
Backup: backups/source-before-mobile-definition-projection-20261016.zip.
