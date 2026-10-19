461F70 death-Animation damage_filter lookup -- 2026-10-18

Blocker removed
  Pre-World Object/Actor source preparation stopped 68 definitions (every
  infantry/engineer/paratrooper/sniper/commando kind plus saboteur and
  secret_agent; list in qa-output/development10/lead-gui/look01/berlin/
  stderr.txt) at the stub "461F70 positive death damage_filter requires actual
  DamageFilterData+2C registry". The "DamageFilterData" is in fact the
  DamageType DataManager; +2C is DamageTypeData::damage_filter.

Native evidence (AA.exe and v14 identical in every executed byte)
  461F70 4620A6..4620EC: for Animation+8 == 3 (die) and +28 not -1 / 0x29A:
    ecx = [[B36530]+3A4]; for eax in 0..[ecx+28] (signed, jle when <= 0):
      if [[ecx+24]+eax*4]+2C == filter -> 4620EF (hit, nothing stored)
    miss -> 410C40("Animation '%s' for actor '%s' specifies an invalid damage
    filter '%d'", Animation+C, ActorData+8 IDS, filter): thrown.
  Animation+28 is stored BEFORE the lookup (462076) and kept either way.
  48DB60 registers root tag "DamageType" to GameData+3A4; 48AA55 allocates
  0x4C bytes and calls 481410(0xB, mode 0). 481410: vector +24/+28/+2C,
  hash +30, singleton B3AAF8. vf0 483A40 -> vf1C 481570 -> mode0 create
  481630 ("Data IDS already used") -> 4C0870 (0x30 bytes, vtable A228D0,
  +C = -1, +18..+28 zeroed, +2C NOT initialized) -> registered (+C = old
  +28) -> vtable[0] 4C0A80 LoadData: 47D960, 5B4C30 color, 5B54D0 glyph +
  48D500 on [B36530]+3BC (zero -> 410C40 BEFORE +2C), +2C = 5B4ED0
  damage_filter, negative -> 410C40 AFTER the store.
  ActorData vtable A05540[0] is LoadData 464BD0, called by 4BDC50 right after
  4BCA00 registration, i.e. inside its own file's 48D660 dispatch. The
  vector 461F70 sees is therefore "DamageTypes of earlier files" (Game/
  damage*.tgi is loaded long before Units/* and SpecialOps/Units/*).

Oracle: native_damage_filter.py -> native-damage-filter.json
  Original parser 5BD080 over the real template_damage_type.tgi and
  damage_types.tgi, real 481410/483A40/481570/481630/4C0870/4C0A80 and real
  typed getters build the vector; actual 463B50/463440/4632D0/461F70 then
  run 95 Animation cases (6 vectors x 15 filters + 5 kind/skip cases) and
  the authored death Animations of ger_infantry_mg, usa_infantry_flame and
  saboteur (66). 3 PCs x original/v14 = 6 machines, all equal.
  Vectors: real [gunfire 0, flame 1, explosive 2, armor_piercing 2]; empty;
  absent manager (+3A4 = 0: skip cases never read it, lookup cases fault at
  4620BA); duplicate/large values; negative +2C (registered, stored, then
  thrown; the file's later root never registers); unknown glyph (registered,
  +2C never stored: the fixture pre-fills +2C with 0xCDCDCDCD and the
  lookup then compares that word -- native compares allocator bytes).
  Seams: 48D500 Glyph Find (fixture handles for ch_gun/ch_fire/
  ch_explosive/ch_ap), 4B9F40 text lookup (key returned), and the prior
  producer's Animation node getters / Sequence / relative-path transports.
  Default run is an immutable comparison; --write is author-only.

Rust
  crates/aa-runtime/src/actor_animation_damage_filter.rs (new): Registry::
  at_actor_load reuses ActorCatalog::native_data_registry("DamageType") (the
  canonical native-order generic DataManager registry; no second registry),
  takes the entries of files loaded before the Actor's file, and reproduces
  4C0A80's +2C preconditions. Explicit errors: unbound manager / mode != 0,
  DamageType in the Actor's own file, file loaded twice, color/glyph/
  damage_filter of the wrong type, unknown or later/same-file glyph, a Glyph
  translation for that name, a negative damage_filter (native truncates the
  file, the canonical registry does not), a re-loaded entry.
  actor_animation_source.rs: validate_damage_filter builds the vector once
  per ActorData load (lazily, only when a lookup is reached) and returns the
  native 410C40 message on a miss. Nothing else is stored on a hit.

Not claimed
  The Glyph registry content (seam), localized names, the whole ActorData
  loader/Resource/FX path, the 48D660 file truncation after a DamageType
  throw (refused instead), and any later consumer of Animation+28.

Verification (2026-10-18, CARGO_TARGET_DIR=target/agent-filter, -j 6)
  native_damage_filter.py read-only replay: PASS (native-replay-01.log).
  cargo test -p aa-runtime --lib actor_animation --offline -- --include-ignored
  with AA_GAME_ROOT: 7/7 PASS (rust-tests-01.log), including
    actual_461f70_damage_filter_hits_misses_and_skips_replay_native_all_pc
      (483 cases: status, getter trace, appended count, kind, +28, weight,
      exact 410C40 text; absent manager <-> explicit unbound error),
    actual_damage_type_vector_projection_matches_native_4c0a80_registration
      (the SAME source bytes through native_load::load_files: real and
      duplicate vectors equal entry by entry; throwing roots refused),
    damage_type_vector_is_taken_at_the_actor_file_time (ordering contract),
    actual_v14_census_death_filters_resolve_in_native_damage_type_vector
      (all 68 blocked definitions: vector == native real vector, every death
      filter hits, base+aspect sets load through the production 463440 core
      and pass 461190; ger_infantry_mg / usa_infantry_flame / saboteur base
      death lists equal the producer's authored lists).
  cargo test -p aa-rules --lib native_load: PASS (crate unchanged).
  cargo build -p aa-runtime --lib: PASS, no new warnings.
  The GUI census itself was NOT re-run here.
