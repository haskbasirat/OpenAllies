Resource release, ResourceCache admission/removal and unload
=============================================================

Ported native functions (original = v14 bytes over every pinned range)
----------------------------------------------------------------------
5AA550 Resource::Release (vf14 of Model/Sequence/Sound; Texture vf14 5A6300
  delegates to a writable child when the last location is writable and +40
  is set, else 5AA550). delta 1: InterlockedDecrement(+4); other deltas: a
  subtraction under manager+20. Only an exact 0 continues (a wrap to -1 or
  any nonzero value returns).
5AEE70 ReleaseResource. bit0 set -> 5AC3D0. Else 5AED90 (under +20, log
  0x636 "removing from task queue", 5AE0E0 list removal): removed -> 404F60
  0x656 and return. Not removed -> 0x661; if still unloaded 0x667, 5B1970,
  return unless bit0 became set; then 0x672 and 5AC3D0.
5B1970 task abort: 5ABB30 (+4,+20,+3C); status1 -> 4, else return; 5ABB50;
  same OS thread as Task18+48 -> status5. Another thread: 6FB190 sleep loop on
  +4 until status5 / B396C0==3 (port: explicit error, needs a concurrent
  dispatcher; the port dispatches on the releasing thread).
5AC3D0 CacheOrUnloadResource (under +20): if flags bit3 clear and UVar54
  ResourceCaching ([B39940]+D8, read only then) is true and 59BF30 admits,
  done. Else 0x69B "Unloading", +3C, clear bit0, vf40, vf54, 0x523
  "Unloaded".
59BF30 -> 59BC70 EnsureEnoughAvailableMemory -> 59B9F0 tail append, 59B8C0
  accounting (class +C/+1C used, +10/+20 peaks by unsigned compare, cache
  +30/+3C totals), bit5.
59BFA0 -> 59BA90: unlink, subtract the CURRENT +C/+10 sizes, clear bit5.
  Reached from 5B0270 re-request (cached branch under +20, then vf2C =
  4B2830 RET for all four classes) and 5ABFF0.
5ABFF0 UnloadResource (5ACA50): +20, +3C; bit0 -> clear, vf40, vf54,
  0x523; bit5 (reread) -> 59BFA0.
Class virtuals (actual vtables): vf40 Texture/Sound 5AA700 (clear bit0 then
  bit1); Model 5A6560 (bit17 -> 73D4B0(+40); 5AA700; +3C Ni entry ->
  59DAB0 + 85C2B0, field 0); Sequence 5A6530 (5AA700 + Ni entry).
  vf54 Model/Sequence 5A7CF0 ("[thread] Unloading Ni resource '%s' (ref
  %d)" 0x32C on B38B98, then +40 to the deferred owner B401AC via 59D430,
  field 0); Texture 5A7C80 (0x268 log; skip if the last location is
  writable and +40 child exists; else +3C to 5A75D0, field 0); Sound
  59AFE0 (streaming41 -> nothing).
B401AC is the deferred-release owner 70EF50 (renderer manager, 419FB5)
constructs at startup; with it absent 5A5030 / the renderer+18 refcount path
leave exactly the same Resource fields (checked).

Removed cap (no-limits policy)
------------------------------
59B6B0 SetSize: budgets = ResourceCachePercentOfTotalRAM (25) of
GlobalMemoryStatus RAM and PercentOfTotalVRAM (25) of device VRAM
(B36510+254, 64 MiB without a device). 59C040: class cap = total/weight-sum
x class weight; on change 59BB70 ReduceCacheToSize. 59BC70: while the class
is over its cap, decache the LRU head ("Decaching" 0x253 / 0x218: 5ABFB0
unload + 59BA90) or, with an empty list, ask 59DA00 (false -> not admitted,
so 5AC3D0 unloads at once). The port admits always (resource_cache.rs).
These budgets depend on the host machine, so nothing deterministic in the
game reads them; eviction only unloads released (count 0) cached Resources
whose next request reloads the same bytes (load timing and log lines). The
oracle keeps the eviction rows as evidence (Rust test checks the divergence
is exactly "admitted instead"). Per-class accounting, LRU order, bit5 and
5ACA50's biome-change unload are kept exactly.

Port (crates/aa-runtime/src)
----------------------------
resource_release.rs  (resource_depot::release) Resources::release_14,
  5AEE70/5AC3D0/5ABFF0, class vf40/vf54, ReleaseLog (+native()),
  ManagerRelease (ReleaseServices adapter for State::release_object).
resource_cache.rs    (resource_depot::cache) ResourceCacheManager state.
resource_task.rs     Task::abort_5b1970; Services::enter_manager_5abb30 /
  leave_manager_5abb50.
resource_depot.rs    Manager.cache; Backend::resource_caching_54,
  enter/leave_unload_3c, release_log, detach_model_root_73d4b0;
  ResourceCallbacks 5B0270 cache owners (59BFA0, vf2C RET);
  Application::set_vworld_resource_context_with (5ACA50 -> 5ABFF0).
resource_ni_cache.rs remove_59dab0.
resource_model_unload.rs eligible_handles_5aca50.

Proof
-----
native_release.py (immutable by default, --write regenerates):
66 cases x 3 PCs x 2 images = 396 machines, 198 rows, 996 executed address
pins. Rust (resource_release_tests.rs): every one of the 159 replayable rows
reproduces the exact backend event sequence and final Resource/class/cache/
Ni-list/task state through the SAME Application manager; the 3 cross-thread
status1 rows refuse explicitly with the native event prefix; plus log
constants, pure cache accounting, B401AC-absent/capped evidence and a
request -> release -> cached re-request round trip.

Client wiring (aa-client)
-------------------------
battle_resource_tasks.rs Current: resource_caching_54 (UI bridge
  ui.uvars.resource_caching_current), enter/leave_unload_3c (Decode3c),
  enter/leave_manager_5abb30/50 (Load04, Registry20, Decode3c; released in
  reverse), release_log (log::info with the native file/function/line and
  formatted native text; the Ni "[%s]" thread is the host thread name).
battle_view_startup.rs update_view_requests_43ec60 now takes the UI bridge,
  execution and PC and releases through ManagerRelease(&mut Current);
  battle_presentation.rs passes them (Double53, as its camera call does).
set_vworld_resource_context_with is NOT used yet: in the port nothing can
  be released (so nothing cached) before the first InitVWorld, because the
  only release caller is the post-VWorld view request queue. It matters from
  a second VWorld init in the same process onward.

Berlin GUI census (release01 exe 822fea4a..., release02 exe c80ffbe2...
with a capped info log at the first 5AC3D0 arrivals), both 3600 s, clean exit
------------------------------------------------------------------------------
InitVWorld and World view creation complete (existing=0, attempted
missing=7970). The ResourceManager task queue then stops at the 9th
dispatched task, the first view model (TemperateTree04.nif era): "5A94F0
actual BoundingSphere selection fallback/diagnostic child is required"; the
failed dispatch prefix blocks later loads. The per-frame 43FF60->43EC60 call
(battle_presentation.rs) runs, but the request-queue head view's Data never
becomes ready (its Resources are not loaded), and 43EC60 stops at the first
not-ready head. So no Data vf18 release occurred: no "5AC3D0 release" line,
no ReleaseResource/Unload diagnostic and no "43EC60 refused" error in
either run. The first real release is blocked upstream by 5A94F0.

Open gaps
---------
- 73D4B0 (Model bit17 detach), Sound 59AFE0 non-streaming release and the
  cross-thread 5B1970 wait are explicit required services.
- 5ACA50's multi-unload order follows the port's namespace order (only log
  order can differ from native bucket order).
- Physical Texture/Sequence publication does not run 59CB00 class creation
  in the port (existing gap); cache class rows are materialised zeroed.
