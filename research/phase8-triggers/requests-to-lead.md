# Phase 8 triggers: requests to the lead

Round 2 status of the round-1 requests first, then new items.

1. **aa-mapworld dev-dependency (done, as approved).** `crates/aa-triggers/Cargo.toml`
   dev-dependencies are now only `serde_json`. `tests/saved_native.rs` reads the
   SHA256-pinned chunks in `research/phase8-triggers/chunks/` (map and chunk
   hashes checked). Cargo.lock was refreshed once with
   `cargo check --offline -p aa-triggers`; the only change is the removal of
   `"aa-mapworld",` from aa-triggers' dependency list. `--locked` builds, tests
   and clippy pass.

2. **Opcode 37 (done, as decided).** No aa-combat dependency. The leaf 543800
   is ported (0.0 test, 4CE6C0 unit, +7C organization, warning path) and proven
   (128 native cases). The morale change itself is the seam
   `aa_triggers::extended::Effects::organization_modify_morale(org, value, 4)`
   (native 579750); the runtime binds it to `aa_combat::morale::modify`.

3. **Runtime binding of the trigger services (open, for the integration
   backlog owners).** `world_program::Program` dispatches all 115 registered
   element types. One runtime type must implement `world_program::World +
   elements::Services`, which now also requires `extended::Effects`:
   sun dimmer 4ED820, SAI profile 79F5D0, music 747630, VFX on actor 50D230,
   fidget variable cell + 40FCA0 permission, game speed 76F370, Kingdom hero
   table 4E2A60/4E1130/4E4C30/4E2CB0, morale 579750, actor display name
   (virtual +24), Area+84 actor list, 4D12C0, 4F6760, teleports 4EE5C0/4EEB40,
   flag stores, 4F03C0, players 76AC60/765E00, audio lookup 416370, and the fog
   tile pass (aa-spatial's 554DD0/5541F0/554330 iterator + 54F930). The
   round-1 list (UI command interpreter 50D120, banner 50CD00, audio
   7476E0/7474F0, camera angle tables, placement 554B30, creation, transports,
   abilities, Kingdom properties/resources, command processing 4D9B60,
   player/selection lookup 76B960) is unchanged. Please add the new names to
   research/phase8-integration-backlog.md (not my file).

4. **MAPSTRINGS (done in aa-triggers).** `aa_triggers::mapstrings::MapStrings`
   parses the chunk (proven subset, explicit errors otherwise) and `bind`
   fills `Value::Localized.inline` exactly like 49AD70.
   `aa_runtime::trigger_world::load_map_triggers(triggers, mapstrings,
   LanguageIDS, budget)` is the map-load entry point; the map loader owner
   should call it (LanguageIDS comes from the game variable). Open: a key that
   is not in MAPSTRINGS is an explicit error, because natively 4B9F40 then
   searches the global KKC_TextDataManager tables (game text) and falls back to
   `error_missing_string`; no original map needs it.

5. **Precision.** Callers pass PC53 (`Precision::Double53`). Opcode 68 is now
   ported: the binary32 store is proven, and results within 2^-40 of a binary32
   rounding boundary are refused because FYL2X (extended precision, not
   correctly rounded; binary64 in Unicorn) is not ground truth there.

6. **Locale (closed in round 3).** AA.exe's only `setlocale` callers
   (7F19D9/7F1A08/7F1AA1) query LC_NUMERIC and only reset it to "C"; nothing
   reaches `__init_ctype`, so the MAPSTRINGS byte reader, `_wcsicmp` and
   `printf` always run in the C locale. MAPSTRINGS bytes >= 0x80 are now
   widened (proven); the oracles use the image's initial locale info.

7. **FYI: shared-checkout breakage.** During this round `aa-navigation`
   (route_cache import in region_search.rs) and `aa-rules` (ability_load.rs)
   failed to compile for a while; aa-triggers depends on aa-navigation, so my
   test runs were blocked until the owners finished. aa-triggers itself
   compiled at every pause point.

8. **FYI: Unicorn caveats.** loader.py/harness.py now fault on any non-FS
   page-0 access; native-elements, native-format and native-mapstrings
   regenerate identical case data. No fixture loads x87 signalling NaNs.

9. **Captured binding for combat (round 3).** `aa_triggers::predicates::
   queue_captured(manager, identity, captured, source, objects)` is the port
   of 54BEA0/54B820, proven against the original (native-capture.json). The
   combat owner should bind `AttemptWorld::capture_notification` to it (the
   doc comment in aa-combat/src/capture.rs names `Manager::push_event`; please
   point it at `queue_captured`, which is not my file). `CaptureObjects` needs
   the object's +14 GTID, virtual +A8 and virtual +108.

10. **Register file (round 3).** The 546AF0 registers are a manager member
    (manager+2C) reused across elements; `Manager::convert_registers` /
    `register_text` model their +4 string. This made the random-hero item 28
    of opcodes 35/48/49 reachable and ported. It needs two more runtime seams:
    `extended::Effects::world_rng` (4F02B0, World+D4) and `random` (85D840,
    the game RNG; presumably aa_simulation::NativeRng), plus
    `actor_data_bank`, `kingdom_hero_count` and `kingdom_hero_at` (4E13F0).
