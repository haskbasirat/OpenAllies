# Native scene initialization segment

`aa-runtime::view_scene_init` owns the semantic scene produced by
**43F948..43FCA4** of `KKC_VWorld::Init43F0A0`. It supplies no AD, loading,
TaskStatus, Sim-unfreeze, application activation or playable-battle receipt.
The real current camera and the actual GPU renderer remain required.

The native types were checked against their constructor vtables, RTTI getters,
CRT registration instructions and embedded names in both executable images:

| VWorld field | Actual type and relationship |
| --- | --- |
| F0 | NiNode; the retained scene root |
| 100 | NiWireframeProperty, disabled; **not attached** to F0 here |
| 104 | NiTexturingProperty, the only property attached to F0 here; apply word2, seven slots, base descriptor filter word2 |
| 108 | Wrapper/context containing a NiDirectionalLight, attached as a root child and root effect; nesting1/flags3 |
| 118 | NiTextureEffect; fresh source texture absent, clamp3/filter2/kind0/enabled1; existing nonnull effect skips its reconfiguration |

In particular, 100 is not a depth property, 104 is not an anonymous property
state and 118 is not a NiTexturingProperty. The directional light is separate
from the current camera. Its world-rotation first column supplies direction;
717460 normalizes XY into the derived planar direction after the root updates.

## Implemented ownership and behavior

The move-only State retains the root, properties, light/context and texture
effect. Internal Rc allocations preserve shared node/property/child identity;
parent and effect references are nonowning where appropriate. Node is also the
reusable semantic867690 owner for the real camera's separately constructed164
root. Transform composition uses exact aa-numeric arithmetic with explicit PC,
the native per-output summation order and binary32 intermediate stores. The
finite arithmetic domain excludes nonfinite/trap/exponent-overflow behavior.

The product caller must lend actual view_grid_startup Seeds through its checked
Globals binder. The public Node factory also accepts the same Globals interface
as VGrid for laboratory replay; supplied laboratory words are not observed
startup proof. Camera.reset_current_7010d0 must execute the real current Reset
and current UVar34 behavior; current_root_164 rereads the current camera after
Reset and lends its actual node. Neither is replaced by a diagnostic camera.

The State's computed planar direction is a derived current-scene output after
the actual writer. Before that writer it remains unbound; this module does not
invent the prior process-global B285CC/D0 values or claim their full lifecycle.
The same active scene/backend owns the derived value used by this modern view.

Modern Rust allocations and growable collections deliberately replace SDK
allocation ABI, reference-count/debug globals and fixed pool/locking internals.
This is a semantic scene implementation, not an SDK ABI or complete Ni graph
implementation. Unrepresented debug/bounding/property-state internals and later
source-texture/effect mutations are not certified. Existing-parent detach and
43E400 teardown remain required for repeated view initialization.

The modern backend must actually install these properties and lighting in its
same renderer. RendererBinding is a retained concrete backend receipt validated
against a nonowning SceneWitness, the same World identity and the backend's own
allocation/epoch. No renderer binding is supplied by the native oracle or the
unit tests. GPU upload/frame display is not VWorld completion. SDK owns that
actual client adapter; camera construction/pose is a separate real dependency.

## Validation

`native_scene.py` executes both original and v14 AA instructions at all three
x87 PCs. There are 51 bounded scene rows, 72 transform/normalization rows and18
constructor-field rows: **282 native executions**, with2377 independently
decoded equal instruction pins. Ni constructors, smart-pointer stores, actual
property/effect list insertion, light attachment, transform/update children and
717460 run. 7010D0 is the explicit current-camera child seam. OS thread and
critical-section imports are allocation transports; the actual8D4B60 pool body
and its three-word node initialization run over a supplied valid freelist.

The constructor projection runs442C00 to442D99 with declared base-child seams,
three allocation fills and both normal/null unrelated-array allocation. It
proves the five initial null fields only, not complete VWorld construction.
The scene rows include an existing-effect branch and pre-call/block cutoffs.
Observed service/cutoff entry PCs are excluded from executed-instruction pins.

The exact production source passes three isolated Rust tests. They compare39
scene success/camera-error rows and all72 numeric rows, and check nonfabricable
scene/World correlation and expiration. The other12 native block-cutoff rows
remain native-only evidence; they are not advertised as Rust error replay.
No whole-runtime unit link was needed. The isolated Globals trait has exactly
the production signature; world_revision_kernel.rs is the byte-identical
production prefix with its unrelated cfg(test) module removed. audit_scene.py
checks that correspondence, all pins and all five class identities. A real
aa-runtime production cargo check also passed; warnings are retained.

Commands (from rust-rebuild):

```powershell
$env:PYTHONPATH='E:/Games/Axis and Alies/reconstruction/tools/python-packages'
python research/development10/view-scene-init/native_scene.py
python research/development10/view-scene-init/audit_scene.py
research/development10/view-scene-init/isolated-tests.exe --nocapture
```

The initial isolated compile included unrelated WorldRevision Sim tests and
failed; the second attempted linking bitcode libraries without LTO and failed.
The third run exposed a test setup error: its preexisting effect used zero
seeds while the native case used the row's nonzero seeds. Only that test input
was corrected. All attempts/logs and the earlier v1 fixture remain retained.
The final accepted source/test/fixture/binary hashes are in findings.json.

## References and license boundary

Native AA instructions provide runtime class, offset, write-order and numeric
authority. Public Niftools documentation provides terminology only:
[ApplyMode](https://www.niftools.org/nifxml/ApplyMode.html),
[TexFilterMode](https://www.niftools.org/nifxml/TexFilterMode.html),
[TexClampMode](https://www.niftools.org/nifxml/TexClampMode.html), and
[TextureType](https://www.niftools.org/nifxml/TextureType.html).
Word2 is modulate/trilinear in those respective enum domains; that alone is not
proof of a runtime field offset. The local Gamebryo documentation was consulted
for conceptual texturing terminology only. No vendor engine implementation,
headers or SDK binaries were copied, integrated or executed.
