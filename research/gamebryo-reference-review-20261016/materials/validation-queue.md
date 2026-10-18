# Bounded next validations

This is a research queue, not implemented behavior or an instruction to integrate SDK code. Keep original AA instructions/native fixtures as the authority. Use SDK facts from the member-hash inventory to design cases and locate fields. All addresses below are leads from the already-exported original AA pseudocode or pre-existing native evidence; new leads have not been executed or instruction-verified in this review.

## 1. Preserve and verify the metadata actually exposed by the corpus

The new census binds every original source hash and each historical exported mesh's source ordinal, hierarchy, name and vertex/triangle counts to the phase6 sidecar. It records absent properties explicitly and separates all exported rows, retained rows, and retained non-team rows. Before expanding the runtime schema, use its concrete examples to define the minimal required material fields, including the distinction between unused authored vertex colors and colors routed into actual lighting.

The next exporter experiment should write a new research sidecar, not overwrite original/converted/release assets. Preserve effective texturing apply mode, every present map's source/UV index/clamp/filter/transform, material ambient/specular/shine, vertex-color source/lighting, shade flags and complete stencil/draw state. Keep source bytes/hashes and property-owner identity so whole-property inheritance and later dynamic updates remain distinguishable. A missing or out-of-range UV stream must receive an explicit native-supported policy; the historical converter's silent index cap is not that proof.

## 2. Sampler and transform tests offer a small, concrete native target

Original `0x73A7C0` binds a map through helpers `0x8A07E0` and `0x8A0830`, and conditionally forwards a texture transform. Existing decompilation leads to table readers `0x88B8A0` / `0x88B8C0` and tables at `0xB443D0` / `0xB44388`. These are useful starting points for a sampler-state oracle. Confirm calling conventions and table initialization in the instructions; do not interpret uninitialized image data as final table contents.

Capture all four clamp codes and six filter codes, including minification, magnification and mip selection separately. Include absent texture/map, UV sets 0 and 1, a clearly out-of-range UV request, existing initialized renderer state versus a changed state, and transformed versus untransformed maps. Hook only external device sinks/allocations with documented seams. Test stage-local state so one material cannot inherit a previous material's sampler accidentally. A 2×2 distinct-color texture plus a known mip chain can then test boundary coordinates and mip selection on the GPU.

The transform matrix builder lead is `0x8A66D0`. The docs identify two method selectors, a center, translation, scale and rotation. First test identity, translation, nonuniform scale, non-default center, negative scale, dirty-to-clean transitions and both methods at zero rotation. Inspect the native instructions before porting: Ghidra currently renders the method selector as tiny float values, which is a type-recovery artifact, not an SDK floating-point enum. For nonzero rotation this function uses x87 sine/cosine: the known Unicorn transcendental limitation in `docs/NATIVE-PORT-RULES.md` prevents claiming exact hardware 80-bit parity from emulator output. Use suitable original hardware observations or mark the boundary explicitly. Preserve rendering-thread precision as an input until its live lifecycle is proven.

## 3. Validate the actual TimeGate texture-stage construction

Keep the existing six-case replacement-color state proof at `0x73A880` plus `0x739B40` as a regression. It proves a bounded RepColor/base/diffuse contract, not every SDK apply mode or optional map.

The original stage-orchestration lead is `0x73ADF0`; it calls the proven alpha/base/diffuse paths and additional helpers at `0x73A6E0`, `0x73AA00`, `0x73AAA0` and `0x73AD50`. Those extra helper semantics have not been proven here. Header slot names may explain field offsets, but assign behavior only after tracing instructions and state writes. In particular, a general SDK description does not prove this custom TimeGate path honors every apply mode identically to the stock renderer.

Useful cases are base present/absent, apply 0/1/2, nonwhite diffuse and nonunit material alpha, RepColor on/off, each extra map alone, multiple ordered decals, a hole in decal slots, no remaining hardware texture stage, and a second material following the first. Capture RGB and alpha operations separately and identify optional-pass/fallback behavior. Do not import legacy hardware stage-count limits into the Rust engine merely because the original renderer queried them; record how original visible behavior depends on those limits and remove artificial capacity limits in the new representation.

## 4. Recover material/vertex-color/shade selection before tuning lighting

Start from corpus-exposed effective mode combinations. Use an authored multicolor triangle and nonwhite material ambient/diffuse/emissive values that produce different outputs for ignore, emissive and ambient+diffuse source routing. Pair each with emissive-only and E/A/D lighting. Compare the no-color-array case and material defaults from AA's actual constructor/default-state initialization. Add a pair of adjacent faces for flat versus smooth mode. This separates data import loss from current fixed preview illumination and avoids attributing an arbitrary brightness difference to the wrong stage.

Preserve the existing pipeline's distinction between material alpha, authored alpha blend/test flags, texture alpha used for team replacement, and final viewport alpha. `loading.rs:362` currently prevents diffuse being multiplied twice by supplying a white uniform; do not infer a double-tint bug from the shader expression in isolation.

## 5. Prove draw mode and stencil independently

The current schema collapses four SDK draw modes to a Boolean and omits stencil operations. Begin with corpus-exposed modes, then confirm original AA dispatch and device state for all four values. Use front/back colored triangles before and after the converter's basis/winding changes; test stencil disabled as well as enabled because draw mode is independent of the stencil enable flag. Treat default CCW-or-both as unresolved until the actual AA renderer is observed. Validate stencil's own comparison numbering, mask/reference, failure action, depth-failure action and success action separately; its comparison enum is not numerically interchangeable with alpha/Z.

## 6. Retain the already-proven cases and state their scope

| Existing evidence | Keep as regressions | Remaining boundary |
| --- | --- | --- |
| Phase5 original alpha/depth vectors and GPU layer test | 532 alpha emissions; 32 ordinary-Z emissions; zero-threshold test enable; ONE/SRC_ALPHA; strict cutout; write/read-only depth | W-buffer, exact driver filtering/comparison, general framebuffer-alpha behavior and full lighting |
| Phase2 RepColor state fixture | Six active/inactive/missing-input cases; base/diffuse ordering; shader mask direction | Optional texture stages, packed-color rounding and native pixel output |
| Phase6 geometry transparency fixture | 1,056 key/sort/submission cases at explicit PC24/53/64 plus 22 queue cases; no-sort hint and ties | Live scenegraph traversal, rendering FPU lifecycle, animated native bounds, dynamic overrides |
| Source-bound model sidecar | Original/converted hashes, mesh identity, inherited alpha/Z, team/shadow tags and static authored centers | General material-state semantics and dynamic property updates |

The generic SDK describes write-without-test Z behavior; the current port's ordinary-Z path must not be changed solely to match that text. If it becomes relevant, test original state emissions **and** original depth/pixel effects. Likewise, destination-alpha enum values exist in the SDK but remain explicitly unsupported by the viewport's opaque-alpha representation; enable them only with an appropriate internal-alpha target and new proof.

## 7. Validate property-state refresh separately from static flattening

Use parent texturing/material properties, an overriding child, a grandchild, and siblings. The child's replacement texturing property should be tested with base absent and an optional-map hole: ancestor maps must not be merged into those holes. Test attach/detach, subtree reparenting and content mutation as different operations. Capture the original effective leaf property identities before/after `UpdateProperties` and before/after actual rendering. This tests a cached native property-state lifecycle rather than merely replaying the converter's static dictionary algorithm. Keep RepColor/Sg's TimeGate preprocessing context as a separate mechanism.
