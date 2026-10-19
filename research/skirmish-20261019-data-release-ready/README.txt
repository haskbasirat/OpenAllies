Data vf18 release / vf1C ready family (including ActorData+534), 2026-10-19

Runtime callers (native)
  45B530 VObject common init: data->vf14(1,0) pre-request, VObject+54 |= bit28,
    view appended to the WorldView request queue (+208).
  43EC60 WorldView update, every frame: while the queue head's VObject vf74
    (45B140 = tail jump to data->vf1C) is true, 45B720: data->vf18(1) and
    clear bit28, unlink the view. Stops at the first not-ready view.
  45BCA0 VObject uninit (from 4450E0 / 458CE0 view destructors): a view still
    flagged bit28 releases with data->vf18(1) and leaves the queue (43C490).
  Pack/unpack and destroy reach these through view creation/destruction; the
  gameplay readers of +534/+548 (575BB0/575C00 pack/unpack predicates, SAI
  7xxxxx, 4E1540, 56F960, ...) read the identities only and are bound through
  aa_rules::native_mobile_deployment (stored/saved actions adapters).

Native functions ported (all original and v14 bytes identical)
  ObjectData vf18 49BB00: 4BB050, +104 vf18, +134 46C7D0, aspects 469BB0
    (count +110 reread), +EC/+F0 vf14, +138 vf14, 428900 x6 (+BC+8i).
  ObjectData vf1C 49BBB0: 4BB060, +104 vf1C, +134 46C800, aspects 469C50
    (reread), +EC/+F0/+138 null-or-loaded, 428950 x6; first false returns.
  ActorData vf18 45DF40 (A05540+18): 49BB00, animation sets (both counts
    reread) vf4, 428900 x8 Actor FX pairs, abilities 4BE450 (counts
    captured), +2A4, then bit19 && +534 -> (+534)->vf18(delta). No bit2 pass.
  ActorData vf1C 45EF20 (A05540+1C): 49BBB0, 45EB10 per animation, 428950
    x8, 4BE480, +2A4, bit19 && +534 -> (+534)->vf1C().
  428900/428950 (cells {node,next}; vf24 CreateVFX -> +3C vf18/vf1C, vf2C
    PlaySFX -> +44 vf18/vf1C), 4BE450/4BE480 (Definition+70+8i, 5 lists),
  VFX 494390/494450 (4BB050 [+40 vf14]) and 4943A0/494810 (4BB060 [&& +40
    null or loaded]), AudioData 46B260 / 46B060 (lists +34/+40/+4C),
  Aspect 469BB0 (+18 -= delta, clamp<0 to 0; model vf14; +38 49B820; skins
    +30/+34; +54 -> 5A4ED0 Model+7C extras, +54 cleared at count 0) / 469C50,
  Banner 46C7D0/46C800 -> Plane 46C860/46C890 -> Decal 46C920/46C950,
  Actor Animation vf4 45EAC0 / 45EB10 (+34 Sequence, lists +38/+40/+48, +50
    timed chain).
  Resource vf14 is 5AA550 for every Resource vtable; it is NOT ported here:
    it decrements +4 and at zero calls 5AEE70 ReleaseResource (abort load task
    or 5AC3D0 unload) - ResourceManager owner work.

Proof
  native_data_release_ready.py, 921 rows at PC24/53/64 (identical original
  and v14 outputs), 1025 instruction pins. Coordinators run with recorded
  child seams (mutations prove rereads/captures; every child as the first
  false; release stops at every child). Kernels run natively with only
  Resource vf14 recorded and flags+8 in memory.
  Rust replays: data_release_fields_tests (coordinators),
  data_release_kernels_tests (every kernel row).

Rust
  data_release_fields.rs   coordinators (traits ObjectRelease/ActorRelease)
  data_release_kernels.rs  leaf kernels + ResourceRelease
  data_object_release.rs   State::release_object / State::object_ready over
                           the prepared Data allocations; ReleaseServices
                           (5AA550 seam) with RequiredReleaseServices default
  data_aspect_release.rs, data_banner_release.rs, object_animation_release.rs,
  actor_animation_release.rs: adapters over the SAME prepared sources.
  data_actor_request.rs: actor_fx_graph / actor_ability_graph accessors.

Results (target/agent-settlement)
  coordinators replay pass, kernels replay pass, full aa-runtime lib tests
  467 passed (1 unrelated native_camera failure in another owner's work).
  Real data: rus_truck_corps prepared, pre-requested (laboratory task queue,
  no decoding), vf1C=false (nothing loaded), vf18(1) recorded 68 Resource
  releases: the truck's then rus_corps' (+534 tail); both Data+20 back to 0.

Open
  - 5AA550/5AEE70 Resource release belongs to the ResourceManager owner;
    release_object stops at the first Resource with RequiredReleaseServices.
  - Aspect +54/+48 extras (46AE20) are unported upstream; reached -> error.
  - The client does not yet port 43EC60/45B720/45BCA0 (its queue is filled
    but never polled); it should call object_ready then release_object(1).
