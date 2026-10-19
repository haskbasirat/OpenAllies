5ACA50 Model Resource unload at VWorld init (Firefight view startup)
====================================================================

Problem
-------
Firefight startup stopped at the 43F0A0 VWorld init prefix with
"5ACA50 actual Model Resource requires same-owner unload; new+80 retained".
The old resource_depot::set_vworld_resource_context refused whenever ANY
Model entry existed in the manager. "new+80 retained" = 5AE0A0 copies the new
Biome texture subfolder to ResourceManager+80 (405B80) BEFORE 5ACA50, so that
assignment survives the error. "same-owner unload" = the manager's own
5ABFF0 KKC_ResourceManager::UnloadResource, which was never ported.
Firefight's Biome subfolder is "Tropical" (startup value ""), so 5ACA50 runs.

Native path (original and v14 bytes identical over every pinned range)
----------------------------------------------------------------------
43F0F1 -> 5AE0A0(Biome+98): 96CB83 UTF16 C-string compare; equal prefixes skip
everything; otherwise 405B80 assigns +80 then 5ACA50.
5ACA50: 4245A0 iterator over Manager+60 (bucket order, chain order). For each
node: vf4 class getter, walk descriptor+8 parents for B3D990
KKC_NiAVObjResource. Only a match reads flags+8; `shr 5; test cl,1` skips if
bit5 clear, THEN signed count+4 is read and `jg` skips if > 0. Otherwise
5ABFF0(manager, resource).

Class chains (executed; parents from the real 409750 slice with immediates
decoded from the CRT registrations):
  Texture  vt A454A8 vf4 5A6180 -> B3D97C -> B38E24          skip
  Model    vt A45C98 vf4 5A7EB0 -> B3D990 -> B3D968 -> B38E24 NiAVObj
  Sequence vt A45290 vf4 5A5610 -> B3D9A4 -> B3D968 -> B38E24 skip
  Sound    vt A42C38 vf4 59AF90 -> B3D76C -> B38E24          skip
  Map      vt A44008 vf4 5A0760 -> B3D7DC -> B38E24          skip
  Film     vt A44130 vf4 5A09E0 -> B3D7F0 -> B3D7DC -> B38E24 skip
The read guard proves non-NiAVObj +4/+8 are never read.

5ABFF0 body (executed, children recorded): lock manager+20 (857E10), lock
+3C; if flags bit0 (loaded): clear bit0 (5AC014, before the call), vf40 (Model
5A6560), vf54 (Model 5A7CF0), 404F90(B38B70, ResourceManager.cpp, 0x523,
"KKC_ResourceManager::UnloadResource", "Unloaded %s", +38 name); if bit5:
59BFA0(manager+9C, resource) ResourceCache removal; unlock +3C, unlock +20.

When the original unloads/evicts a Model
----------------------------------------
- Request count reaching zero: Resource::Release 5AA550 -> ReleaseResource
  5AEE70 (aborts a still-queued load) -> CacheOrUnloadResource 5AC3D0. If
  flags bit3 is clear and UVar54 ResourceCaching (UVars.tgi: true) is set,
  59BF30 asks 59BC70 EnsureEnoughAvailableMemory and on success 59B9F0
  appends the resource to the class LRU list and sets bit5 (its ONLY writer).
  If not admitted, it is unloaded at once ("Unloading %s"/"Unloaded %s").
- Budget pressure: only cached (bit5, count 0) entries are ever evicted:
  59BC70 at admission and 59BB70 ReduceCacheToSize (from 59C040 class
  accounting) "Decaching %s" -> 5ABFB0 unload + 59BA90 list removal (clears
  bit5). Per-class caps = total/weight-sum*class weight (59C040), totals from
  KKC_ResourceCacheManager::SetSize 59B6B0: ResourceCachePercentOfTotalRAM=25
  of GlobalMemoryStatus physical RAM and PercentOfTotalVRAM=25 of the device
  VRAM (B36510+254, 64 MiB without a device). Machine dependent. The
  NiAVObj class weights are RAM 30 (ResourceCacheNiAVObjRAMWeight) and the
  59CB00 constant VRAM 10 (resource_model_class.rs).
- Biome subfolder change (THIS site): 5ACA50 unloads cached unrequested
  NiAVObj resources so their next request re-resolves biome textures.
  Requested Models are untouched and keep their already-loaded textures.
- Re-request: 5B0270 removes a bit5 entry from the cache (59BFA0, vf2C).
- Location change / removal: 5AA660, 5AC510, 5AD860, 72F2D0 call 5ABFF0.

Port state and fix
------------------
The port has no Resource release or cache admission (no 5AA550/5AEE70/
5AC3D0/59B9F0 path), and none of its flag writers stores 0x20, so no Entry can
be eligible. The exact 5ACA50 predicate is now implemented
(resource_model_unload.rs); every Model the port can hold takes the native
skip, so startup continues exactly as the original would with those states.
An eligible entry (only possible once the cache is ported) is an explicit
error naming 5ABFF0's unported vf40/vf54/59BFA0 owners; +80 stays assigned.
Traversal order is unobservable while every entry skips.

No-limits policy: growable budgets never evict under pressure. Eviction only
ever affects released (count 0) cached resources and only changes load timing
and log lines; the original budget is itself machine dependent. 5ACA50 is not
budget driven and must stay as is once the cache exists (it is visible:
cached models would otherwise keep pre-biome textures).

Proof
-----
native_model_unload.py (default: immutable replay; --write regenerates):
92 cases x Single24/Double53/Extended64 x original+v14 = 552 machines, 276
rows, 206 executed address-byte pins. Cases: every flag/count state the port
can construct (all skip), Model predicate edges (bit5 x counts 0,1,-1,
INT_MAX, INT_MIN), cached states on every other class (skip, unread), a
7-bucket multi-eligible row (native order 3,6,0,4), the equal-prefix no-op,
and four 5ABFF0 body rows.
Rust: resource_model_unload_tests.rs (3 tests, all 276 rows with genuine
Entry constructors through vworld State) and
resource_depot::view_residency_tests::namespace_5aca50_skips_uncached_models_and_refuses_cached_unload.

Real check (GUI census, tag unload01, fast build target/agent-unload,
exe sha256 3b91f009d437e908...)
---------------------------------------------------------------------
qa-output/development10/lead-gui/unload01/firefight: InitVWorld passed 5AE0A0
("Tropical") and continued: resetting editor variables, terrain, World view
creation begin/end (existing=0, attempted missing=2361), zero `view census:`
failures. The next stop is the first ResourceManager task load,
ThingStartLocation.nif: "selected NIF versionA000100 raw graph loader requires
implementation" (aa-client model_payload_backend.rs, NIF 10.0.1.0 vs the
accepted 10.1.0.0). Clean exit after 1551 s.

Open gaps
---------
- Resource release/cache (5AA550, 5AEE70, 5AC3D0, 59BF30, 59BC70, 59B9F0,
  59BA90, 59BB70, 59C040) is unported. Whether the original releases any
  NiAVObj before 43F0A0 on this startup path is not proven here; the port
  releases none.
- 5ABFF0 positive unload (Model vf40 5A6560: 73D4B0 when bit17, 5AA700,
  59DAB0/85C2B0 Ni cache entry; vf54 5A7CF0) is not ported.
- Modern view registrations (view_resources) still refuse at 5ACA50.
