ActorData+534 (Actor request 45DD80 tail), 2026-10-18

Problem
  View creation stopped on rus_corps with "45DD80 positive ActorData534
  requires actual derived SettlementData source writer". The guess in that
  message was wrong: +534 is not a Settlement payload. ActorData+174 bit19 is
  component slot 19, MobileDeploymentComponent (enum registration 497FA0;
  464BD0 sets it at 468554; Verify 461340 "Mobile Deployment Actor %s is not paired to another actor"), and +534
  is the forward "unpack to" ActorData written by the deferred 4605B0 pass.

Native evidence (AA.exe image base 0x400000; every executed instruction is
byte-identical in the accepted v14 image)
  4624B0 ActorData constructor 462D3B..462D7B (ebx = the 4624E1 zero, no write
    to ebx before it): +534/+538/+53C/+540 = 0, +544 = 0xBF800000 (-1.0f),
    +548/+54C/+550/+554 = 0.
  464BD0 ActorData LoadData block 46852F..4686D9:
    5B4410("MobileDeploymentComponent", 0) present -> +174 |= 0x80000;
    5B54D0("unpack_to_actor_ids"); CString length (data-0xC) == 0 ->
      5B5130 pack_health_min/packing_length/unpacking_length -> +54C/+550/+554;
    else operator new(0x10), 473AD0 prepends a deferred reference at
      GameData(B36530)+824 (vtable A0347C -> 461840, +4 IDS, +C this),
      5B4D80 packed_position_offset -> +538/+53C, packed_orientation ->
      +540/+544, and 46864A..4686AB raise 410C40 "Invalid packed orientation
      (%i, %i) loading actor '%s'" when |x| and |y| are both ordered and
      <= A032E0 (binary32 1e-4; exact compare at any PC).
  475230 walks the list head first (reverse creation); 461840 -> 4605B0:
    this+534 = 4BDBA0(IDS, 0x2B = any category ActorData) with the Body,
    MobileDeployment, Element-parity and soft-shape (+54) 410C40 gates, and
    target+548 = this. (Ported/proven earlier: aa_rules::native_mobile_deployment,
    research/skirmish-20261016-mobile-actions.)
  45DD80 = ActorData vtable A05540 slot 0x14. Tail 45DF0B..45DF29: if +174 &
    0x80000 and +534 != 0, (+534)->vf14(delta, 0): the target's own 45DD80
    with mode 0 (recursion through original code in this oracle).
  So every packable building (the pack side: corps/HQ/depots/motor pools/
  brigades/airfields) has +534 = null and requests nothing there; each truck
  requests its building with mode 0.

Rust (crates/aa-runtime/src)
  actor_deployment_534.rs (new, module data_pre_request::actor_request::deployment):
    read_block / validate_block = the 464BD0 block in native read order;
    forward_534 = bit19 gate + the completed deferred projection, cross-checked
    against the block (offset, +54C, queued reference) and refusing +534 cycles
    (native 45DD80 recursion would not terminate).
  data_actor_request.rs: new prepare step 7 (after Portal) runs the block on the
    actor's own component, consults Catalog::native_mobile_deployment_data(pc)
    only for bit19 actors, resolves the target by registered identity and stores
    it; actor_deployment_534 replaces the old unconditional error.
  data_request_fields.rs / data_object_request.rs: trait method renamed
    deployment_data_534 (was settlement_data_534); dispatch unchanged.

Proof
  native_actor_deployment_534.py executes ORIGINAL 4624B0 slice, 464BD0 block,
  473AD0/4012C0, 475230/461840/4605B0 and recursive 45DD80 under Unicorn
  (research/phase8-conversion machine). Seams: typed rule readers 5B4410/
  5B54D0/5B5130/5B4D80 answer the case values, 4BDBA0 identity/Find lookup,
  6F7810 bump new, 410C40 stops the case, 4288B0 FX-list requests recorded;
  aspects/banner/abilities are empty and their entries fail if reached.
  70 cases / 434 instructions: 69 synthetic (PC24/53/64: HQ pair both load
  orders, non-mobile, two trucks + alias, 3-link chain, elements, 10 orientation
  boundaries incl. +-eps, next-after, -0, NaN, inf, unknown target, no body,
  not mobile, element mismatch, outside/edge soft shape, authored building
  fields) and 1 shipped case: all 140 actual MobileDeployment ActorData from
  shipped-deployment-input.json (exported by the Rust real-data test).
  Shipped result: no data error; 75 trucks have +534, 65 buildings +548;
  every native +174 after the block equals the Rust completed mask.

Tests (CARGO_TARGET_DIR=target/agent-settlement)
  native_actor_deployment_534_fixture_replay (always runs): every row through
    deployment::read_block/validate_block (reads, +174, +538..+554, errors),
    aa-rules resolve_reference (lookup order, +534/+548, errors),
    deployment::forward_534 and the real 45DD80 dispatcher recursion (FX trace,
    mode 0 at the target, Data+20 counters).
  actual_mobile_deployment_534_for_every_registered_actor (ignored, AA_GAME_ROOT):
    1028 ActorData, 140 MobileDeployment, 75 forward, 65 pack side; HQs null;
    trucks -> buildings; compares with the shipped oracle row.
    AA_EXPORT_DEPLOYMENT_INPUT=<new path> exports the oracle input (create_new).
  actual_hq_buildings_and_trucks_prepare_their_534 (ignored): production
    prepare_object_requests for rus/ger corps HQs and trucks, others.

Results (2026-10-19, CARGO_TARGET_DIR=target/agent-settlement)
  fixture replay: 70 cases, 140 shipped actors, 312 recursive 45DD80 requests, ok.
  data_pre_request lib tests 28 passed; both AA_GAME_ROOT tests passed;
  aa-rules native_mobile_deployment (incl. ignored real data) 3 passed.
  GUI census berlin deploy534-01: pre-World source preparation 1469 prepared,
  0 failed; World view creation completed (attempted missing=7970), no
  "view census" failures; RUSCorps/Pack/Unpack resources queued. Next stop is
  outside this area: ResourceManager load of TemperatePineTree01.nif fails
  "selected Model requires exact source-bound property/effect field projection"
  (aa-client model_graph_fields; index has 3 models), later an egui-wgpu
  staging-buffer panic at ~1368 s.

Reproduce
  PYTHONPATH=reconstruction/tools/python-packages python native_actor_deployment_534.py
  (default compares the fixture; --write regenerates).

Open gaps
  - Position: native 464BD0 loads Portal (46832A), City, HQ, then
    MobileDeployment (468538). Rust step 7 runs right after the Portal step,
    so among the ported 464BD0 parts the block is already at its native
    position. A 410C40 there fails that definition's source preparation and
    battle_source_preparation::prepare returns that error, aborting the whole
    startup as the native throw does (only the AA_STARTUP_CENSUS diagnostic
    mode continues).
  - The deferred 475230 pass runs natively after ALL LoadData; here the whole-
    source projection is evaluated at the first bit19 actor's step 7. Outcome
    is identical (shipped data has no error; any error aborts the load), but
    with malformed data a deferred error could be reported before a later
    definition's own LoadData error. Exact ordering needs the client to run a
    post-loop deferred pass (client file, not changed here).
  - +548 and the vf18/vf1C +534 paths: see
    research/skirmish-20261019-data-release-ready.
