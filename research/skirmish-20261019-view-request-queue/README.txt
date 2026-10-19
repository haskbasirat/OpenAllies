VWorld request queue: 43EC60 drain and 45BCA0 uninit prefix, 2026-10-19

Native (AA.exe; original and v14 bytes identical for every executed instruction)
  VWorld vtable 9FDFF8: +C = 43FF60, the per-UI-frame VWorld update. While
    VWorld+AD it runs (after 5EEC70, camera 43DA00 and the 41C0E0(5) profile
    begin) 43EC60, then 41C110(5), 43A1C0, 71FAC0, 4361B0.
  43EC60 (VWorld update body): IsGWorldValid assertion, 50C750, 43A400,
    4318E0, 4312F0, 725340, then the request-queue segment 43ED1A..43ED53:
      cell = [VWorld+208]; while cell && cell.view->vf74():
        45B720(view): (view+10 Data)->vf18(1); view+54 &= ~bit28
        next = cell.next; 50C510(VWorld+208, cell) unlinks/frees; cell = next
    then the +E0/+E4/+E8 FX loop (4289A0/429050/45C7A0), not part of this.
  vf74 = 45B140 (mov ecx,[ecx+10]; jmp [Data vtable+1C]) for VObject (A02798),
    VActor (A02270) and the organization view (A01EF0): Data vf1C readiness.
  45B530 (common Init) is the only producer: Data vf14(1,0), +54 |= bit28,
    43D030 append.
  45BCA0 (VThing uninit, reached from VObject 458CE0 / VActor 452D10 /
    44EFA0 / 4450E0 uninit chains at vtable+CC): if +54 bit28 -> Data vf18(1),
    clear bit28, 43C490 -> 4F1430 on [B36538]+208 removes the FIRST cell
    holding the view. The view destructor 45A4D0/45A3A0 does not reach it.

Proof
  native_view_request_queue.py: 66 cases (PC24/53/64): drain of empty, all
  ready, head/middle/last not ready, duplicate allocations (incl. a duplicate
  that is no longer ready), a flag-clear view still released; uninit with
  flag clear/set against empty/head/middle/tail/absent/duplicate queues.
  Real 45B720, 50C510, 43C490, 4F1430 execute; vf74, Data vf18 and 6F7430
  are recorded seams. 111 instructions pinned.
  Rust replay: aa-client view_directory tests
  native_request_queue_drain_and_uninit_replay (cell identities and tail).

Rust (crates/aa-client/src)
  view_directory.rs: drain_ready_43ec60 / uninit_request_45bca0 kernels,
    ViewDirectory::drain_ready_requests_43ec60 and ::uninit_request_45bca0
    over the SAME request queue, Data allocations and Resource application;
    re-exports ReleaseServices / RequiredReleaseServices.
  view_object_fields.rs: request_pending / clear_request_pending (+54 bit28).
  view_world_owner.rs: VWorldOwner::drain_ready_requests_43ec60.
  battle_view_startup.rs: State::update_view_requests_43ec60(source, session),
    one call per native UI frame (RequiredReleaseServices until 5AA550).
  battle_source_preparation.rs: post-loop 475230 deferred pass
    (State::complete_deferred_links_475230), so a 4605B0 error follows every
    LoadData error; census mode records it as a failure.

Open
  - The per-frame caller is the camera owner's battle_presentation.rs job;
    see the lead report for the exact call.
  - Release stops at the first Resource (RequiredReleaseServices) until the
    ResourceManager 5AA550/5AEE70 port lands.
  - The VThing uninit chain (vtable+CC) and 43C420 virtualC4 destruction are
    unported; uninit_request_45bca0 is ready for that chain's 45BCA0 position.
  - The other 43EC60 / 43FF60 children listed above are not ported here.
