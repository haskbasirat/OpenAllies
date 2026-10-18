Phase 8: native trigger system for the campaign and tutorial missions
====================================================================

Scope
-----
Every trigger element (event, condition, action) registered by the ORIGINAL
AA.exe (115 types) is ported and proven against the original machine code
under Unicorn; every original map (80 RMP files: campaign, tutorial, trailer,
developer) loads its TRIGGERS and MAPSTRINGS chunks through ported code that
is proven against the original loaders.

  campaign missions                         26
  element types used by them                79 / 115 registered
  element types ported AND oracle-proven    115 / 115 (round 2: the 26 types
                                            no campaign mission uses)
  element instances in all 80 maps          7346; 7346 dispatched to ported code
  element instances in the 26 missions      3993; 3993 dispatched
  TRIGGERS chunks decoded                   80 / 80 (48 manager v5, 32 v4)
                                            == original 54D3D0 loader, field
                                            by field (native-triggers.json)
  MAPSTRINGS chunks parsed                  80 / 80 == original 47EA40 loader
                                            (native-mapstrings.json)
  localized trigger parameters bound        55 / 55 (== original 49AD70 text)

coverage.json lists, per element type: kind, missions using it, instances,
the native 546AF0 case address, the leaf calls of that case, the selector
registers, the Rust module, the number of native oracle cases and the status
(proven / ported / missing). summary.types_missing is empty.

Round 3 (this round)
--------------------
Locale      Traced: setlocale is 97042E (_setlocale_lk 970255,
            _setlocale_get_all 96FDAB, category table B35580). Its only callers
            are 7F19D9 / 7F1A08 / 7F1AA1 in one component: setlocale(LC_NUMERIC,
            NULL), and setlocale(LC_NUMERIC, saved) only when the saved name is
            not "C" (AA05A8) -- so nothing ever leaves "C". __init_ctype
            (97E948) has no direct caller (only the category table, used by
            _setlocale_set_cat); _pctype [B358A0] is written only there and
            __ptlocinfo [B35444] only by setlocale. The CRT's startup
            _setmbcp(_MB_CP_ANSI) changes _mbctype (the _mbs* functions), not
            isleadbyte (96CBD7 reads _pctype) or mbtowc (9710C6 reads the
            locale's LC_CTYPE handle). Result: the MAPSTRINGS reader, _wcsicmp
            and printf run in the C locale on every Windows code page. The
            oracles now use the image's static initial locale info B353F0
            (loader.py, harness.py) instead of a zeroed stand-in; all fixtures
            were regenerated. MAPSTRINGS text bytes 0x80..0xFF are widened to
            U+0080..U+00FF (new probes high_bytes/high_before_quote/high_c1);
            the GER_Art_Map entries now resolve.
Capture     54BEA0 TriggerManager::QueueCaptured(captured, source) -> 54B820
binding     payload (vtable A39DA4, kind virtual 4F0500 = 1): +4 captured +14,
            +8 source +14, +C (source if virtual +A8 else virtual +108)'s +14,
            prepended to manager+50. Ported as predicates::queue_captured
            (CaptureObjects: +14, virtual +A8, virtual +108); a null source is
            an explicit error (54B820 calls its +A8 through null). Proven by
            native-capture.json (27 cases incl. existing queue entries, two
            pushes, owner null/unregistered, null captured, null source fault)
            in capture_native.rs. This is the binding of aa-combat's
            AttemptWorld::capture_notification (4D5230 -> 54BEA0).
Registers   The 546AF0 register file is a manager member (manager+2C, 8
            registers of 0x24 built by 54CEE0 with 54DE30: empty CStrings),
            reused by every element evaluation for the manager's lifetime and
            never saved. 5422F0/54A5A0 assign the +4 string only for storage-6
            parameters, so a non-text item reads the string an earlier element
            left in that register. Manager::convert_registers (called at the
            546AF0 entry after validity) and Manager::register_text model the
            +4 slot; 54E910 for the ma_hero register of opcodes 25/26/35/48/49
            reads it. This makes the random-hero item 28 reachable and ported:
            48 (544D48: ActorData bank GameData+C entries with +38, +174 bits
            17 and 8, byte +2E4, not yet held; 4F02B0 + 85D840 random, unsigned
            remainder; 4E1130 + 4E4C30), 49 (544F28: Kingdom+1FC entries via
            4E13F0 with eligible data and +8 set or +C/+10 zero; random pick;
            4E2CB0) and 35 (warning). Proven by 66 random-item cases and 6
            prelude cases in which earlier elements fill the register file
            (native_extended.py "prelude": the same native register file is
            reused). The other round-1 selectors still refuse non-text string
            reads (could use the same register model).

Round 2
-------
MAPSTRINGS  crates/aa-triggers/src/mapstrings.rs. Native path: 473050 data
            reader, 5BD2C0 load (5AA990 memory file, 856F90 character reader:
            CRT isleadbyte/mbtowc, or MultiByteToWideChar after a UTF-8 BOM;
            857190/856D20 lines), 5BD080 TGC_DataFileParser (5BCC80/5BEFD0
            tables), language subnode selection (5BB0xx, 5BC8A8), 494000
            DisplayString escapes, KKC_TextData 475180/4BA710 registered in the
            KKC_TextDataManager (ctor 4BAD50(0x1E): 1009 buckets, missing key
            "error_missing_string"), lookups 4B9D40 / 4B9F40 GetDisplayString.
            Proven on all 80 maps (1975 entries, 2455 lookups incl. every
            trigger key) and 90 synthetic probes: 17 native rejections are
            reproduced with the same throw site, 15 inputs outside the proven
            subset are refused explicitly (escapes '&' '\', UTF-8 BOM with
            non-ASCII, lines > 2047 bytes, NUL, ';' between several quote
            pairs, nodes other than [Text], mixed language attributes, unquoted
            name, non-ASCII keys). Bytes >= 0x80 inside quoted values are
            widened byte by byte (round 3: the game stays in the C locale).
            Binding: 49AD70 (key empty -> static empty CStringW B43B48, built
            by 9E7400/405B10 and never assigned; else 4B9F40 with default
            B43B48). MapStrings::bind fills Value::Localized.inline; a key not
            in MAPSTRINGS is an explicit error (native continues into the
            global text tables, warns, then falls back to the
            error_missing_string key). aa-runtime trigger_world::
            load_map_triggers(triggers, mapstrings, LanguageIDS, budget) is
            the map-load entry point.
TRIGGERS    decode.rs: manager v4 (54D79F: one string + one word read and
loader      discarded, no string list) and v5; trigger v4; element v2 and v3
            (5468A0: identical layout, word_10 read for v>=2); parameter v4.
            544190 per-opcode load rules are explicit errors (36 below v3 ->
            localization upgrade via 49AEF0; 71 with 3 params -> 548BC0
            default; 4/94 with ma_cutscene -> ma_string conversion) and the
            generic path's count check ("Unexpected number of parameters.",
            858AD0) is enforced; none occurs in the 80 maps. 54D3D0 skips
            empty variable names (ported); duplicate variable names are an
            explicit error (native keeps both, lookup finds the newest).
            native_triggers.py runs the ORIGINAL TriggerManager (ctor 54CEE0,
            registries from the static ctors 546600/54A440/549D90, definitions
            filled by 4C6E40) on every map after the original MAPSTRINGS load
            and reads back every field: triggers_native.rs matches all 80 maps
            (7346 elements). After loading, 54D3D0 broadcasts
            "TriggerListUpdate" (50D120); the runtime owns that message.
Extended    crates/aa-triggers/src/extended.rs: the 26 element types no
elements    campaign mission uses, 1341 native cases (native_extended.py ->
            native-extended.json, extended_native.rs):
              4 54C2F0/54C240 (wcscmp over manager+40), 11 542580 (morale vs
              value*max*0.01f), 20 541A60, 21 542BD0 (+0.5f, floor 96D5E0,
              binary32), 24 542340, 25 543550 (4364C0 member iterator),
              26 542360 (401060 handle check), 32 5436D0 (4CA760 move to every
              owned Area+84 actor), 33 5430B0 (relative teleport, 4D12C0 skip,
              4F6760 on failure), 35 541C20, 37 543800 (4CE6C0 unit;
              organization_modify_morale seam = aa_combat::morale::modify),
              38 5431D0 (542A30 guard command), 39 5426F0 (554B30 placement,
              4EE5C0 / 4EEB40 member moves, bit 30 of +8 set around moves),
              41 5432A0 (4B29B0 with 40FCA0 permission), 45 541D80,
              48/49 544CF0/544ED0 (named hero; others add/remove null),
              65 5420A0, 68 5420D0, 84 541EA0, 88 541F20, 89 4F96D0,
              90 546A30, 106 544940 ("CameraFollow %d %s"), 111 542950
              (query; tile pass is a seam), 113 545080 ("ShakeCamera %f %s").
            Stores into engine fields (hero entry +18, fidget variable cell,
            teleport flag bit) are recorded in order as "Store" calls.
Opcode 68   game_speed_set: only v > 0 (or unordered) proceeds; result =
(FYL2X)     FYL2X(ln2, v) / FYL2X(ln2, 2.0) divided at the precision control,
            stored as binary32 for 76F370. 70 cases (60 binary32 stores, 21
            inputs at PC24/53/64) match. LIMIT: FYL2X results are extended precision,
            not affected by PC and not correctly rounded by specification;
            Unicorn computes them through binary64. Rust decides the binary32
            from a binary64 log2 and refuses (explicit error) any result within
            2^-40 relative of a binary32 rounding boundary, where neither the
            emulator nor this model is ground truth.
Harness     Emulator caveats (docs/NATIVE-PORT-RULES.md) checked: page 0 now
caveats     faults unless the instruction is FS-prefixed (loader.py
            guard_null_page, also used by harness.py); native-elements,
            native-format and native-mapstrings regenerate byte-identical case
            data under the guard. Faults are recorded only for genuine page-0
            null dereferences (native-extended: 45 cases; the Rust port returns
            an explicit error for each). No fixture loads x87 signalling NaNs
            (scanned); actions.rs fld_fstp quiets them as hardware does. Stubs
            are written once before execution (no stale translations).
            Allocations are served by seams (6F7810, 85C680, HeapAlloc).

What "proven" means here
------------------------
The ORIGINAL code executes for: the 546AF0 dispatch (argument registers are
evaluated in the native right-to-left order), 541900 validity, 5422F0/54A5A0
register conversion, every selector, the TriggerElement leaf of each opcode,
and the trigger-owned engine methods listed below. Calls into other engine
subsystems are replaced by recorded seams (receiver + arguments + order); the
Rust port must produce exactly the same seam sequence, result, manager context
(context word, condition/event Kingdom, subject list/cursor), objective lists,
mission results, weather and actor flag words. The seams are the deeper
engine services, named after their native addresses in elements::Services and
actions::Effects. They are NOT proven here; the runtime must bind each to its
native port (several already exist elsewhere, e.g. 4D9B60 in
aa_navigation::control::process, 4454A0 in aa_trig::rotate).

Ported native functions (beyond the 546AF0 case blocks)
--------------------------------------------------------
Selectors   54D9E0 value, 54DA20 bool, 54DA40 compare, 54DA60 location,
            54DA70/4F1AD0 objective, 54DA80 area (+vt BC), 54DAC0 Kingdom and
            54DC20 actor with their context writes (event -> manager+28,
            condition -> manager+24 / actor GTID -> manager+10), 54DB80 actor
            GTID, 54DCC0 camera (+vt C0, 404ED0 warning), 54DE00 string,
            54DED0 localized pair, 54DF10 difficulty (_ftol2), 54E8E0 command,
            54E910/54E8A0 ActorData (+vt 38 / vt 48).
Events      queue payload kinds 0..5 (54B7B0..54B9F0) and scanners 54C450,
            54C530, 54C5F0, 54C660, 54C710, 54C790, 54BA90, 54BB00, 54BB70,
            54BBE0, 54BC50, 54BCC0, 54CAC0 (+CRT _wcsicmp 96CAD6, C locale),
            subject insertion 54C380 semantics.
Conditions  541DE0 compare, 541B60, 541AD0, 5423D0, 542890 (life x87 product),
            542DE0 (__allshl state bits), 542630/542690, 5423A0, 5425F0,
            543B60/543A10 and 541A20/4ECD40 (selection lists), 541AA0,
            541B90/541BC0, 542CA0/542D00/541B40/541B20/541AF0, 5423F0,
            542490/542460/5424F0/5424C0/542550/542520, 542DB0.
Objectives  4E7050 gain (SortObjectives ordering by text, wcscmp), 4E2120
            lose, 4E2020 find, 4E21F0/4E2250/4E22B0 state, 5458F0, 5459A0,
            545A50, 545B00, 545BB0 (+ "ButtonObjectives" flash).
Victory     542040/542060 -> 4E9C20/4E9C30 -> 4E9720 SetMissionResult
            (TeamVictory gate, team member count, World+48, Kingdom+239,
            messages 0x34/0x35, local-Kingdom UI end, 4F4DA0 notify).
Camera      544540 (pan; realtime clock 59ABA0 path) and 544780 (return to
            default): exact "MoveCamera %f %d %f %f %f %f %f %d %s" text,
            including x87 time sums at the active precision.
Commands    542EB0 (4D9DC0/4D9AB0/4D9BF0 command fields), 5435D0 move
            (4D9E20(5)/4CA760 + speed product), 543410 lock/unlock.
Creation    5452A0 named actor/organization/unit/building (4C8C70 request),
            543BA0 transported creation (4281E0 subtract, 409230 normalize,
            altitude/flags paths), 544A50/544BA0 flares, 544040 ability use.
Other       5438E0 cutscene mode, 5428D0/541F80/541FB0/542010 technology,
            541D10 banner, 541DA0 sun, 542C50 resources, 542120 counter,
            4F1390, 541ED0 sound, 541F00 speech, 542170, 543FD0, 543510,
            541CF0, 542FF0 VFX (incl. overlapping orientation vector),
            543F40/543F80 chat, 542E60 animation, 545C60 PlayVideo text,
            World+290 weather.
CRT         VC7.1 _vswprintf (96C7EC) %f/%d: 17 significant digits rounded
            half-even then half-up to the precision, '-' kept on zero.
            native-format.json executes the original on 15,722 inputs; the
            model matches all |v| < 1e13 (larger values are explicit errors).

Rust (crates/aa-triggers)
-------------------------
  selectors.rs   registers/selectors above, element kind, _ftol2.
  predicates.rs  event payloads + scanners (replaces the unverified draft).
  objectives.rs  Kingdom objective list + 4E9720 mission result.
  elements.rs    conditions/events + objective/victory/speech actions.
  actions.rs     remaining actions (camera, creation, commands, UI, audio ...).
  crt.rs         VC7.1 %f/%d, x87 f32+f32 -> f64 store.
  world_program.rs  phase7 world wrappers now use the shared selectors (Kingdom
                 items 23/24 are now bound); `Program` is the complete 546AF0
                 dispatch: core -> elements/actions -> phase7 world wrappers.
  extended.rs    the 26 types no campaign mission uses (round 2).
  mapstrings.rs  MAPSTRINGS parser, lookup, 49AD70 binding (round 2).
  decode.rs      TRIGGERS records, manager v4/v5, element v2/v3.
  lib.rs         native_module(opcode), element_schema(opcode),
                 argument_index(name).
crates/aa-runtime/src/trigger_world.rs: Kingdom+19C mission result, +239 flag,
+1EC team handle over the canonical Kingdom shell, and State fields for the
World+250 objective definitions, Kingdom+300 objective lists (bound only from
an empty saved vector) and World+290 weather.

FPU
---
Precision control is an explicit input. Triggers run from the world tick at
PC53 (lead note: TGC_NetNode::Update thread, CW 0x027F). Every opcode with x87
arithmetic (17, 10, 31, 30, 71, 72, 73, 83 ...) is proven at PC24, PC53 and
PC64. No fixture relies on Unicorn FPATAN/FSIN/FCOS/FYL2X: camera angle
helpers 4DBD80/4DBD40 (table atan/asin 85D8D0/85DA10) and rotation 4454A0
are seams; opcode 68 (game speed, FYL2X) is ported with the guard band
described under "Opcode 68".

Tests (all from E:\Games\Axis and Alies\rust-rebuild)
------------------------------------------------------
  set CARGO_TARGET_DIR=target/s-triggers
  cargo test -p aa-triggers --release --locked --offline -j 4
    elements_native   4,276 native cases (13 are documented Rust errors for
                      non-finite value registers)
    extended_native   1,381 native cases of the 26 round-2 types (45 native
                      null-dereference faults -> Rust errors; 16 documented
                      Rust errors for NaN value registers; 66 random-hero and
                      6 register-file prelude cases)
    capture_native    27 native 54BEA0 cases (combat capture binding)
    crt_native        15,722 native _vswprintf cases
    mapstrings_native 80 maps + 90 probes vs the native string-table loader;
                      every localized trigger parameter of the 80 maps binds
    triggers_native   80 maps vs the native TriggerManager loader
    missions_native   every map's TRIGGERS chunk decodes (48 v5 + 32 v4),
                      census, kinds, 541900 validity, 3993/3993 campaign
                      instances dispatched, coverage.json consistent
    missions_dry_run  3,728 campaign instances evaluated against an empty
                      world with MAPSTRINGS bound: zero errors (smoke test)
    saved_native      tutorial records == phase7 native-writer fixtures,
                      v4/v5 manager framing and truncation errors
    program_compose, manager/program/world_program_native (phase7 corpora)
  cargo clippy -p aa-triggers --release --locked --offline --all-targets --no-deps -- -D warnings
  cargo check -p aa-runtime --release --locked --offline (trigger_world.rs)

Reproduce research data (PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages)
  python census.py           per-map census (all 80 RMP files)
  python extract_chunks.py   chunks/*.bin (TRIGGERS), chunks/mapstrings/*.txt
                             (MAPSTRINGS) + index.json (SHA256-pinned)
  python native_elements.py  native-elements.json (~7 min)
  python native_extended.py  native-extended.json (~2 min)
  python native_format.py    native-format.json (~10 min)
  python native_mapstrings.py native-mapstrings.json (~6 min; probes in
                             mapstrings_cases.py)
  python native_triggers.py  native-triggers.json (~4 min)
  python native_capture.py   native-capture.json (<1 min)
  loader.py                  shared Unicorn loader (seams, CString manager,
                             imports, page-0 guard) for the loader oracles
  python coverage.py         coverage.json
  python record_findings.py  findings.json
  disasm.py / case.py        read-only disassembly helpers
tests/compose/delegate.rs is generated (trait forwarding for the composition
test); regenerate it if the Services/Effects traits change.

Gaps (explicit errors, never default success)
---------------------------------------------
- Every engine service behind the seams still needs a runtime binding (see
  requests-to-lead.md): UI command interpreter 50D120, audio 7476E0/7474F0/
  747630, camera angle tables, placement 554B30, creation/transport/flare/
  ability services, Kingdom properties/resources/heroes (4E2A60/4E1130/
  4E4C30/4E2CB0), command processing 4D9B60, selection/player lookup, morale
  579750 (-> aa_combat::morale::modify), teleports 4EE5C0/4EEB40, fog tile
  pass (aa-spatial iterator 554DD0/5541F0/554330 + 54F930), game speed 76F370,
  sun dimmer 4ED820, SAI profile 79F5D0. trigger_world.rs provides the Kingdom
  mission/objective/weather state and load_map_triggers.
- Native null dereferences become explicit errors (missing TeamVictory var,
  empty difficulty variable, null transport+C0/flare+57C, null Kingdom+1C4,
  stale Area+84 GTID in opcode 32, null organization arrays, ...).
- Native registers are reused between elements. The +4 string slot is now
  modelled (Manager::register_text) and used by the ma_hero register; other
  selectors reading storage their own parameter does not own remain explicit
  errors. The world RNG (4F02B0/85D840) used by the random-hero item is a seam.
- Non-finite value registers are rejected by the numeric layer.
- Opcode 68: results within 2^-40 of a binary32 rounding boundary (FYL2X).
- TRIGGERS: manager versions 0-3, element versions 0/1 and the 544190 upgrade
  rules (none in the 80 maps) are explicit errors.
- MAPSTRINGS: see the refused inputs above; a trigger key missing from
  MAPSTRINGS (native falls back to the global text tables).
- Function statics (SortObjectives binding B3CBE0, flare variable B3D464/
  B3D46C) are modelled as service calls; the runtime must cache like native.
