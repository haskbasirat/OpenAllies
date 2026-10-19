Recruit order side: 61F210 description, 4D9A40 command sync, B39F58 Object
references, 4ED360 NULL-group delivery (KKC_TellSelectedCommandOrder 84/86)

Scope (after the coordinator's scope change): the order/request side only.
The KKC_RecruitComponent handler 583A70 -> 5838B0 -> 582E80 and the unit
creation belong to the live developer (live_recruit). This work brings a
recruit request through decode, CanSend, Send, the Admin seal, the decree,
61F210, 4ED360/4D9A40, 4CF430/4CE680 and 4D9B60 up to Routing::handle for
the Recruit component slot, where an explicit required error names the
missing live_recruit call.

Native addresses ported
-----------------------
0x4D9A40  KKC_GActorCommand sync dump (cdecl label, command): 77DC50(label,0)
          marker; command != NULL: 77D930("_command->m_value", [cmd+28]) and
          77DA70("_command->m_bits", [cmd+0C]) on synchronizer A [B3655C].
          Each logger: enabled(+0) && suspend(+4) <= 0, then
          GetCurrentThreadId()==[B39828] and [B36528] NULL or +5C/+5D clear.
          Labels: "_command" (A2960C; 4ED375, 4DFE46, 57C76E) and "_cmd"
          (A346CC; every component vt+24 incl. Recruit 583AA9).
          Rust: aa_runtime::player_order_world::command_sync::
            pub fn sync_command_4d9a40(sync: &mut live_clocked::SyncBinding<'_>,
                label: &str, command: Option<&mind_world::PlainCommand>)
                -> Result<(), String>
            consts COMMAND_LABEL "_command", CMD_LABEL "_cmd".
0x61F210  KKC_TellSelectedCommandOrder::Process (GameOrders.cpp). Its own
          effects: the 4DA570 guess (already the decree kernel's) and one
          404F90 diagnostic on B38B70. Empty selection (line 0x15A): order
          name, player kingdom (765E00: [B36534] ? Player+28 : NULL; +188;
          NULL -> 999), Player+8 name (766380 flag 0), time 59ABA0(B396E0);
          "rejecting", no 4ED360. Selected (line 0x16F): vt+B4, vt+108
          (Actor+E8), [4DA570], order name (bit6: [cmd+8]+8; default: the
          guess; NULL -> L"?" 9F7228), actor K, player K, name, [B39B1C]
          view time (binary32 -> double), actor+14, [actor+4]+8 IDS, time
          40B000(B39690) = 59ABA0(B39690+50 == B396E0); then 4ED360.
          Time: fild signed32, fadd 2^32 (9F240C) if negative, fmul 0.001f
          (9F4580) at the thread's x87 PC, fstp qword.
          Rust: player_order_decree::describe (Description, Sources,
          seconds_f64_bits, view_f64_bits, order_choice, assemble,
          describe_61f210); Diagnostics::describe (new default method,
          carries the kernel's guessed ordinal); player_order_frame::
          selected_with (FrameDiagnostics: describe + 4D9A40).
0x76A960  Object-definition reader (61EBA0 bit1, 4C2C60 entries): reads
          40C7C0([B39F58]+28 + 1) bits; v < count, [mgr+24][v] != NULL and
          vt+38 store the definition, else NULL. vt+38 = 4F0500 (1) for
          category kinds 2 (ctor 4624B0, vtable A05540) and 19 (469980,
          A09B30); 4C9FA0 (0) for the other 22 kinds (static join of
          4BCA00's jump table 4BD028 with each case's constructor vtable).
0x76A880  writer: def ? def+0C : [mgr+28].
0x4B4E60  registration (4BCA00 -> 4BCF66 is the only Thing-manager caller):
          def+0C = [mgr+28], vector +24 append, IDS map +30 (repeat IDS
          throws "Data IDS already used", case-insensitive). So the index is
          the registration slot = ActorCatalog::actors order (native load).
          Rust: player_order_decree::object_reference
          (object_registry_count_b39f58, registered_object_b39f58,
          reader_76a960, resolve_object_76a960, writer_76a880,
          object_reference_76a880; command +14 token = Bank
          "ActorDefinition" like the saved-command binder);
          decree::from_fields_with_objects, player_order_frame::decoded_in;
          allocate_decoded_with_bank now resolves bit1 into
          PlainCommand.object_definition (bit1 kept even when NULL).
0x4ED360  VWorldState selected delivery, NULL group: no type branch. The
          former Rust "bounded selected move5/attack0 command entry" (for
          explicit and guessed commands) was not native and is removed.
aa-netcode: game_data::Current registry_count/try_definition for B39F58
          (local_object_registry.rs), the 86 decree uses decoded_in and
          selected_with with the SAME retained A=[B3655C] cell and its
          OS/application gate (local_live_frame.rs Hooks.sync_a), 61F210's
          sources (local_order_describe.rs: Player+8 and Player+28 of the
          SAME Admin+284 allocation, Frontend::application_milliseconds_
          b396e0_current / view_time_b39b1c_current, sink = frame effects).

Oracles (Unicorn, research/phase8-ai/native_machine.py through
recruit_lab.py; both pinned images, PC24/PC53/PC64, verify_v14 on every
executed instruction; --write refuses to overwrite, a bare run replays
byte-identically: *-replay01.log)
  native_command_sync_4d9a40.py   2112 rows (352 cases x 6). Coverage
      4D9A40 23/23, 77DC50 49/49, 77D930 51/51, 77DA70 51/51, 77D680
      16/16, 77CEE0 89/96, 77D7D0 88/98 (unexecuted: ring reallocation
      77CD40 call, multi-entry retire loop iterations and CString frees).
  native_describe_61f210.py       3912 rows (652 cases x 6). Coverage
      61F210 237/245 (unexecuted: two CString free-at-zero tails), 74B3B0
      2/2, 765E00 7/7, 766380 28/28, 40B000 11/11.
  native_object_reference.py      942 rows. Coverage 76A960 37/37, 76A880
      21/21, 411FA0 57/65 (negative-value assertion unreachable for <=16
      bits), 40C7C0 55/73 (widths above 16 bits), 40D5D0 87/87, 4B4E60
      48/57 (preset out-of-order +0C throw), 4F0500 2/2, 4C9FA0 2/2.
  native_selected_delivery_4ed360.py 11520 rows (96 groups: 48 explicit
      groups x all 39 types, 48 guessed). Coverage 4ED360 113/297: all 113
      instructions reachable with a NULL group (the 184 others are the
      group/formation branch 4ED3C0..4ED521, 4ED58F..4ED628, 4ED68F..
      4ED6A5, whose leaves trap in the oracle), 4D9A90 11/11.

Rust replay tests (aa-runtime --lib, filter player_order)
  command_sync::tests: every 4D9A40 row, Direct and Shared SyncBinding
      (hash, window, entries, gate-call counts); 174 rows whose modular
      window crosses INT_MAX check hash/begin/end only, because
      aa_orders::net::sync::Synchronizer::entry compares indexes signed.
  describe::tests: every logged 61F210 row (line, format, file, function,
      channel, time/view binary64, kingdoms, names, event order).
  object_reference::tests: every register/read/write row; ignored real-v14
      test resolves rus_corps / rus_truck_engineer / regiment_rus_hq_
      infantry through 76A880 -> 76A960.
  delivery_tests: every 4ED360 row against the binding's per-actor
      sequence, and the 39-type invariance of explicit commands.

Headless recruit (tests/live_recruit_order.rs, AA_GAME_ROOT, Berlin):
see live-run-*.json and the report; result recorded in findings.json.

Remaining explicit errors (owner)
  * Routing::handle Recruit: "583A70 KKC_RecruitComponent command
    virtual+24: live_recruit handler ..." (live developer). 583A70
    receives this = the actor's Recruit component ([actor+60+16*4]) and the
    command pointer; it reads +0C (bit5), +34 ParameterSet, +08 type and
    +14 ActorData, and calls 77DC50 + 4D9A40("_cmd") first.
  * Command +34 ParameterSet (flag bit5, 4D9E90 from ActorData+2F0, the
    OrganizationLayout instance): PlainCommand has no field for it; any
    regiment recruit (regiment_rus_hq_*) carries it. Decree decode keeps
    "actual4C2C60 ParameterSet allocation/definition owner" (owner:
    mind_world PlainCommand + B3AB00 OrganizationLayout registry).
  * UI CanSend: player_order_input::order_world::plain_from_ui_wire refuses
    wire.object (UI developer); it needs bit1 -> resolve_object_76a960 and
    a read-only Bank token("ActorDefinition", id).
  * B396E0 application timer / [B39B1C] view time: no owner shared with the
    order frame; Frontend defaults are required errors (client owner).
