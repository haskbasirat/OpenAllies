# Material reference review — 2026-10-16

The evaluation package is useful as a vocabulary and test-design reference for material state that the ordinary-model export currently discards. It does not replace AA machine code or establish the original game's pixel output. The strongest existing material proofs remain the native alpha/depth, replacement-color and transparency fixtures.

This review added research files and isolated reference-reading extracts only. No SDK installer, tool, sample, library or engine executable was run. No SDK implementation was copied into Rust. The selected CHM pages were read using the installed generic archive program `C:\Program Files\NVIDIA Corporation\NVIDIA App\7z.exe` (7-Zip 22.01), not a binary from either SDK archive. The extracted CHM/HTML remain under `E:\Games\Axis and Alies\reconstruction\references\gamebryo-archives-20261016\review-reading-materials`, outside the Rust evidence tree; sibling reviewers also used that shared reading folder. No shader, exporter, cache, original game, build configuration or release was changed. Native/GPU tests were inspected, not rerun.

## Source identity and citation convention

The source is `Gamebryo-1.1.2-Evaluation.zip`, preserved SHA-256 `171e36a2d6ed8008b9d59711ebcf5ca3d906f7175d5843f821d6f15dc0feaa87`. Its `SDK/Win32/Include/NiVersion.h` identifies internal version 10.1.0.0. The archive manifest says this matches AA's internal version string; matching strings do not establish an exact engine build, renderer implementation, platform configuration or TimeGate modifications. The archive's included license notice and signed-agreement boundary remain as recorded by the archive README; no reuse/distribution permission is inferred here. The 1.2.2.6 RAR and 2.6 repository were not used to prove material behavior.

[reference-members.json](reference-members.json) gives raw-byte SHA-256, byte count and physical line count for every reviewed header and CHM page. CHM pages are nested members of `Documentation/HTML/Gamebryo_1_1.chm`, whose freshly computed SHA-256 is `2f02a4ffda66beac525fbf4d78bc2ab1697111b2522c444002c24e13c1f4453d`. Line citations below are 1-based physical header/HTML lines. They are not rendered page positions. These aliases resolve to unique members in that inventory:

| Alias | Member |
| --- | --- |
| H-TEX | `SDK/Win32/Include/NiTexturingProperty.h` |
| H-ALPHA / I-ALPHA | `SDK/Win32/Include/NiAlphaProperty.h` / `.inl` |
| H-Z / I-Z | `SDK/Win32/Include/NiZBufferProperty.h` / `.inl` |
| H-STENCIL / I-STENCIL | `SDK/Win32/Include/NiStencilProperty.h` / `.inl` |
| H-VC / I-VC | `SDK/Win32/Include/NiVertexColorProperty.h` / `.inl` |
| H-MATERIAL | `SDK/Win32/Include/NiMaterialProperty.h` |
| H-SHADE / I-SHADE | `SDK/Win32/Include/NiShadeProperty.h` / `.inl` |
| H-XFORM | `SDK/Win32/Include/NiTextureTransform.h` |
| H-STATE / I-STATE | `SDK/Win32/Include/NiPropertyState.h` / `.inl` |
| D-TEX, D-ALPHA, D-Z, D-STENCIL, D-VC, D-MATERIAL, D-SHADE, D-XFORM | Corresponding `Ni…Property.htm` / `NiTextureTransform.htm` under CHM `Reference/CoreLibs/NiMain/Main_Class_Reference/` |
| D-PROPS | CHM `Using_Gamebryo/Programmer_s_Guide/General_Topics/Programming_for_Scene_Rendering/Working_with_Properties.htm` |
| D-STATIC | CHM `Using_Gamebryo/Programmer_s_Guide/System_Details/Texturing/Texturing_with_Multitextures_Basics/Static_Multitexturing/Introduction_to_Static_Multitexturing.htm` |
| D-DARK, D-DETAIL, D-GLOSS, D-GLOW, D-BUMP, D-DECAL | Sibling `Dark_Map.htm`, `Detail_Map.htm`, `Gloss_Map.htm`, `Glow_Map.htm`, `Bump_Map.htm`, `Decal_Map.htm` in that static-multitexturing directory |

## Texturing: useful facts and current boundaries

The header numbers the five apply modes as replace=0, decal=1, modulate=2, highlight=3 and highlight2=4. The class reference defines replace as using the sampled texture RGB/alpha; decal mixes texture RGB with the incoming vertex RGB using texture alpha while preserving incoming vertex alpha; modulate multiplies texture and incoming vertex RGBA. The two highlight modes are described as PS2-specific. These are texture-stage operations, separate from framebuffer alpha blending. An `APPLY_DECAL` base-stage mode is also separate from having one or more decal-map slots. [H-TEX lines 63–74; D-TEX, “Texture Apply Modes”, HTML lines 641–844.]

Ordinary model export does not preserve apply mode. `reconstruction/tools/convert_models.py:148` exports only the base texture filename and UV-set index. `crates/aa-assets/src/types.rs:189` has no apply-mode field, and `crates/aa-client/src/scene.wgsl:54` / `dynamic.wgsl:82` always use the preview's base modulation path. This establishes missing retained metadata, not that all existing rendered models use an unsupported apply mode. Also, `loading.rs:362` supplies a white material uniform while `aa-assets/src/model.rs:107` embeds diffuse/opacity in vertex color: the visible multiplication does **not** by itself prove diffuse is applied twice.

The texturing property has base, dark, detail, gloss, glow, bump and ordered decal slots, plus separate shader maps. Map descriptors carry a texture, UV-set selection, independent clamp/filter settings and an optional texture transform. Bump maps have additional luminance and perturbation parameters. The SDK's shader-map identifier is a shader input identifier, not a normal base/decal slot number. [H-TEX lines 76–130, 141–212, 217–255, 293–315.]

The static-map documentation describes distinct roles: dark maps multiply/darken, detail maps use twice the product, gloss maps attenuate/tint an environment contribution, glow maps add late in the pipeline, bump maps perturb a specialized path, and decal maps overlay in array order with independent UV sets. These descriptions are useful candidate cases; they are not permission to substitute a modern roughness/normal-map shader. Actual pass order, saturation, optional-map omission and feature fallbacks must be recovered from AA. [D-DARK, whole page; D-DETAIL, whole page; D-GLOSS, whole page; D-GLOW, whole page; D-BUMP, whole page; D-DECAL, whole page; hashes in the inventory.]

The current ordinary-model schema retains one texture and one UV array. It discards other maps, shader-map inputs and UV streams. The converter chooses the base-map UV set, caps an out-of-range requested set at the final available set, rounds it, and flips V; `loading.rs:55` restores V for this renderer. A future material-state audit must distinguish that historical conversion from a native malformed-UV policy. [Converter lines 148–152, 204–207; `aa-assets/src/types.rs:249`.]

The four clamp values are clamp/clamp=0, clamp/wrap=1, wrap/clamp=2, wrap/wrap=3. Clamping is edge replication; wrapping repeats the texture. The six filter values are nearest without mipmaps=0, bilinear without mipmaps=1, trilinear=2, nearest texel/nearest mip=3, nearest texel/linear mip=4, bilinear/nearest mip=5. These are per-map settings. [H-TEX lines 43–61; D-TEX, “Texture Clamping Modes”, HTML lines 475–639; “Texture Filter Modes”, HTML lines 846–1091.]

Ordinary static and dynamic renderers instead create fixed repeat/linear/trilinear samplers with anisotropy 4. The import metadata does not preserve authored sampling. PNG loading regenerates mipmaps rather than using each original texture's exact mip chain. Thus even a corpus whose authored base maps all choose wrap/trilinear would not establish texel/filter/color-space parity. [Static `gpu.rs:802`; dynamic `gpu_dynamic.rs:113`; `loading.rs:477` and `:489`.]

Texture transforms include translation, nonuniform scale, rotation, a rotation center and a method selector; the docs distinguish Max and Maya composition order. The default constructor is documented as uninitialized, whereas the separately declared identity instance has defined identity components and a half-UV center. Do not treat every absent/uninitialized transform as a constructed identity object. The ordinary material schema drops the transform entirely. [H-XFORM lines 23–62; D-XFORM, Description/Constructors/Static Constant Instances/TransformMethod, HTML lines 41–70, 142–208.]

## Alpha, depth and sorting: confirmations, not reasons to replace proven code

The SDK matches the established bit vocabulary: alpha blend bit 0; source factor bits 1–4; destination factor bits 5–8; alpha-test enable bit 9; compare bits 10–12; no-sort bit 13. Factor order is one, zero, source color, inverse source color, destination color, inverse destination color, source alpha, inverse source alpha, destination alpha, inverse destination alpha, source-alpha saturate. Compare order is always, less, equal, less-or-equal, greater, unequal, greater-or-equal, never. Its constructor description agrees with the existing native alpha default 0xEC, testing/blending off, threshold zero. [H-ALPHA lines 31–57, 108–121; I-ALPHA lines 16–25; D-ALPHA, Constructors/Alpha Blending Functions/Alpha Testing Functions.]

Depth test/write are separate property bits 0/1, and the compare ordering matches alpha's ordering. The SDK default is test and write enabled with less-or-equal. [H-Z lines 33–52, 77–83; I-Z lines 17–22; D-Z, Constructors and Z-Buffer Comparison Test.]

AA already has stronger, bounded evidence: 532 native alpha-state and 32 ordinary-Z state emissions in `research/phase5-material-render/native-render-state-vectors.json`, plus renderer pixel checks documented in its `FINDINGS.txt:31`. Current `gpu.rs:157` consumes source/destination factors and depth state; `gpu.rs:229` preserves enabled tests at zero threshold; shaders implement all eight alpha comparisons. The separate native queue/sort work has 1,056 key/sort/submission cases and 22 queue cases in `research/phase6-material/`, covering explicit precision, ties and reverse submission. These recorded counts were checked; no fixtures or native tests were regenerated in this review.

The retained non-team corpus already has an explicit zero-destination-alpha-dependent-factor result in the phase5 census, and factors 8–10 are rejected by the current opaque-alpha viewport path. SDK enum availability is not evidence they can be enabled without internal-alpha work. The phase5 pixel checks cover the consequential ONE/SRC_ALPHA case, strict zero-alpha cutout, depth-writing translucent geometry and read-only depth. Preserve these as regressions. Sorting's live traversal, current FPU mode and animated bounds remain separate limitations. [Phase5 `FINDINGS.txt`; `corpus-material-state.json`; phase6 `FINDINGS.txt`; `gpu.rs:178`; `transparency.rs:11` and `:133`.]

The generic SDK Z-buffer documentation describes write-only/no-comparison operation. Do not change `gpu.rs:164` from its current ordinary-Z behavior based only on that prose: property bits, original D3D9 state emissions, and effective hardware writes are different evidence layers. If this combination matters, execute the original renderer path and test target pixels/depth before changing the port. W-buffer behavior remains outside existing proof.

## Material, vertex colors, shade and culling

The SDK material has ambient, diffuse, specular and emissive RGB, shine, and material alpha. The class documentation gives SDK constructor defaults of ambient/diffuse 0.5, specular/emissive 0, shine 4, alpha 1. These are SDK defaults only; missing-property defaults in AA need original constructor evidence. The current converter begins diffuse at white and exports authored diffuse/opacity/emissive but omits ambient/specular/shine. [H-MATERIAL lines 33–49, 79–84; D-MATERIAL, Constructors, HTML lines 60–278; converter lines 139–147.]

Vertex-color state has two independent controls. Source mode can ignore authored colors, route them to emissive, or route them to ambient+diffuse. Lighting can select emissive only or emissive+ambient+diffuse. The default pair is ignore plus full E/A/D. Consequently an authored vertex-color array is neither automatically diffuse tint nor automatically used. `NiShadeProperty` independently selects smooth/Gouraud or flat shading, with smooth as the SDK default. [H-VC lines 33–52; I-VC lines 17–21; D-VC, Supported Source Vertex Modes/Supported Lighting Modes, HTML lines 91–365; D-SHADE, Description/Constructors/GetSmooth, HTML lines 49–91.]

The ordinary mesh schema has no authored color array or source/lighting/shade modes. `aa-assets/src/model.rs:107` makes uniform vertices from material diffuse/opacity. The ordinary shaders use a fixed ambient-plus-Lambert expression and emissive addition, rather than recovered general Gamebryo lighting. This proves a narrower representation and an explicitly preview lighting model; it does not identify which missing term caused any particular screenshot. A separate VFX path does read vertex colors, alpha/material-color/flip/texture-transform controllers (`vfx/extract_nif_effects.py:358`, `:390`, `:415`); this review's import-gap statements apply to ordinary exported model meshes, not every client path.

Culling is on `NiStencilProperty` even when stencil buffering itself is off. The SDK has four draw modes: default CCW-or-both=0, CCW-only=1, CW-only=2, both=3. The default mode deliberately permits platform-dependent CCW-only or both behavior. Stencil's compare ordering begins with never and ends with always, unlike alpha/Z; do not reuse the alpha/Z numeric comparison mapping for stencil. [H-STENCIL lines 30–60; I-STENCIL lines 17–29; D-STENCIL, GetDrawMode and Using the Draw Mode to set Triangle Culling, HTML lines 153–349.]

The converter retains only `draw_mode == 3` as `doubleSided`. `gpu.rs:563` chooses between no culling and back-face culling. It drops CW-only, default-mode semantics and all actual stencil settings. This is demonstrable metadata loss; its impact depends on the retained corpus modes, TimeGate renderer overrides and final winding after conversion. Recover those before deriving a culling fix from the generic API.

## Inheritance and runtime invalidation

A lower property of the same type replaces the ancestor property. A texturing replacement replaces the **whole** property, including map slots explicitly absent on the child; ancestor maps are not filled into its holes. SDK geometry states cache effective property pointers. Attaching/detaching properties or moving the subtree requires property-state refresh; changing a property object's contents is a different operation. [D-PROPS, whole page, particularly HTML lines 35–128; D-STATIC, whole page, particularly lines 35–50; H-STATE lines 43–61; I-STATE lines 17–47; `NiAVObject.h` lines 63, 163.]

The existing converter/export audit copies the ancestor dictionary and replaces entries by property type, which is consistent with whole-property replacement for its static export. The source-bound sidecar preserves inherited alpha/Z and static source sphere centers, with source/converted hashes and mesh identity checks. It is not proof of live UpdateProperties lifecycle, arbitrary shared-node graphs, duplicate-property malformed input, dynamic overrides or original traversal. TimeGate RepColor/Sg context propagation is separately recovered and must not be generalized into ordinary property inheritance. [Converter lines 178–183; `research/texture-phase2/export_model_materials.py:135`, `:173`, `:189`; `model_fixes.rs:768`, `:798`, `:886`; `research/phase2-textures/REPCOLOR.txt`.]

## Recommended continuation

[validation-queue.md](validation-queue.md) turns these distinctions into bounded original-code tests. [original-material-census.json](original-material-census.json) and [corpus-summary.md](corpus-summary.md) record the new read-only inventory of the existing exported-mesh population; they establish authored-data exposure, not rendering parity. No full shader rewrite follows from this reference review.
