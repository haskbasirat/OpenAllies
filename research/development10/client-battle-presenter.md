The battle scene modules were already compiled through `live_scene::battle` and
its nested `native` module. They do not use the obsolete runtime `live_battle`
draft. The missing integration was the move from `Staged` to the retained
`live_world_frame::World` owners, and source/model rebinding after replacement.

The new `World::presentation_parts()` loans the actual ActorStore, adopted
SpatialWorld, source Bank, settings/current variables, ZoneMap, Kingdom registry
and map inventory immutably. It does not expose a constructor, mutable simulation
field, activation capability or tick. BattleScene and NativeColors now accept
that World directly while retaining the existing Staged diagnostics.

`live_scene::presenter::Presenter::bind_current_actor` resolves the current GTID,
canonical definition/source precision, actual aspect/variant and source model
path before changing GPU or picking state. It prepares the actual NIF/material
and textures using the existing source-bound spawn loader. Then it hides the old
binding and appends the new model; the old placeholder's converted placement key
cannot choose the replacement model. Hidden GPU allocations remain until scene
reload. This is presentation residency, not a native object/list lifetime.

The currently admitted model selector is the existing ordinary/default view
branch: actual construction false and Actor stance15C=-1. Positive construction,
stance and derived view branches remain errors. The actual visibility/color
`Presentation` service is mandatory. No local-player, fog, thread/editor or
all-visible default is supplied. Remembered-model selection4EAB20/4EACC0 remains
outside the existing visibility getters. A selector change invalidates the bound
model rather than continuing to draw its previous aspect.

After required model rebinding, main integration should call `seal_current`;
each accepted native update calls `retire_absent` and `present_current` with a
monotonic presentation serial. These calls update only rendering and picking.
Camera controls and inspector `Selection` are not native orders. Issuing orders
still requires the retained Session selection/authorization path and actual
`Frame::with_native_order` loans. `main.rs` has not been changed here.

The pre-unfreeze UI stage can instead borrow the same `WorldFrameInputs::staged()`
through `bind_staged_actor`, `seal_staged`, `present_staged` and
`rendering_stage_staged`. Both entry points share the same source/model binding
algorithm and ActorStore/Spatial witnesses. They never construct a frame World,
rebuild actor state or manufacture an application activation receipt.

`Presenter::rendering_stage` can return a client-private `BoundGpuScene` only
after source/map/owner binding, a successful GPU/picking publication, an attached
matching scene epoch and a current model/owner recheck. The receipt borrows the
exact renderer. It exposes map identity, bound actor IDs and epoch so the view
coordinator can verify its required resource/actor set. Current completeness
conservatively requires every native identity to be classified/bound. An
unsupported derived view is an error; this cannot call74FF50 or unfreeze.

The native completion discovery is separately pinned in
`client-view-init-discovery.json`. Loading Tick682EE0 reads flags15/17/16, then
forwards actual VWorldAD through76ACA0→74FF50. VWorldAD becomes1 at43FF36, after
the substantial43F0A0 initialization body; UI5C3000/40CF00 also has ordered work.
Builder owns the portable view coordinator and Settlement owns the genuine
completion/aggregate/unfreeze protocol. A rendered frame alone is insufficient.

Release/locked/offline client production check03 passed41.33 seconds. The first
check retained the peer PlacementRecord factoring errors; the second passed
before the rendering-stage receipt addition. Current libtest typecheck is
separate and passed31.95 seconds before the staged siblings. The staged siblings
and load_for_battle subsequently passed current production check in1m43s.
No full-screen, GUI or GPU acceptance test was run here.

load_for_battle verifies exact archive identity and original decoded native IDs
against the same inventory. Exclusions include removed original IDs and current
native IDs. Terrain/static geometry, picking and LiveScene remain available;
current dynamic models are bound from the live canonical source.

The concrete client host retains non-Send World on SimulationThread. New
view_directory upload packets transfer only immutable model/pose/source data.
Private replies follow actual model/material loading, GPU upload and picking;
correlation checks current actor allocation, World revision, request, map and
exact renderer allocation/epoch. Same-Application resource registration precedes
upload and its move-only lease remains with the actual view directory. Changed
Biome invalidation retains its +80 assignment then refuses until real modern
resource class/refcount/unload ownership is implemented.

The backend child does not publish Thing+C or certify native view Init. Actual
43C420 flags/list work and derived view/aspect services remain required. The
native publication proof has228 rows/420 executed pins; concrete view
constructors/Init/Delete are named service leaves in that bounded proof.
