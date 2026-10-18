# Held Actor Init(NULL) integration

`crates/aa-runtime/src/load_actor_init_world.rs` connects the existing native
`aa_actors::actor_init_null` 4D7230 coordinator to the current allocation held
outside its exact `Loading` registry slot. It does not create an ActorStore
copy, infer absent components, replace saved state, or publish a battle.

The adapter validates the real canonical source identity, template chain,
category/class, final definition flags and component mask. Definition numbers
use the completed source Bank's precision; live arithmetic keeps its separate
explicit precision. A standalone diagnostic Catalog has no source authority.

Concrete bindings include actual required-object+158 and associated-object+50
fields, slot presence, the actual +108 build-target list refresh, and the
existing 4D1350 action kernel over the same F0/F4 fields. Action clears and
partial updates survive an error in a reached contributor or publication.
Properties InitExisting/AfterInit uses the separate proved concrete component
adapter. Body dispatch uses the separate concrete Body prefix adapter: healthy
and native construction-state-skip paths complete, while damaged-health and
active-Construction services stay explicit with earlier writes retained. Its
actual healthy source case is a neutral Kursk building, not a military unit.
There is no invented Organization or fallback owner field.

The source+4E0 null branch of 4D1FD0 returns true only for the supported actual
null source inputs. Wrong typed IDS are refused. Nonempty authored IDS alone
does not certify a linked native pointer: its real deferred-reference/world
service remains required. Both 4D67F0 and 4D4130 have a concrete bit29 early
return before 4D10A0; their other branches remain required services.

SAI calls use the Catalog's same managed inactive observer owner. TrackInfo is
read lazily after SAI, from the captured current canonical source definition
and existing portable Bank token. Native track arithmetic/order remains in
the established coordinator. Nonempty tracks still require actual GameData
scale/divisor and an owned updatable allocator/constructor.

The reached existing-FX branch binds the shared `definition_fx::Source` at
that point. Only its certified empty `Constructed` (+E4) list authorizes the
native empty-list return. Positive authored entries require their actual FXC
compiler/graph and presentation owner; the adapter never marks those complete
from an authored-entry count or a synthetic empty-list probe.

## Evidence and limits

`native_actor_init_gates.py` independently executes original and v14 images at
Single24, Double53 and Extended64. Its frozen fixture contains 162 paired
cases / 324 native executions and 54 executed instruction pins. Nonnull
required-object branches stop before +158; non-bit29 refresh branches stop
before 4D10A0. These are explicit required boundaries, not substituted effects.
The same fixture pins both class3/class4 virtual+108 to actual 4D1E80 (read E8).

`native-actor-init-gates.json` SHA256:
`0f04b60e09d82551523175f8eccb11f98b9ad8dfd737322adcf77b0b843d871b`.

Actual empty 42A590→429080 and six-list source construction/loading are proved
by the separate shared FX area:
`../skirmish-20261016-load-post-create/native-empty-fx-source.json`.
Source-positive authored lists are not a compiled-graph completeness claim.

`actual-kingdom-actor-tail-v1.json` is historical: it stopped at required FX,
used an explicit Object Init seam and a separate managed inactive test scope.
It is superseded by the v2 same-owner test once that fixture is produced.
The v2 test deliberately retains the explicit Object Init seam and identifies
it in its scope. It exercises the actual loaded first tutorial02 Kingdom and
real Actor tail at all three live PCs, with the same inactive observer owner.
It cannot establish full Object Init, Kingdom Init, map startup or skirmish
readiness. The persistent LoadWorld integration must exercise those together.

Build/test logs retain failed compiler attempts as well as final reruns.
Source pins and accepted test outcomes are sealed separately after the final
guards and shared service modules have compiled. No original game data,
released executable, manifest, lockfile or release archive was edited here.

## Authoritative world version

The immediate lifecycle Context/Inputs now borrow the actual mutable GWorld+2C
word. Dynamic records reborrow LoadContext.world_version, and Team/Kingdom
records reborrow RestoreReport.world_version. `split_current_with_world_version`
returns that same word together with the actual held allocation and registry.
It does not reconstruct a mutable version from immutable inventory metadata.
The broader `split_initialize_parts` also returns the same RNG, message Vec,
presentation Vec and immutable inventory borrows for concrete nested services.

The focused lifecycle suite passed eight tests after this change, including
same-address writes across records (28→29→30) and retention of 29 after a later
PostCreate error. The earlier independent lifecycle review covers the prior
owner/order adapter; a separate read-only review by the component/spatial
author accepted this narrow reference-owner change. The native 636-row tail
coordinator fixture is unchanged by Rust reference plumbing.

## Subsequent concrete component and query binding

The current adapter additionally dispatches the frozen real Body, typed constant-component and Mover adapters. Mover's actual world-version read is lazy through `Services::world_version`; an unbound reached read is an explicit error, not a guessed version. The persistent map owner binds its actual mutable GWorld+2C authority. Properties continues to use its typed owner. Component native/actual proof scopes remain in their own research areas, including nonzero Mover scheduler/state-effect boundaries and positive City AfterInit.

The new overlap owner and concrete ordinary refresh are documented in `overlaps.md`. This supersedes only the former ordinary4D67F0 required-service gap when its real services are bound; other positive refreshes/components/global Property/FX graph and whole-world startup remain separately required. The older frozen Actor-tail v2 fixture retains its explicit ObjectInit seam and cannot certify full initialized world readiness.
