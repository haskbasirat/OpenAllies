# Requests from the phase8 metagame session (crates/aa-metagame)

1. aa-rules (not owned by this session): `ResolvedNode::field` rejects the TGI
   type `localized` ("type localized not implemented") and `plain_string_field`
   rejects quoted values, so the metagame cannot read authored localization keys
   (`name = "#mg_campaigns_kingdom_jpn_name"`, event/effect `message`, tooltips,
   MGAI `name`). Request: a read-only accessor such as
   `ResolvedNode::localized_key(&self, name) -> Result<Option<(String, Provenance)>>`
   returning the exact quoted literal (quotes stripped, no translation) with the
   same inheritance/declaration checks as `field`. Until then these fields are
   left out of the metagame data model (README gaps).

2. aa-rules TGI parser: two shipped sources are rejected and therefore missing
   from `ActorCatalog` (diagnostics in `ActorCatalog::load_v14`):
   - `Units/USA/usa_carrier.tgi` at byte 2079 "body has no preceding node
     header" (`[Size]` followed by property lines, then a `{ ... }` body).
   - `Game/trigger_elements.tgi` at byte 37375 "unterminated quoted string".
   The native loader evidently accepts usa_carrier (it is the shipped
   `rts_water_actor_ids` of the USA infantry division). aa-metagame reports the
   reference in `MetaGameData::unverified_references` instead of accepting or
   rejecting it. Request: port the native parser behaviour for these forms in
   aa-rules (owner of that crate), then the metagame check becomes complete.

3. No new external crates are needed. aa-metagame uses only its declared deps
   (aa-formats, aa-rules, aa-numeric, serde, serde_json, thiserror). Root
   Cargo.toml / Cargo.lock were not touched. (aa-formats is declared but not
   used yet; it can stay for the save-format work.)

4. Shared evidence gap (all float ports): the x87 precision control of the thread
   that runs the metagame (and the simulation) is still unproven
   (research/phase4-orders/fpu-startup-static.json). aa-metagame takes the
   precision as an explicit parameter and is proven at PC24, PC53 and PC64
   separately; quick-battle results differ between PC24 and PC53/64 (army
   strength sums, technology bonus, odds and the logged roll). Whoever
   establishes the runtime FPU control word should tell this crate's owner.

5. Integration note for whoever owns kingdoms/relations: the metagame battle
   code calls kingdom-actor virtuals vtable+0x140 ("enemy", cf. 954B30) and
   vtable+0x138 (ally-like) and army-actor vtable+0x108 (owning kingdom).
   aa-metagame takes them through the `KingdomRelations` trait; binding them to
   the native kingdom/team relation port (aa-runtime relations.rs covers
   4CF040/4E52D0/4E10F0) needs the vtable slot -> function mapping, which is not
   established here.

## Round 2

6. Dependency request: add `aa-trig = { path = "../aa-trig" }` to
   `crates/aa-metagame/Cargo.toml` (needs a Cargo.lock update, so it is yours
   to apply). Reason: army display positions on the strategic map come from
   the original TerritoryPositionSolver (959420, circle/square layouts 959560 /
   959240) which uses the original 4096-entry sin/cos lookups (85DAA0/85DA80)
   that aa-trig already ports. InitCreate is proven without positions until then
   (positions are recorded in research/phase8-metagame/native-init.json).

Status updates (end of round 2, 2026-10-14):
- Item 1: aa-rules now has `ResolvedNode::localized`. aa-metagame does not
  read localized fields yet. Wiring them in changes metagame-data.json, which
  means regenerating every oracle (about 10 minutes in parallel). That is the
  next data step.
- Item 2: the parser now accepts `Units/USA/usa_carrier.tgi`.
  - `unverified_references` and `unresolved_things` are now empty.
  - metagame-data.json was re-exported, so its sha256 is now 96a66de6...703b.
  - Every oracle was regenerated against it. The outputs are identical to the
    previous ones except for the pinned data sha.
- Item 4: superseded by the lead's PC53 policy. aa-metagame uses
  `GAME_PRECISION = Double53` (thread inference in README.txt).

7. Hazard, fixed in this crate: `cargo test -p aa-metagame -- --include-ignored`
   used to rewrite metagame-data.json (export test). Since every native-*.json
   pins that file's sha256, any change in aa-rules output silently broke all
   pins. The test now FAILS when the export is stale, and rewrites the file only
   with `AA_METAGAME_EXPORT=1`. Anyone changing aa-rules parsing should run it.

8. Data that is not owned here and that the battle launcher (948EF0) needs:
   - the RMC loader 4A4EB0 (sliders +0x50/+0x68/+0x74/+0x80, game settings
     +0x90, slot lists +0xF8/+0x104);
   - the resource registry order (B36530+0x34) as used by ResourceList.
   Until then `launch::LaunchEnv` takes them as inputs. Who owns the RMC port?

## Round 3 (2026-10-15)

- Request 6 (aa-trig): applied by the lead. positions.rs uses `aa_trig::lookup_sin` / `lookup_cos`.
- Request 7 (export check): now in the rules doc.
- Request 8 (RMC): the loader is ported here (rmc.rs).

9. String tables. The native getter 5B5620 translates `#key` values through the string table (4B9F40) at load time. aa-metagame and rmc.rs keep the keys, because the string tables are not part of this crate. This affects metagame names, tooltips and messages, and the CreateExplicitAI order name. Who owns the localization tables (Data/Localization, 4B9F40)?

10. The RMC loader has no Unicorn replay yet. Replaying it needs native TGI node objects (fields, children at node+0x34, getters 5B4xxx/5B5xxx) and the engine registries (things, data managers, world rules, biomes) in the emulator.
    - Does any session already build native TGI nodes under Unicorn? If so, rmc.rs's exported nodes (rmc-data.json) can drive that harness.
    - Otherwise this becomes a dedicated task.

11. KingdomsMax. rmc.rs takes the SVars_AA.tgi assignment, which is loaded after SVars.tgi (index_aa.lst `Game/SVars*.tgi`). That follows the 16-kingdom audit, not a separate native trace. Please confirm this override order is the established one.
