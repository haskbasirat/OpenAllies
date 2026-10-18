# Retained VWorld composition and bounded native tail

`view_init_world::State<V,A>` retains one prefix/VGrid, asset owner, scene,
visual heightfield, canonical Terrain slot array, terrain backend receipt,
auxiliary12C, helper10C and trigger-VFX list. The client's `VWorldOwner` wraps
this same State plus its real ViewDirectory and allocation token. The FX author
also added the same State's actual command-list owner; it is not a second view.

The native sequence is prefix43F0A0..43F1D4, assets43F1D4..43F948,
scene43F948..43FCA4, slots/grid/heightfield43FCA4..43FD94, actual terrain backend,
captured camera6FF130, camera-target diagnostic, auxiliary86C490, helper42D0D0,
then TriggerManager54BDE0 and every final VFX request. This module has **no AD
writer** and cannot report native view, UI, task, Sim or battle readiness.

The concrete asset specialization checks `view_assets::State::require_initialized`
before entering the scene. That owner currently refuses completion until its
real environment/texture children exist. The generic scene/tail methods remain
bounded service composition; a supplied successful camera callback cannot mint
view completion. Leader owns the real camera constructor/directory, and the
captured camera from43FB40 must be reused for the late6FF130 call.

Terrain uses the canonical source Terrain registry count, never a map palette.
The initial capacity3 is native constructor state, not a cap. Resizing preserves
unknown inactive words and zeroes the active range after committing its count.
Grid and heightfield use the same live World's extents/surface. TerrainQuadSize
is read lazily from the **same actual Ui UVar owner** through a callback, once
inside metrics and again at431BC0. An initial in-progress draft incorrectly
selected the World's unrelated canonical SVar owner; this was corrected before
the final production check. No TerrainQuadSize SVar or copied UI mirror remains.

The terrain and scene backend receipts must validate their actual GPU operation,
same World, same height/scene allocation and live renderer epoch. Modern wgpu
installation intentionally replaces the legacy Ni GPU ABI. It does not establish
all723E00 visual details. The scene binding follows the initial static upload so
it cannot be certified by a zero epoch or immediately invalidated by that upload.

`view_init_globals` retains the actual42D0D0/F0 weak alias behavior once per
application. Dropping an older helper clears a newer registration exactly as
native does. B39D8C/90/94 lie in the PE virtual zero-fill tail; they are not raw
initialized data. The first failed producer attempted to read them as raw PE
bytes; `native_tail_fields_attempt01.py` is retained and excluded. Auxiliary12C
keeps only86C490's actual written bytes. Unwritten padding remains unknown.

`view_trigger_vfx::List` borrows actual loaded trigger actions and canonical
one-hop source references. Only descriptor-kind2 opcodes82..84 read parameter0.
Each qualifying found VFX is inserted at the head; duplicates stay duplicated.
The same Application Data-request owner performs each final vf14 request.
Kinds5..9,12,13,15,16,22,23 use the proved base-only child. Kinds10,11,14,17
commit the base Data+20 count and then require their actual current resource40
owner. A constructor-null field is not treated as source-load evidence.

Validation currently includes:

* `native-tail-fields.json`:48 paired rows/96 original-v14 executions at all
  three PCs; canonical slots, helper constructor/destructor and auxiliary fields.
* `native-trigger-vfx.json`:45 paired rows/90 executions at all PCs; actual
  clear/manager/action/element/head-insert/request bodies, duplicate order,
  nonqualifying NULL-parameter laziness and lookup-error prefixes. Canonical
  lookup is a declared immutable response seam. Laboratory resource40=null is
  only that fixture's child output, not actual source-null evidence. The AD
  store entry43FF36 is an observed cutoff and never executes.
* `pin-audit.json`:246 equal, individually decoded instruction pins. Service
  and cutoff entry PCs are excluded conservatively.
* Exact production leaf code passes3 isolated field tests and2 collection/request
  tests. The latter replay all45 rows and a required-child partial prefix.
* Real `cargo check -p aa-runtime --lib --release --locked --offline -j4`
  passes, with existing warnings preserved in production-check-04.log.

The aggregate has not yet completed against a genuine activated World/GPU
path. Its constructor/allocation domain assumes valid typed inputs and successful
allocation; native allocator failure, Rust poisoned-lock/panic behavior and whole
43E400 teardown are not claimed. No fixed memory/VRAM/cache ceiling was added.

Replay commands from rust-rebuild:

```powershell
$env:PYTHONPATH='E:/Games/Axis and Alies/reconstruction/tools/python-packages'
python research/development10/view-init-world/native_tail_fields.py
python research/development10/view-init-world/native_trigger_vfx.py
python research/development10/view-init-world/audit_tail.py
research/development10/view-init-world/isolated-tail-fields.exe --nocapture
research/development10/view-init-world/isolated-trigger-vfx.exe --nocapture
```
