phase8-nif: AA.exe NiStream NIF reader port (all shipped NIF versions)
=====================================================================

Inventory (every .nif member of the original Data.rwd; all 1900 are byte
identical to reconstruction/assets/original/Data; no v14 overlay or skin
archive contains a NIF):

  header line                                  version     user version  files
  "Gamebryo File Format, Version 10.1.0.0"     0x0A010000  0 (read)       1859
  "NetImmerse File Format, Version 10.0.1.0"   0x0A000100  not stored      31
  "NetImmerse File Format, Version 4.2.2.0"    0x04020200  not stored      10

The native LoadHeader 8810F0 accepts 3.3.0.11 (B323C4) ..= 10.1.0.0 (B323C8)
and reads the user version only for versions >= 10.0.1.8; the accepted user
version range B44174..B44178 is never written, so it is 0..=0.

Native oracle (Unicorn, original AA.exe unchanged)
--------------------------------------------------
native_nif_load.py runs the original loader: C++ RTTI static constructors
(872E40 stubs in the initializer table), the animation-key table initializers
(860AA0..865C10), NiInit 860120 over the static-data-manager table (pre-init
8C3ED0 builds the loader map 881600 and registers NiMain classes; 865CA0,
89AB50, 89ACA0, 89B090), the NiStream constructor 8816B0 and
NiStream::Load(char*,int) 87FEF0 -> NiMemStream -> LoadStream 881BD0.
Seams: game allocator 6F7810/6F7430 (bump heap), CRT per-thread data 9738A9
(first-use layout, ANSI single-byte code page), thread id / critical sections.
Any other import, CRT heap use or unmapped access aborts the run.

  native_nif_corpus.py  every shipped NIF: LoadStream result, native error,
                        version, class of every object, top-level objects and
                        SHA-256 of the exact NiMemStream::Read trace (offset,
                        requested, delivered, object index, link-list
                        lengths), link-ID list, link-count list and LinkObject
                        pop sequence -> native-nif-corpus.json (+ .tsv via
                        corpus_tsv.py), native-nif-samples.json.
  native_nif_fields.py  the native object graph after linking (object memory
                        read at the LoadBinary/LinkObject offsets: names,
                        controllers, AV flags/transform/property list, node
                        children/effects, geometry data, controller flags and
                        times, property values, textures, texturing maps)
                        -> native-nif-fields.tsv, native-nif-fields-samples.json.
  compare_trace.py      first divergence between a Rust trace dump and native.

Result: all 1900 files load natively (LoadStream returns 1, error 0, every
byte consumed, no short read, every pushed link ID/count consumed). 4627
executed basic blocks are pinned identical in the accepted v14 image.

Rust port: crates/aa-formats/src/nif
------------------------------------
mod.rs      LoadHeader 8810F0, GetLine 876020, class table 881960, group
            table 880BB0, old inline class names 881AF0, object loop of
            881BD0, top-level objects 8812F0, loader map lookup (8EB090/8EB0C0).
stream.rs   the read primitives (one method per native helper: 869420,
            870C10, 880EE0, 880A40, 880AA0, 880B70) and the optional trace.
classes/    LoadBinary of the 76 classes the corpus streams, every version
            branch (registry of all 143 registered names; the other classes
            are explicit UnportedClass errors).
link.rs     LinkObject list order: 86F3A0 (properties, head insert from
            4.1.0.8), 8678D0 (child slots, effects), 86B0E0, 89F160.
rtti.rs     the native NiRTTI hierarchy (183 classes) for NiIsKindOf.

Tests (cargo test -p aa-formats):
  tests/nif_native_corpus.rs  all 1900 files: read trace, classes, link IDs,
                              link counts, pop sequence, top-level objects
                              identical to the native oracle.
  tests/nif_native_fields.rs  all 1900 files: the native object graph
                              (1896 field by field, the 4 post-processed
                              files by class); ThingStartLocation.nif values.
  nif::tests                  header and registry error paths.

Model loading (crates/aa-client): model_payload_backend.rs parses the actual
selected bytes with the port, accepts every natively loadable version, and
checks the graph projection against it (kind, name, file flags, transform
words, property link order, controller file flags, vertex/normal/bound words);
nodes keep the native stored flags, property order and controller flags.
model_graph_fields.rs computes the property/effect/geometry field projection
at load time for every model (the 3-model sidecar is only a test fixture now).

Native model update (controllers, world transforms, bounds)
----------------------------------------------------------
native_model_update.py  loads every shipped NIF with the original NiStream at
                        PC24 (0x007F), PC53 (0x027F) and PC64 (0x037F), runs
                        the start-up initializers 9E8380/9E83A0 (B45E38/B45E3C)
                        at PC53 and the original NiAVObject::Update 86E8D0
                        (0.0, 1) on the first top-level object (87ABC0, the
                        first model-processing update with controllers), and
                        records per object: NiAVObject flags, local, world
                        and world bound; NiMaterialProperty words; controller
                        clock words +0x20..+0x2C and +0x40; NiKeyframeController
                        and NiAlphaController last-key indices; Euler per-axis
                        last keys; the B44630.. clock cache (own column). Reads
                        of .bss bytes nothing wrote are logged per file: only
                        zero-initialised never-written globals (B43CF8, B43E68,
                        B43F20), the clock cache, CRT fmod/mbc state and the
                        texture/user-version globals the loader already reads.
                        -> native-model-update.tsv (5700 rows: 5688 ok, 12
                        post-processed), native-model-update-samples.json.

Rust: crates/aa-client/src/model_controllers.rs (module of
model_payload_backend.rs): 866BE0 (Property chains via 86A520, then the own
chain), NiKeyframeController::Update 8C0200, NiAlphaController::Update
8C2740, DontDoUpdate 87C780, the aa-animation port of the clock 89E9B0, key
search 860890/861000/862170, float linear 8B7CC0 and quadratic 8B7CE0,
position linear 863D10 and quadratic 861720 (+ load fill 861900), rotation
linear 864120 (blend 8B7F80 + normalize 8B7E20, load fill 862490), Euler
862FE0 (871130 FSIN/FCOS enclosure, 870E80), 4465A0 quaternion -> matrix,
and the metadata visitor 872650/8785D0 (APP_INIT diagnostic 0x64E, largest
stop time into metadata +4). The backend runs the controllers in preorder
before each source object's world transform in the updates with controllers
(87ABC0 and 5AA0DD) and writes the target locals / material alpha.
Test model_payload_backend::controllers::tests::
every_shipped_model_update_matches_the_native_update (ignored; needs
AA_GAME_ROOT): 3615 file/precision updates identical to the native digest
(831 with time controllers); the rest are explicit unported domains (skins,
NiBillboardNode, NiCamera, NiParticleSystem, dynamic effects, LookAt/Path/
MaterialColor/TextureTransform/Vis controllers, multi-key TBC rotations in
USAInfantryCorpse03.nif, the 4 post-processed files).

Load-time model graph: crates/aa-client/src/model_native_graph.rs builds the
source graph of every selected model from the native object graph (root =
the first top-level object, as 5A7120/5A7175 take it; NiNode child slots
after the 867C40/86EA10/866DF0 re-parenting, computed in aa-formats
link::link_children). The assets-cache graph JSON is no longer read. The 265
models without a former projection are the 265 files whose footer lists a
second top-level object that is also a child of the first (PyFFI walked it
twice); natively no object is linked twice in the corpus. Test
model_payload_backend::tests::native_graph_matches_every_former_projection:
all 1582 former projections are reproduced field by field (PyFFI's "" for a
NULL LoadCString descriptor name normalised).

KF files (sequence resources)
-----------------------------
581 .kf members: 568 x 10.1.0.0, 13 x 10.0.1.0; classes NiControllerSequence
(the one top-level object), NiTextKeyExtraData, NiKeyframeController,
NiKeyframeData. NiControllerSequence::LoadBinary 883650 / LinkObject 883730
are ported (classes/controller.rs, link.rs sequence_controllers).
NIF_CORPUS_EXT=.kf runs native_nif_corpus.py / corpus_tsv.py /
native_nif_fields.py over them -> native-kf-corpus.json/.tsv,
native-kf-fields.tsv (+ samples); the aa-formats tests
every_shipped_kf_matches_the_native_stream_reader and
every_shipped_kf_builds_the_native_object_graph compare all 581 (the field
records now include NiTextKeyExtraData keys, so the 101 NIFs with text keys
were refreshed in native-nif-fields.tsv).
native_kf_length.py runs the original 5A5650 ComputeLatestTimeController on
each natively loaded root (404ED0 as a recording seam) ->
native-kf-length.tsv; aa-client sequence_decode::tests compares lengths and
reported lines for all 581 (7 files report 0x733, total length 0).
The resource load 5A8190 is aa-runtime sequence_resource_load.rs; the
decode service 5A7230 + 5A5650 is aa-client sequence_decode.rs.

Open gaps (explicit errors, not ported)
---------------------------------------
* Old-style particle systems: 8E93B0 (registered by 89B090) converts them
  after linking (4 shipped files). NifFile::requires_native_post_process.
* NiStream texture palette (+0x194, 8806D0/8806C0, 880A00) can make a later
  NiSourceTexture resolve to an earlier one with the same name.
* AddExtraData 86AE60/86AC80 naming of unnamed extra data and array order.
* NiTimeController::LinkObject below 10.0.1.16 marks the target's geometry
  data for "vertex controllers" (vtable +0x48; in the corpus only
  NiParticleSystemController, which is in the post-processed files).
* PostLinkObject (vtable +0x2C) side effects are not ported.
* Native out-of-range class indices, group IDs, link IDs, short reads and
  oversized stack buffers (class names > 255, > 15 pixel-data mipmaps) are
  errors here; the native code has undefined behaviour there.
* Controllers other than NiKeyframeController/NiAlphaController, multi-key
  TBC rotation/position/float keys, quadratic rotation keys, and the reverse
  cycle bit (the shared 89E9B0 cache does not key it) are explicit errors.
  87C780's second comparison uses the unrounded 89E9B0 result; when the
  stored result equals +0x2C it is an explicit error (never reached at the
  model-processing time 0).
