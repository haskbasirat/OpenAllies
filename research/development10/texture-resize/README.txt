Texture power-of-two pad: 5A6A10 -> 87C5C0 / 87C280 / 875AC0
=============================================================

Native (original = v14 bytes)
-----------------------------
5A6A10 (already ported in texture_load_kernel::normalize) calls
87C5C0(Pixel40, &{pw,ph} floats, &{0,0,0,0} border, &B323B4 prefs).
87C5C0: new 0x4C NiSourceTexture (87BDD0, list append 87B930), +20/+24/+28
  = prefs (B323B4 image words 6,2,3), 87C280(old pixel, target, border);
  B323B0 != 0 (before renderer init 710433) also calls the new source's
  vf4C and returns NULL if it fails.
87C280: new 0x50 NiPixelData via 875AC0(ftol w, ftol h, old format +0C,
  old level count +44), Source+40 = it (refcounted), palette +30 shared;
  border colour from the floats (here 0); for every level, for x < new
  width (outer) and y < new height (inner): x < old width and y < old
  height copies bytes-per-pixel (+48) from the old texel, otherwise stores
  the border (format type 0: 3 bytes, type 1: 4 bytes, other types: none);
  PixelData+4C revision += 1. A top-left pad, never a scale.
The PNG reader canonicalizes every PNG to RGBA (type 1), so the 42x42 RGB
HQ icons become 64x64 RGBA with transparent black padding.

Port
----
aa-runtime resource_texture_owner.rs: State::resize_87c5c0 (new Source +
  padded Pixel in the SAME arena; B323B0 set, compressed or palettized
  formats refuse explicitly; the decoded RGBA view pads with (0,0,0,0) for
  type 1 and opaque black for type 0); Payload::Padded; B323B4 cell.
aa-runtime texture_pixel_layout_core.rs: allocate_875ac0 (the existing
  875AC0 layout binding).
aa-runtime resource_depot.rs: Resources::resize_texture_source_87c5c0.
aa-client battle_texture_services.rs: Backend::resize_87c5c0 calls it; the
  kernel then releases the old source, retains the new and stores U/V.
No PixelData revision field exists in the port (no reader).

Proof
-----
native_texture_resize.py (immutable replay; --write): 8 cases x 3 PCs x 2
images = 48 machines: IconCorps.png and IconBunker.png (real 42x42 bytes,
PNG decoded independently in the script, alpha 255) -> 64x64; RGBA 3x5,
RGB 5x3, RGBA 6x5 with three levels, RGB 42x42, RGBA 1x3 with two levels,
and DXT1 6x6 (bytes-per-pixel 0: nothing copied, borders unwritten).
Rust: resource_texture_owner::tests::native_87c5c0_pads_layout_and_texels_
all_pc replays every row (layout words, stride, refcounts, prefs, every
texel; DXT1 and B323B0=1 refuse).

Open gaps
---------
- The port's PNG decoder output for the two icons is not compared here
  with the script's decode (the replay uses the script bytes).
- vf4C with B323B0 set, compressed/palettized pads stay explicit.
