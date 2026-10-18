Movement actor context554350

Production: crates/aa-spatial/src/actor_context.rs
prepare(index,actor,strict,policy)->Result<PreparedQuery> is the explicit native
entry used by movement callers. Assembly554350..5543E3 first initializes the
descriptor and then directly calls5531F0(actor,strict) at5543D8. All default
shape/pose fields are overwritten by that call for ordinary immutable actor
getters. Therefore this entry deliberately reuses the exact shared helper with
extra_flags=0. This relationship is recovered from original bytes, not inferred
from a similar query. In particular it does not add57C080's extra0x400 bits.

The resulting descriptor uses the actual blocking shape (virtual+C), source
actor/definition, native definition snapping, actor facing, and5522D0 masks.
All ignore/owner/hostility/custom fields are initialized to their original
values. set_facing ports5522B0 raw facing writes; PreparedQuery computes its
orientation on use, so no native orientation-cache value can remain stale.
No direction normalization or empty-query fallback is introduced.

SpatialIndex remains the authoritative ordinary actor shape/transform source.
Arbitrary derived mutable shape getters and observation of a partially built
descriptor are outside that existing API domain. Native global defaults are
read by554350 before being overwritten; the fixture varies them to prove they
do not affect the supported result. This proof is not an implementation of the
global registry's startup lifecycle. A missing actor is an explicit error.

original_context.py executes actual554350, its original5531F0/5522D0/552250
callees, ordinary blocking-shape getter and5522B0 setter.384 cases cover all
PC modes, shape kinds0..3, source actor flags10/12/18/21, both strict values,
definition snap16, ObjectData-view presence, poison-filled descriptor memory,
nonzero startup defaults and zero/nonunit/signed-zero facing. Complete portable
descriptor fields match Rust. Native internal self-pointer and cache reset are
also asserted by the producer. Only existing prepared definition-view/policy
fixtures and setup allocator leaves remain explicit boundaries.

Reproduce:
 python research/phase7-spatial-context/original_context.py
 cargo test -p aa-spatial actor_context::tests -- --nocapture
 cargo clippy -p aa-spatial --all-targets --no-deps -- -D warnings
All three tests and strict Clippy pass; logs are preserved. No original game/save/
release files are changed. This closes the caller-context kernel needed to
compose real531170/5312C0 with sweep553CE0, not every remaining movement service.
Independent peer review is separate from the producer findings.json.

Caller arithmetic helpers
current_actor_goal ports531170: both direction*distance products store to
binary32 before position addition. explicit_origin_goal ports5312C0: X product
stays extended through origin addition while Y product stores first.672 original
x86 fragment cases validate both helpers across all PC modes, fractional and
large coordinates, zero/signed-zero and nonunit direction. The corpus includes
inputs that distinguish the two store patterns. These are arithmetic fragments,
not a claim that complete531170/5312C0 world dispatch executed in this fixture.
The runtime adapter preserves descriptor-first and live actor/target lookup
ordering before consuming these pure functions.
