# Requests to lead — phase 8 class-changed conversion (2026-10-15)

Evidence: `native-conversion.json`, `native-conversion-real.json` (original
AA.exe under Unicorn, `native_conversion.py`), replayed by
`crates/aa-mapworld/tests/class_conversion.rs` (10 tests pass, 2 of them need
`AA_GAME_ROOT`). Module: `crates/aa-mapworld/src/class_conversion.rs`.

## C1. Restoration agent: wire the conversion into the record loop (group 1, 7,879 records)

All in files the restoration agent owns (`aa-mapworld/src/restore.rs`,
`aa-runtime` restore path). Exact hook:

1. **One queue per map load** (`4FC600` creates one vector for
   USED_REQUIRED_OBJECTS, STATIC_THINGS and DYNAMIC_THINGS):
   `let mut conversions = aa_mapworld::class_conversion::ConversionQueue::<D>::new();`
   (`D` = whatever definition handle `create_thing` needs, e.g. the
   `RuntimeDefinition`/`DefinitionProof` plus its native kind).

2. **At the class compare** (`4FAFA8`), i.e. in `restore_dynamic` after
   `null_record_cause` returned `None` and the resolved
   `definition.native_class != source.class_code` (today `load_record` returns
   the "source class # differs from definition class #" error there):

   ```rust
   use aa_mapworld::class_conversion::{self, MismatchOutcome, ConversionError};
   match class_conversion::on_placement_class_mismatch(
       source,                       // &Placement (decoded record)
       definition_handle,            // D, moved into the queue
       definition.native_class,      // [[def+18]+1C]
       Some(&mut conversions),       // 4FC340 sections: always Some
   ) {
       // 4FAE80 yields null; 4FC340 seeks to record start + index length:
       // continue with the next index record. Do NOT register the saved ID.
       Ok(MismatchOutcome::Converted { index, consumed }) => { /* report as converted, not failed */ continue }
       // Only reachable with queue = None (TEAMS/KINGDOMS, embedded 4FC200):
       // warning "load_warning_changed_gclass|s" (5A1040 at 4FB077), null.
       Ok(MismatchOutcome::Skipped { warning, .. }) => { /* NativeMessage */ continue }
       // "Bad data version": the ORIGINAL map load fails (5A2B90 catch
       // 5A312F) and the queue is dropped (nothing is replaced).
       Err(ConversionError::BadDataVersion { .. }) => { /* map load error */ }
       // ReadPastChunk: native chunk-stream error path not ported (error).
       Err(e) => { /* error */ }
   }
   ```

   - Inputs: the record (decoded `Placement`, or for a record the inventory
     decoder rejected (`Inventory::unsupported`) the BYTE path
     `on_class_mismatch(stream, ...)` where `stream` = the DYNAMIC_THINGS data
     chunk from the record's first byte to the chunk END: natively nothing
     bounds these reads by the index length). The placement path equals the
     byte path on all 7,879 real records (test
     `real_maps_placement_path_matches_byte_path`).
   - `null_record_cause` already implements the skip rule; it can call
     `class_conversion::check_class(saved, definition_class, context)`
     (`ClassCheck::{Matches, Convert, Skip}`) instead of its own expression.
   - `load_embedded_record` (4FC200) correctly passes no context.
   - Outputs: `MismatchOutcome` (queue index or warning, bytes read);
     errors as above. The record yields null either way.

3. **Replacement — the LAST step of the map load** (`4FD47C..4FD781`), after
   all sections, the PostLoad loop (`4EFBE0` -> vf DC), `4F1E10`, `4F5290`,
   `4F1C20` and the "[4610] InitForLoadedMap" log:

   ```rust
   let replaced = class_conversion::replace_converted(conversions, &mut world);
   ```

   `world: impl ReplacementWorld<D>` (= `VariationSource<D>` + `create_thing`):
   - `is_object_data(d)` = definition vf+30 =
     `class_conversion::native_kind_is_object_data(category.native_kind).unwrap()`
     (oracle section `vf30`, all 24 data classes of `4BCA00`);
   - `variations_enabled(d)` = ObjectData+108 =
     `catalog.native_creation_definition(id, pc)?.variations_enabled`;
   - `aspect_count(d)` = ObjectData+110 =
     `native_creation_definition(id, pc)?.aspects.len() as i32`
     (these three are called lazily in native order; the values used for the
     27 maps are in `real-definitions.json`);
   - `create_thing(&ObjectCreationParams<D>)` = YOUR port of `4F9C10`
     (`KKC_GWorld::CreateGThing`) with the A25B1C block: definition, position
     (3 words), direction (x, y), EMPTY name, variant, requested ID = saved ID.
     Return `Ok(Some(created))` for non-null, `Ok(None)` for the native null,
     `Err` only for an unported path (it stops the loop; `ReplacementError`
     lists the completed entries). Native dispatch for this block (oracle
     section `dispatch`, pinned by `native_create_dispatch_table_is_pinned`):
     empty name -> no name-collision check (4F9C39); class 3 / 8 -> 404ED0
     "Actor/Group class requires a GACD: %s" (line 0x7F6 / 0x82A) and null;
     class 4 -> "Kingdom class requires a GKCD: %s" (0x821) and null;
     classes 2/5/6/7/9/10 -> constructors 4EA5E0/4E66D0/4DB610/4DBEE0/
     4DCB30/4DC450, then vf E4, E8, EC(params, 0), 4F9510, 50C220, vf B0 ->
     42A590 (null + 4F5490 if E4 or E8 is false); classes 0/1/>=11 -> native
     null dereference at 4F9F6E (report an error, never success).
     The fresh creation consumes the authoritative RNG (variant -1 ->
     weighted aspect selection, random scale) HERE, in queue order, after
     PostLoad — not at the record's position in the stream.
   - Output: per entry `Replacement { index, saved_id, created, warning }`;
     the warning (5A1040) is `load_warning_replaced_changed_gclass|s` or
     `load_warning_cant_replace_changed_gclass|s` with the saved IDS.

4. **Reporting**: converted records are not failures. Expected effect on
   inventory group 1 once wired: 7,879 -> 0 failed records (every record is
   converted natively; oracle section `real`). Until `4F9C10`/`4DCB30` are
   ported, the replacement step will surface as one unported stop per map
   (27 maps) instead of 7,879 record failures. Once ported: 7,878 trees/
   grasses/hedges/walls are created fresh as DynamicDoodads (class 9, ctor
   4DCB30) and london_bridge (TrailerMaps/axis08_sealion.RMP, saved ID 8089,
   now an Actor definition) logs "can't replace" (native null, not a failure).

## C2. Note for the definitions agent's R3 text (no action needed)

Record +2C is the Object variant word (version >= 1, else -1), not an owner;
the saved name is not copied into the creation block (its name is empty);
only direction x, y are passed. README.txt here has the corrected layout.

## C3. Noted engine limit (restoration loop, no action in my files)

`4FC340` reads a section index into a fixed 0x40004-byte block (more than
65,536 entries overflow it natively). The Rust inventory uses growable
storage; please record this removed cap with the loop port.

# Round 2 (2026-10-15): fresh creation

Module `crates/aa-mapworld/src/fresh_creation.rs`; evidence
`native-fresh-creation.json` (`native_fresh_creation.py`), replayed by
`crates/aa-mapworld/tests/fresh_creation.rs` (4 tests).

## C4. Hooks: who calls what

All entry points take the x87 precision explicitly (simulation: PC53).

1. **Restoration (class-changed replacement)**. This replaces the
   `ReplacementWorld` you would otherwise write:
   ```rust
   let mut fresh = aa_mapworld::fresh_creation::FreshReplacement {
       world: &mut your_fresh_world, pc: Precision::Double53, created: Vec::new() };
   let replaced = class_conversion::replace_converted(queue, &mut fresh)?;
   // fresh.created: Vec<FreshObject<D, Event>> to insert into the world store
   ```
   `your_fresh_world: impl FreshWorld<D>` has one method per original
   subsystem function (see the trait docs):
   - GWorld RNG +D4 (`NativeRng`, the single authoritative stream), begin
     time +E0, the direction table, GMap tile size/shift, map mode, scene gate;
   - identity = `aa_actors::IdentityRegistry`: `register_requested_id` =
     `seed_saved_generation` + `register_saved`; `allocate_id` = `allocate`
     with `Exhausted` -> `None` (oracle-matched cursor/highest/count);
   - spatial index insert/remove (must set/clear object flag bit 26),
     placement test 4F4590, terrain height 4F0E50 and snap 46EE00
     (aa-spatial), blocking 4EB6E0, tile footprint 4EA150, visibility 4EBC70,
     ground paint 4EB500;
   - world list GWorld+8, available cities GWorld+260 (with the B3CC40
     assertion latch), scene add/transform, creation FX;
   - deferred queue add (GWorld+1E8 vf C; probably aa-world
     `Queues::add_after`, unverified) and unlink 4F1A70, destroy 4F5490, logs.

2. **Combat summon (`aa-combat summon.rs` `SummonWorld::create`)**:
   - `Creator::Generic` + `Descriptor::Object` (A25B1C):
     `fresh_creation::create_thing(&ObjectCreationParams { definition,
     position_bits, direction_bits, name_utf16: vec![], variant: u32::MAX,
     requested_id: 0 }, &mut world, pc)?.object.map(|o| o.gtid)`.
     Complete for class 2/9 definitions. Class 3/4/8 definitions return
     `None` after the GACD/GKCD log exactly as the original. Classes 0/1/>=11
     are the native null dereference (`FreshError::NullDereference`).
   - `Creator::Actor` (4FA030): `fresh_creation::create_gactor(&mut entry,
     position_bits)`.
   - `Creator::Organization` (4FA150):
     `fresh_creation::create_organization(&mut entry, &mut OrganizationParams
     { position_bits, facing_bits, byte_51: flag51 as u8 })`.
   - For both, `entry: impl EntryWorld`: `create` = 4F9C10 on the A25B28
     block (actor path, see C6), `spawn_member` = 57AD30 (combat
     regeneration / `world_creation::spawn_member` 4FBF40), `member_count` =
     57A730, latches B3CC43 / B3CC44, assertion dialog 855D30.

3. **Triggers reinforcements / economy BuildActor / construction
   completion** create actors (A25B28 blocks, 4F9C10 class 3:
   `aa-production construction.rs` `create_actor` / `replace_create`,
   `aa-triggers actions.rs` transport) and are blocked on C6. Object
   (class 2/9) creations from any area call `create_thing` as in 2.

## C5. Definitions agent: inputs for real-data creation

`ObjectDefinitionInputs` needs, per Object/DynamicDoodad definition, values
aa-rules does not expose yet:
- +24 (radius factor read by 4EBE50);
- +34 city byte (definition virtual 24, `462E50`);
- +38 shape A {+0 kind, +4 radius, +10, +14};
- the DynamicDoodad component definitions' +10 (the 5904A0 delay factor;
  probably `update_time`, the first scalar `native_dynamic_doodad` reads,
  unverified);
- the +10 display string.

Already available: flags +1C (`native_final_flags`), scale +28/+2C
(`native_object_definition`), +108/+10C/+110/+128
(`native_creation_definition`), class (`native_category`). With these the
7,878 converted records can be created in the oracle and in Rust.

## C6. Actor (class 3) fresh creation: next-round candidate

Not done. No `CreateWorld` composes:
- the actor constructor 4D6BB0 (`aa_actors::creation::construct_actor`, no
  component factories);
- InitCreate 4D6F10 (`aa_actors::init_create`);
- InitExisting 4D7230 (unported);
- PostCreate 4CF300 (`aa_actors::world_publication`);
- the actor branch of 4F9510 (vf B4 -> 4D62F0);
- the 31 component factories 4CF480 (owned by several agents).

The 4FA030 / 4FA150 / 4FBF40 control flows are ported; their actor `create`
call is the missing piece. Proposed: one owner for the actor `CreateWorld`,
using the same Unicorn approach (native chain, component constructors as
recorded seams).

## C7. Noted native limit

`4EFDB0` refuses a new GTID when 0x8000 IDs are live (logged "no IDs left").
aa-actors keeps it as `NATIVE_ADD_LIMIT`, because GTIDs are 16-bit slots in
the saved/network format. It is not removed here.
