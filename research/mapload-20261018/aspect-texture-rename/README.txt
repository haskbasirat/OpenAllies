Aspect TextureRename source chain (46A8A0 list, 46A180, 469D10)

Problem: pre-world source preparation of gbr_brigade_engineer stopped at
data_aspect.rs "46A180 positive TextureRename requires actual469D10 source
producer". Its AspectNormal (Buildings/GBR/gbr_brigade_engineer.tgi) has
skin=%sBase.dds (AABuildingPackable template), glow NULL and one
[TextureRename] substring_from=DeathArcticBase substring_to=Base.

Native facts (disassembly + oracle):
- 49C670 constructs the Aspect (46A510: +0 = copy of ObjectData+B0
  resource_prefix, +C IDS), 469A70 stores +44 = &ObjectData+B4 (the
  TextureSearchPath list), then 46A8A0 loads the source node.
- 46A8A0 reads aspect_type, aspect_prefix(+4), visual_radius_estimate,
  model(+20), weight, then EVERY TextureRename child: operator new(0xC),
  node {+0 substring_from, +4 substring_to, +8 next}; node+8 = old +3C,
  +3C = node, +40 = node only when +40 was null. The list is therefore
  PREPENDED: head = last source child. Then skin -> 46A180(skin,1) -> +30,
  glow -> 46A180(glow,1) -> +34, then Animation, ground_size, disable_animations.
- 46A180(this=Aspect, CString* source, flag): empty source returns 0 before
  anything (no copy, no 5A79F0). Otherwise copy = source (shared), and for
  node=+3C..next: copy.Replace(node+0, node+4) (469D10). flag1: Format(copy,
  Aspect+0 + Aspect+4) via 4108C0/405D60, with NO '/' special case (unlike
  the model path). flag0: assign the copy. Then 4763C0 (relative to the
  current 0xB36530 context folder) and return 5A79F0(name, Aspect+44).
  An empty positive name (rename/format to nothing) IS requested.
- 46AE20 (loaded Model extra textures, called from 46AEC0) calls
  46A180(model texture name, 0) for each Model texture: the second consumer
  of the stored list.
- 469D10 is ATL CStringT::Replace: count pass, PrepareWrite2(max(new,old)),
  replace pass over NUL-separated segments up to the ORIGINAL length, and the
  published length is the count-pass value.

Oracle: native_aspect_texture_rename.py -> native-aspect-texture-rename.json
(--write authors, default replays and asserts equality). Original and v14
images x Single24/Double53/Extended64 = 6 machines per row, all equal.
- replace: 35 rows of real 469D10: the gbr_brigade_engineer skin and NIF
  names (no match), DeathArcticBase name, temperate_hedge pairs, grow/
  shrink/equal under Fork (refs2), Reallocate (refs1 exact) and in-place
  (refs1 spare), delete, overlap, pattern-in-replacement, entire, case,
  empty/NULL old and new, empty source, format text, surrogate pair,
  embedded-NUL segments. Two embedded-NUL grow rows publish storage nobody
  wrote (poisoned 0xCDCD units): a later segment shifted beyond the
  original end is never visited, yet the count-pass length is published.
- loader: 7 cases of real 49C670->46A510->469A70->46A8A0->46A180->469D10->
  4108C0->4763C0, incl. the real gbr_brigade_engineer AspectNormal, a
  three-rename order case (skin %sBase.dds -> %sX.dds proves reverse
  order), absolute aspect_prefix (no '/' rule in 46A180), rename that
  empties the skin (5A79F0 receives ""), empty skin with renames (no
  request) and empty folder. Then real 46A180(flag0) on the SAME Aspect for
  the actual GBRBrigadeEngineer NIF texture names (46AE20 form).
Seams (explicit): CString manager vtable (Allocate/Free/Reallocate/Nil/
Clone), 856370, 5B54D0/5B4AE0/5B49D0/5B4D80/5B5380/5B4220/42BE10 typed
getters and list iteration, 497220 aspect_type map (fixed index 7), 6F7810
heap, 405D60 printf (%s/%% only), 424C10 trim, 85A030 identity, 5A79F0
Texture request (records name + search owner, returns a sentinel), 410C40
(must not be reached), 49C360 append. 1226 executed instruction bytes are
pinned equal in both images.

Rust (crates/aa-runtime/src/data_aspect_rename.rs, used by data_aspect.rs):
TextureRenames (traversal order), replace_469d10 (exact buffer model with
unwritten-storage tracking), TextureRenames::texture_name_46a180. The
Aspect Input now stores the rename list, aspect_prefix and the exact
5A79F0 names for +30/+34 (None = native null). Input::texture_name_46a180
(flag0/flag1) is the entry a future 46AE20 port uses.

Tests: data_aspect_rename_tests.rs replays every replace/loader row and the
real gbr_brigade_engineer input; data_aspect_tests.rs adds the empty-name
refusal and an ignored AA_GAME_ROOT lab that prepares the actual
gbr_brigade_engineer Aspects.

Not proven / open:
- 405D60 printf beyond one %s/%% (existing restricted path::expand domain).
- 4763C0 folder = definition source folder assumption (existing; native
  reads the CURRENT 0xB36530 context at call time - a 46AE20 port must pass
  the context current at request time).
- 5A79F0/5A77E0 with an empty name: refused explicitly in Rust.
- Negative memmove balance in 469D10: guarded as an error; no input that
  reaches it was found (brute force over a/b/NUL strings up to length 10).
- 46AE20 itself (Model texture enumeration 5A4ED0/5A5590/5A55A0) is still
  the existing explicit error in data_aspect.rs request_extra.
