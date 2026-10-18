Native soft descriptor and distinct movement range boundaries

aa_spatial::soft_range::capture implements4EA0F0: snapshot native XY and
facing, then call actor virtual+10, retaining the returned shape identity.
Ordinary4EA020 returns source definition+50, regardless of target override
byte80. This is a soft shape; target/range virtual+14 is a different getter.
The runtime must not apply mesh bounds or per-object rendering scale.

source_first_in_range implements the geometry branch selected by52FE60:
capture source before target, retain both shape handles until both getters
finish, and call existing native46F100 with TARGET as receiver and SOURCE as
the other descriptor, ordinary (not inverted) range mode. It intentionally
does not repeat52FE60's earlier near-point, target-GTID or flag10 gates; those
already live in element_move::think. The caller supplies0x3DCCCCCD for that
original arrival branch.

source_receiver_in_range implements the different577530..57757D fragment
inside Mover5774A0: the soft descriptors are still captured SOURCE first,
TARGET second, but SOURCE is the46F100 receiver. It excludes the enclosing
movement admission, target validity and later motion. It is also distinct from
4EE800, which captures TARGET first and uses virtual+14 target shapes.
336 additional original fragment executions cover the same mutable/aliased
inputs and six equal-kind/equal-radius rectangles whose different bounds make
swapping the receiver change the result. These six witnesses catch accidental
reuse of the arrival helper. The original330-case corpus remains unchanged.

World::Actor is associated and Copy, so ActorStore::ObjectKey can be passed
without narrowing its64-bit identity. A concrete adapter should read placement
through ActorStore::placement(key,Spatial), and use the retained source-native
definition identity for ShapeHandle. It must not create a second placement
or resolve/clone the first shape before the second getter has run.
Source ordinary data bindings and derived getter implementations remain the
runtime adapter's required services; no successful default World is provided.

330 original native executions across PC24/53/64 passed. They run full52FE60
with a prepared distant destination and a valid flag10 target, forcing its
soft-descriptor branch. Actual4EA0F0 and46F100 execute. Ordinary4EA020 runs
unhooked; derived mutable+10 getters are explicit fixture callbacks. Tests
cover all four shape types, both descriptor orders, unequal geometry,
target override enabled, position/facing/shape mutation and shared aliases.
Rust tests use actor identities above2^40 to verify no narrowing is required.
A second test makes the target getter fail after the source getter changes
a shape: the first effect remains committed and no shape reads/comparison
occur. Three tests and strict scoped Clippy pass over666 native executions.

Limits: finite explicit nearest precision only, consistent with existing range
helpers. Actor and shape identity lifetimes must survive capture/comparison.
Native stack assertions are harness checks, not Rust ABI claims. No full live
movement/match/world binding or arbitrary derived actor implementation is
established by this geometry wrapper. Runtime integration and independent
review remain separate gates.

Commands:
python research/phase7-soft-range/native_soft_range.py
python research/phase7-soft-range/native_mover_range.py
cargo test -p aa-spatial --test soft_range_native
cargo clippy -p aa-spatial --all-targets --no-deps -- -D warnings
