# Requests to lead (phase8 layouts)

## L-1 Wiring for the restoration agent (R2-a): exact API

The producer is `aa_rules::native::layout_data` (file
`crates/aa-rules/src/native_layout_data.rs`, module line appended in
`crates/aa-rules/src/native.rs`). Build it once per catalog, next to the
`ActorCatalog` the `RestoreDefinitions` adapter already holds
(aa-runtime `catalog.rs`):

```rust
use aa_rules::native::layout_data::{NativeLayoutData, native_actor_properties};
// once (e.g. a field or OnceCell beside `actors`):
let layouts: NativeLayoutData = actors.native_layout_data(pc).map_err(|e| e.to_string())?;
```

`pc`: the producer's only precision-dependent values are the ActorData
OrganizationComponent `+304` (operational_range squared) and
`rampage_direction` products; all shipped data gives identical results at
PC24/PC53/PC64 (oracle-checked). Synthetic subnormal products do differ at
PC24, so keep it explicit. The data load runs on the main thread; whether
that is before (PC53) or after (PC24) `CreateDevice` is not established here.

Then, in `RestoreDefinitions` (aa-mapworld restore.rs trait, aa-runtime
catalog.rs impl):

```rust
// 463FA0 on [B3AB00] (4C3050). Translations of the OrganizationLayout
// manager are applied; an empty IDS is None.
fn organization_layout_data(&self, ids: &[u16], _: Precision) -> Result<Option<String>, String> {
    let Ok(id) = String::from_utf16(ids) else { return Ok(None) };
    Ok(self.layouts.find_layout(&id).map(|l| l.ids.clone()))
}
fn organization_layout_info(&self, layout: &str, _: Precision)
    -> Result<aa_actors::organization_state::LayoutInfo, String> {
    let l = self.layouts.find_layout(layout).ok_or_else(|| format!("no LayoutData {layout}"))?;
    let f = self.layouts.layout_info(l).map_err(|e| e.to_string())?;
    Ok(aa_actors::organization_state::LayoutInfo {
        name: f.name,
        primary_slot: f.primary_slot,
        entries: f.entries.into_iter().map(|e| aa_actors::organization_state::LayoutEntry {
            property: e.property,
            property_name: e.property_name,
            required: e.required,
            locked: e.locked,
            morale_line: e.morale_line,
        }).collect(),
        slot_groups: f.slot_groups,
        formations: f.formations,
        formation_44: f.formation_44,
        formation_48: f.formation_48,
        property_50: f.property_50,
    })
}
```

In `organization_restore.rs` (`RestoreOrganizationWorld`), through two new
`RestoreDefinitions` methods (or direct access to `layouts`):

```rust
// Fact: LayoutFormation +C FormationData +1A byte and +24 property handle.
fn formation_facts(&self, formation: u32) -> Result<(u8, u32)> {
    layouts.formation_facts(formation)            // -> Result<(u8, u32), LayoutError>
}
// 45EEF0 on an ActorData (canonical Thing IDS).
fn has_property(&mut self, actor_data: &String, property: u32) -> Result<bool> {
    layouts.actor_has_property(actors, actor_data, property, pc)
}
```

ActorData `+2F0` default layout (OwnerFacts::default_layout): the existing
`ground.rs` OrganizationDefinition (layout id + parameters) is identical to
`layouts.organization_component(actors, actor_id, pc)?` for all 192 shipped
organization definitions (`parameters` canonical IDS, `layout_ids`), so the
current catalog value can stay; `organization_component` additionally gives
every OrganizationComponent field (`+2F4..+31D`, `[+314]` rampage block) and
the native element-link checks.

Handle conventions (`u32`, nonzero, standing for native pointers):
* Property: registered definition = Property registry ordinal (`+C`) + 1;
  embedded ModifierPropertyData (layout BrokenProperty `+50`, formation
  `+24`) = `0x8000_0000 | (index + 1)`. `PropertyRef::{handle, from_handle}`.
* LayoutFormation (`+34` entries): global creation order + 1;
  `NativeLayoutData::layout_formation(handle)`.
These are the only handles `apply_property` / `remove_property` /
`formation_facts` will see from the layout side; the Property producer
(contents of 4C42D0) must key its objects the same way.

## L-2 Saved Organization records that cannot load natively (restoration)

Of the 1,082 saved Organization payloads in the 80 maps
(research/phase8-saved-components/real-payloads.json) every saved LayoutData
IDS resolves and every saved parameter count equals its LayoutData entry
count. But 143 records in 29 maps (campaign maps included, e.g.
axis06_kursk 8, allies04_kursk 7, the *_ART maps 6 each) save an ActorData
parameter with no v14 definition (`*_hq_*_bunker`, `*_hq_bunker_aa`,
`ger_light_tank`, `jpn_command_tank`, `usa_artillery_infantry`, ...; their
translations sit in the commented-out translate.tgi block). 4C2E60 throws
"Unknown IDS %s for class %hs" for them inside 57D130. Please establish
whether natively those owners are skipped/converted before 57D130 (owner
definitions such as regiment_ger_hq_infantry_unpacked exist), or the
exception reaches a catch, before treating them as restore failures. Two of
them (cutscene_short_02, regiment_jpn_tankette) also save 6 slots for a
7-slot LayoutData (57D130 "Layout size changed" would follow).

## L-3 TGI lexer divergence (owner of crates/aa-rules/src/tgi.rs)

The original parser keeps a value that starts with `//` as the value
(`icon = //lead/x.png` -> 5B54D0 returns `//lead/x.png`; oracle case
`formation_icon_double_slash` in native-layout-data.json). tgi.rs treats
`//` anywhere as a comment start and returns an empty value. No shipped
file uses such a value (the shipped replay matches), so nothing in the
80 maps depends on it. The Rust replay lists this case in
KNOWN_PARSER_DIVERGENCES instead of asserting it.

## L-4 Definitions agent: Property is a generic DataManager

GameData `+464` is rebuilt at 48B1B4 by 47EFD0(0x40, 1): a DataManager with
vf0 483A40, vf1C 48B580 (mode **1**), create 48B640 (`+C` ordinal), vf18
48D490 (= 463FA0, same Find). tag-managers.json lists it with `manager:
null`, so `GENERIC_DATA_TAGS` lacks "Property". This producer keeps its own
Property registry (same rules, oracle-proven on the shipped files); adding
"Property" (mode 1) to native_load would let `native_named_definition`
use it too. Also: native_load registers OrganizationLayout/Formation roots
without their type LoadData, so a throw inside 4C3610/4C34A0 does not
abandon the rest of the file there; this producer models it (and refuses a
file where native_load records a different first exception).

## L-5 Not done here (other owners)

* KKC_ModifierPropertyData::LoadData 4C42D0 (Property contents: modifiers,
  icons, FX, required properties, its 47D960 name step and throws) for the
  BrokenProperty and formation properties - recorded as seams only.
* GroupFormation type LoadData (49A660 object; group formation positions,
  aa-movement area) - registration only.
* Organization callees listed in R2-b other than formation facts and 45EEF0.

No Cargo.toml / Cargo.lock change. No original game file was modified.
