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
* Model graph projection (assets-cache/model-native-graphs) exists for 1582
  of the 1847 audited models; the 265 with an AV object linked twice have
  none (native NiNode SetAt 867C40 re-parents through 86EA10).
