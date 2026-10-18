Concrete battle scene orchestration — 2026-10-17

Source composition, currently awaiting the genuine asset/backend entry and
actual activated-World execution. A source check is not a playable-battle or
visual-parity result.

NEW aa-client/src/battle_scene_startup.rs retains the same pre-7010D0 camera
handle and real backend receipts beside the one VWorld allocation. It calls:
  1. assets.require_initialized (actual source/world owner);
  2. initialize_scene_from_assets using same process camera and source Seeds;
  3. initialize_grid_and_heightfield using current UI terrain-quad reads;
  4. Prepared::from_world from that same sampled heightfield and same Arc store;
  5. Pending::request -> static Request.load -> upload_with_terrain, one upload;
  6. Pending.accept, install_terrain, actual scene Request/install and adoption;
  7. same process visual-map publication at the documented modern terrain stage;
  8. initialize_tail_children_with_inputs: captured-camera6FF130, diagnostic,
     actual auxiliary/helper owners, actual trigger collection and requests.

The common runtime tail coordinator is unchanged except an additive scoped
readonly callback loan of the SAME Inputs and heightfield. The legacy entry
delegates to that one body. The loan ends before helper_globals mutation.

The main frontend retains a separate UI-only CpuScene; viewer inspection data
never substitutes for it. UiBridge reads the same retained Arc<AssetStore> and
current configured modern LoadSettings once at preparation. The World and its
authoritative ActorStore never cross threads. Failed mutable native fields,
captured camera and rejected retained terrain/scene bindings remain reachable.

No AD writer, resource/task completion, UI-ready status, rendering enablement
or order enablement is provided by this module. Root owns the full specialized
completion guard. Mandatory assets initialization remains before this module;
there is no successful empty asset/backend replacement.

Validation log client-check01.log is a preserved earlier snapshot failure in
foreign in-flight texture_resource_load::load / audio_data_pre_request source.
It is not a successful compile and did not reach client type checking.

Native numeric/source kernels are existing separately proved modules. This
orchestration adds no new native arithmetic. Modern terrain GPU mapping retains
its existing documented visual gaps; no old Ni ABI or complete-render claim.
