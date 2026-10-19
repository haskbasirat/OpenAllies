ObjectAnimation positive CreateVFX/PlaySFX source children - 2026-10-18

Root cause
  Pre-World Aspect source preparation (46A8A0 -> 49DA40 -> 49D900) stopped
  at an explicit stub for any authored [PlaySFX] child of an Object
  [Animation] (51 definitions: 20 general portraits, 31 doodads). The two
  child loaders 49D0A0 (CreateVFX) and 49CF80 (PlaySFX) were not ported.

Native evidence (AA.exe original and accepted v14, identical bytes)
  49D900 calls 49D0A0(node) then 49CF80(node) after start_max, before the
  49DA40 overall/typed appends.
  49CF80: count=5B4220("PlaySFX"); for index 0..count (index>=0):
    grow +2C vector (49B630; new pairs zeroed {0,0}), count+1 stored BEFORE
    the child is read; 42BE10 {node,index,"PlaySFX",count} -> child;
    5B54D0(child,"sfx_ids"); empty -> null WITHOUT 416370; nonempty ->
    416370 with ECX=[[B36530]+584] (same manager slot/call as FXC 42C3C0);
    nonempty miss -> 410C40 "Unknown IDS '%s' for data manager '%s'"
    (manager+20) before the store; store pair+0; null -> 410C40
    "Unknown sfx '%s' specified by object animation '%s'" (Animation+4);
    5B5380(child,"looping") -> pair+4 byte.
  49D0A0: same loop over "CreateVFX" into +20 (877EA0 grow, 79E130 zero):
    4BDB70(vfx_ids) with ECX=[[B36530]+C] -> pair+0, null -> "Unknown vfx
    '%s' specified by object animation '%s'"; 42C070(location) with
    ECX=[[B36530]+580] -> Data+C or -1 -> pair+4, <0 -> "Unknown location
    '%s' specified by object animation '%s'". Same managers/calls as FXC
    42C5E0. 42C070 returns null for an empty IDS at 42C08A (executed here).
  410C40 always throws. 49B7E0 request does not read either vector; 49B720
  (animation start) reads +2C pairs for 7474F0 playback (not ported).
  Timing: 49DA40 runs inside the owner's parse-time LoadData (464BD0/
  4718B0/4922E0 -> 49DC40 -> 49C670 -> 46A8A0), so the Finds see only
  roots registered before the owner, exactly like FXC LoadData.
  483A40 names a DataManager (+20) from the first root dispatched to it.

Oracle
  native_object_animation_children.py -> native-object-animation-children.json
  24 cases x PC24/53/64 x original/v14 (144 loads, 1436 executed pins).
  Actual 49BF20/49C020/49D900/49D0A0/49CF80/49B630/877EA0/49DA40/497220/
  405D60 bodies and the 42C070 empty-IDS entry run. Seams (explicit, each
  recorded): typed getters, 5B4220, 42BE10, 4763C0, 5A80D0, 416370, 4BDB70,
  nonempty 42C070, 410C40 (formatted message), 855D30. The registries and
  translations behind the Finds are the existing FX lookups, not re-proved.
  Replay: python native_object_animation_children.py (compare, read-only);
  --write regenerates (author only). native-replay01.log: PASS.

Rust
  object_animation_core.rs: 49D0A0/49CF80 loops in the shared kernel;
    Animation gains vfx (+20) and sfx (+2C) pair vectors; the Source trait
    exposes 5B4220/42BE10/getters/Finds/manager name as seams.
  object_animation_set.rs: production Source; Finds through
    fx_graph::OwnerLookups at the owner's source position; owner = the one
    registered Thing root whose authored span contains the Animation.
  fx_graph.rs/fx_graph_sound.rs: OwnerLookups wraps the SAME Bound::find
    ("Audio"/"VFX"/"Bone"); audio_manager_name for the 410C40 text.
  Tests: object_animation_core_tests.rs replays both fixtures (old 120 +
    new 144 loads, 54 requests); object_animation_set_tests.rs is an
    AA_GAME_ROOT real-data lab over the 51 owners (see run logs).

Verification (CARGO_TARGET_DIR=target/agent-sfx, --offline -j 6)
  runtime-object-animation-tests01.log: lib filter object_animation
    4 passed, 1 ignored (the real-data lab), incl. the new
    original_positive_create_vfx_play_sfx_children_all_pc.
  runtime-fx-aspect-tests01.log: lib filters fx_graph + data_aspect
    16 passed, 6 ignored (pre-existing AA_GAME_ROOT labs).
  runtime-real-data-lab01.log (AA_GAME_ROOT=E:\Games\Axis and Alies):
    all 51 census owners, 57 positive PlaySFX Animations; each stored pair
    is the registered Audio identity/source and authored looping; the real
    prepare_object_aspects returned Ok for all 51 (no later stop).
  A standalone mutation (empty IDS calling 416370) fails the new replay.
  Missing Sequence (coordinator follow-up): Set::load_source_node_with_policy
    passes the caller's TexturePolicy to Resources::sequence_5a80d0_with_policy
    (5A80D0 TGC_MissingResourceException catch). NULL -> the unchanged kernel
    skips the whole Animation (no 49C020 allocation/reads/appends), as proved
    by native-object-animation.json row 'missing' and this fixture's 'nores'.
    load_source_node (no policy) keeps the explicit RequiredTexturePolicy
    error. runtime-object-animation-tests02.log: 4 passed, 2 ignored.
    runtime-real-data-lab02.log: tropical_bamboothicket/tropical_grasstall
    4 Animations skipped under explicit mode=1; mode=0 rethrows; no policy
    -> DataCompatibilityMode required; the 51-owner PlaySFX lab still PASS.
    Pending one-line caller change in data_aspect.rs (other owner):
    load_source_node(...,resources) -> load_source_node_with_policy(...,
    resources,texture_policy).
  No crates/aa-runtime/tests integration test mentions ObjectAnimation.

Not proved / open
  - The owner position is derived (containing registered root); a
    template-authored Animation with positive children is an explicit
    error. Passing d.proof.source from data_aspect.rs would remove the
    derivation (data_aspect.rs is owned by another session).
  - Manager+20 name: Rust uses the registry tag "Audio" once a registered
    Audio root precedes the owner (all shipped Audio roots spell "Audio");
    before any Audio root it is an explicit error.
  - FX find("VFX") additionally reads the VFX "cutscene" flag (declared by
    the VFX template with a default); 4BDB70 itself does not.
  - 42BC40 "Too many nodes." (index > 9999999) is not modelled.
  - No playback (49B720/7474F0), AudioData pre-request/Sound resource for
    ObjectAnimation SFX, or VFX creation is claimed. No shipped data has an
    ObjectAnimation CreateVFX child; that path is proved only by the oracle.
