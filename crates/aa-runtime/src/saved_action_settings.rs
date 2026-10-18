//! Effective settings read by saved Mind+B4: 530510 FogOfWar and 521E10
//! MoraleRetreatPenalty. RULES is distinct from MAP_HEADER. Native 512420
//! starts with definition defaults, overlays same-type records by identity,
//! preserves the serialized locked byte, and 512200 applies values regardless
//! of that byte. Evidence: research/resume-20261016-saved-mind-actions/settings.
use crate::catalog::Catalog;
use aa_mapworld::{WorldInventory, header::WorldRules};
use aa_numeric::Precision;
use aa_rules::{actors::ResolvedNode, native::TypedValue};

type Result<T> = std::result::Result<T, String>;

pub(crate) fn rules(inventory: &WorldInventory) -> Result<WorldRules> {
    let sections: Vec<_> = inventory
        .opaque_initialization
        .iter()
        .filter(|s| s.name.eq_ignore_ascii_case("RULES"))
        .collect();
    let [section] = sections.as_slice() else {
        return Err("saved Mind settings require exactly one retained RULES chunk".into());
    };
    if section.source.bytes != section.bytes.len() as u64
        || section.source.sha256 != aa_rules::sources::sha256(&section.bytes)
    {
        return Err("saved Mind RULES bytes differ from their recorded source".into());
    }
    WorldRules::decode_bytes(&section.bytes).map_err(|e| e.to_string())
}

fn mode(catalog: &Catalog<'_>) -> Result<u32> {
    match catalog.native_load_mode {
        Some(value @ (1 | 2 | 4)) => Ok(value),
        _ => Err(
            "saved Mind settings require explicit native MapResource load mode 1, 2, or 4".into(),
        ),
    }
}

fn text(node: &ResolvedNode<'_>, field: &str, precision: Precision) -> Result<String> {
    match node.field(field, precision).map_err(|e| e.to_string())? {
        Some(TypedValue::String { value, .. }) if value.is_ascii() => Ok(value),
        _ => Err(format!(
            "saved Mind settings require an ASCII WorldRule {field} string"
        )),
    }
}

fn source_rows<'a>(
    catalog: &'a Catalog<'_>,
    name: &str,
) -> Result<&'a [crate::settings::Declaration]> {
    catalog
        .settings
        .as_ref()
        .ok_or("saved Mind source settings are unavailable")?
        .declarations
        .get(&name.to_ascii_lowercase())
        .map(Vec::as_slice)
        .ok_or_else(|| format!("saved Mind SVar {name} has no source declaration"))
}

fn fog_source(catalog: &Catalog<'_>) -> Result<i32> {
    let [row] = source_rows(catalog, "FogOfWar")? else {
        return Err(
            "FogOfWar SVar override order outside the proven single source declaration".into(),
        );
    };
    if !row.source.path.eq_ignore_ascii_case("Game/SVars.tgi")
        || !row
            .field_type
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("int"))
    {
        return Err("FogOfWar requires its Game/SVars.tgi int declaration".into());
    }
    // 401940 / 40E110: a signed integer, bounded to the initial [0,2].
    row.raw
        .trim()
        .parse::<i32>()
        .map(|v| v.clamp(0, 2))
        .map_err(|_| "FogOfWar requires a supported literal integer source value".into())
}

/// Registered474C50 source default, before4902B1 takes the reset snapshot.
/// The signed value remains distinct from its later nonzero Bool getter.
pub(crate) fn source_fog_definition(catalog: &Catalog<'_>, precision: Precision) -> Result<(i32,bool)> {
    let _source_value = fog_source(catalog)?;
    let registry = catalog
        .actors
        .native_data_registry("WorldRule")
        .ok_or("native WorldRule registration is unavailable")?;
    let definition = registry
        .find("FogOfWar")
        .ok_or("native FogOfWar WorldRule definition is unavailable")?;
    if !definition.ids.eq_ignore_ascii_case("FogOfWar") {
        return Err("FogOfWar lookup translates to an unproven SVar definition".into());
    }
    let node = catalog
        .actors
        .native_data_node(definition)
        .map_err(|e| e.to_string())?;
    if !text(&node, "type", precision)?.eq_ignore_ascii_case("enum") {
        return Err("FogOfWar WorldRule type is outside the proven enum scope".into());
    }
    let values = node.children("EnumValue").map_err(|e| e.to_string())?;
    let names: Vec<_> = values
        .iter()
        .map(|v| text(v, "IDS", precision))
        .collect::<Result<_>>()?;
    if names.len() != 3
        || !names
            .iter()
            .zip(["NONE", "EXPLORED", "FULL"])
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    {
        return Err(
            "FogOfWar enum domain differs from the proven NONE/EXPLORED/FULL definition".into(),
        );
    }
    // 490B6E -> 473440: first case-insensitive EnumValue match is its index.
    let default = text(&node, "default", precision)?;
    let default = names
        .iter()
        .position(|v| v.eq_ignore_ascii_case(&default))
        .ok_or("FogOfWar WorldRule default is absent from its EnumValue list")?
        as i32;
    let editable = node
        .field("always_editable", precision)
        .map_err(|e| e.to_string())?
        .ok_or("FogOfWar always_editable definition is unavailable")?
        .boolean()
        .map_err(|e| e.to_string())?;
    Ok((default,editable))
}

fn fog_value(catalog: &Catalog<'_>, saved: &WorldRules, precision: Precision) -> Result<i32> {
    let load_mode = mode(catalog)?;
    game_type(catalog, saved)?;
    let (default,editable)=source_fog_definition(catalog,precision)?;
    // Immutable registry binding retained for matching serialized identities.
    let registry=catalog.actors.native_data_registry("WorldRule")
        .ok_or("native WorldRule registration is unavailable")?;
    let definition=registry.find("FogOfWar")
        .ok_or("native FogOfWar WorldRule definition is unavailable")?;
    // 474C50 applies this definition default before the B39AD0 snapshot;
    // 511CE0 establishes it again before the serialized records are read.
    let mut value = default;
    for row in &saved.rules {
        let key = c_string(&row.key, "WorldRule IDS")?;
        let Some(found) = registry.find(&key) else {
            continue;
        };
        if found.path == definition.path && row.kind == 3 {
            // Native record+4 is `locked`, not enabled. It does not gate Load
            // or UpdateSVars. Later recognized records replace earlier ones.
            value = row.value as i32;
        }
    }
    // GWorld4FC600: mode bit1 invokes511640. Only always-editable definitions
    // reset; ordinary FogOfWar inherits false from template_world_rule.tgi.
    if load_mode & 2 != 0 && editable {
        value = default;
    }
    // 474C50 sets bounds [0, enumCount-1]; 512160 clamps on SVar publication.
    Ok(value.clamp(0, 2))
}

/// 53056F..53057E, read only when the native Element contributor asks for it.
pub fn fog_of_war(
    catalog: &Catalog<'_>,
    inventory: &WorldInventory,
    precision: Precision,
) -> Result<bool> {
    Ok(published_fog_integer(catalog,inventory,precision)? != 0)
}

pub(crate) fn published_fog_integer(catalog:&Catalog<'_>,inventory:&WorldInventory,precision:Precision)->Result<i32> {
    fog_value(catalog,&rules(inventory)?,precision)
}

/// 521F13..521F25. Current authored WorldRules have no rule targeting this
/// SVar; refuse a newly introduced rule until its typed definition/default
/// merge is supported, rather than silently discarding a saved override.
pub fn morale_retreat_penalty(
    catalog: &Catalog<'_>,
    inventory: &WorldInventory,
    precision: Precision,
) -> Result<u32> {
    mode(catalog)?;
    game_type(catalog, &rules(inventory)?)?;
    morale_value(catalog, precision)
}

/// 512360: an empty GameType IDS stores null; a nonempty unknown IDS throws.
pub(crate) fn game_type(catalog: &Catalog<'_>, saved: &WorldRules) -> Result<()> {
    // Native tests the serialized CString length before Find. A nonempty
    // serialized string starting with NUL still calls Find("") and throws.
    if saved.game_type.is_empty() {
        return Ok(());
    }
    let name = c_string(&saved.game_type, "WorldRules GameType")?;
    if name.is_empty() {
        return Err(
            "unknown WorldRules GameType (nonempty serialized string starts with NUL)".into(),
        );
    }
    let registry = catalog
        .actors
        .native_data_registry("GameType")
        .ok_or("native GameType registry is unavailable")?;
    registry
        .find(&name)
        .map(|_| ())
        .ok_or_else(|| format!("unknown WorldRules GameType {name}"))
}

/// 470470/48D500/4915F0 use NUL-terminated UTF16 keys, independently of the
/// saved CString length. Code units after the first NUL are not lookup text;
/// even an unpaired surrogate in that unused tail must not reject the key.
fn c_string(units: &[u16], label: &str) -> Result<String> {
    let end = units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units.len());
    String::from_utf16(&units[..end])
        .map_err(|_| format!("{label} lookup prefix contains unpaired UTF16"))
}

fn morale_value(catalog: &Catalog<'_>, precision: Precision) -> Result<u32> {
    let registry = catalog
        .actors
        .native_data_registry("WorldRule")
        .ok_or("native WorldRule registration is unavailable")?;
    if registry
        .entries
        .iter()
        .any(|d| d.ids.eq_ignore_ascii_case("MoraleRetreatPenalty"))
    {
        return Err(
            "MoraleRetreatPenalty WorldRule override is outside the proven source scope".into(),
        );
    }
    let rows = source_rows(catalog, "MoraleRetreatPenalty")?;
    if rows.len() != 2 {
        return Err("MoraleRetreatPenalty requires both proven source declarations".into());
    }
    let mut effective = None;
    for path in ["Game/SVars.tgi", "Game/SVars_AA.tgi"] {
        let matched: Vec<_> = rows
            .iter()
            .filter(|r| r.source.path.eq_ignore_ascii_case(path))
            .collect();
        let [row] = matched.as_slice() else {
            return Err("MoraleRetreatPenalty source declarations are ambiguous".into());
        };
        if !row
            .field_type
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("fixed"))
        {
            return Err("MoraleRetreatPenalty source must be fixed".into());
        }
        effective =
            Some(aa_rules::native::scalar_literal(&row.raw, precision).map_err(|e| e.to_string())?);
    }
    effective.ok_or_else(|| "MoraleRetreatPenalty has no resolved source value".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_rules::{
        actors::{
            ActorCatalog,
            native_load::{NativeLoadReport, load_files},
        },
        sources::{Origin, ResolvedFile, sha256},
        tgi::Limits,
    };
    use serde_json::Value;

    fn file(path: &str, text: &str) -> ResolvedFile {
        let bytes = text.as_bytes().to_vec();
        ResolvedFile {
            logical_path: path.into(),
            origin: Origin::V14Loose,
            sha256: sha256(&bytes),
            bytes,
        }
    }
    fn actors(editable: bool) -> ActorCatalog {
        let text = format!(
            "[WorldRule]\n{{\nstring IDS = FogOfWar\nlocalized name = \"Fog\"\nstring type = enum\nstring default = EXPLORED\nflag always_editable = {editable}\n[List EnumValue]\nstring IDS\nlocalized name\n[EnumValue]\nIDS = NONE\nname = \"None\"\n[EnumValue]\nIDS = EXPLORED\nname = \"Explored\"\n[EnumValue]\nIDS = FULL\nname = \"Full\"\n}}\n[GameType]\n{{\nstring IDS = NORMAL\nlocalized name = \"Normal\"\n}}\n"
        );
        let mut report = NativeLoadReport::default();
        let catalog = load_files(
            vec![file("Game/world_rules_aa.tgi", &text)],
            Limits::default(),
            &mut report,
        );
        assert!(
            catalog.native_data_find("WorldRule", "FogOfWar").is_some(),
            "{:?} {:?}",
            catalog.diagnostics,
            report.events
        );
        catalog
    }
    fn settings() -> crate::settings::Settings {
        crate::settings::Settings::from_files(
            [
                file(
                    "Game/SVars.tgi",
                    "[SVars]\n{\nint FogOfWar = 2\nfixed MoraleRetreatPenalty = 5\n}\n",
                ),
                file(
                    "Game/SVars_AA.tgi",
                    "[SVars]\n{\nfixed MoraleRetreatPenalty = 0\n}\n",
                ),
            ],
            Limits::default(),
        )
        .unwrap()
    }
    #[test]
    fn saved_action_settings_replay_native_fog_merge_and_clamp() {
        let doc: Value = serde_json::from_slice(include_bytes!(
            "../../../research/resume-20261016-saved-mind-actions/settings/native-fog-rules.json"
        ))
        .unwrap();
        for case in doc["cases"].as_array().unwrap() {
            let actors = actors(case["always_editable"].as_bool().unwrap());
            let mut catalog = Catalog::new(&actors);
            catalog.settings = Some(settings());
            catalog.native_load_mode = Some(if case["editor"].as_bool().unwrap() {
                2
            } else {
                1
            });
            let rows = case["records"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| aa_mapworld::header::Rule {
                    key: units(&r[0]),
                    enabled: r[1].as_u64().unwrap() as u8,
                    kind: r[2].as_u64().unwrap() as u32,
                    value: r[3].as_i64().unwrap() as u32,
                })
                .collect();
            let saved = WorldRules {
                version: 4,
                mode: 0,
                game_type: Vec::new(),
                rules: rows,
            };
            let actual = fog_value(&catalog, &saved, Precision::Double53).unwrap();
            assert_eq!(
                actual,
                case["output"]["fog_integer"].as_u64().unwrap() as i32,
                "{}",
                case["name"]
            );
        }
    }
    fn units(value: &Value) -> Vec<u16> {
        match value.as_str() {
            Some(text) => text.encode_utf16().collect(),
            None => value
                .as_array()
                .unwrap()
                .iter()
                .map(|unit| unit.as_u64().unwrap() as u16)
                .collect(),
        }
    }
    #[test]
    fn saved_action_settings_game_type_cstring_prefix_matches_native() {
        let doc: Value = serde_json::from_slice(include_bytes!(
            "../../../research/resume-20261016-saved-mind-actions/settings/native-fog-rules.json"
        ))
        .unwrap();
        let actors = actors(false);
        let catalog = Catalog::new(&actors);
        for case in doc["game_type_cases"].as_array().unwrap() {
            let saved = WorldRules {
                version: 4,
                mode: 0,
                game_type: units(&case["units"]),
                rules: Vec::new(),
            };
            assert_eq!(
                game_type(&catalog, &saved).is_ok(),
                case["output"]["accepted"].as_bool().unwrap(),
                "{}",
                case["name"]
            );
        }
    }
    #[test]
    fn saved_action_settings_refuse_missing_mode_and_wrong_definition() {
        let actors = actors(false);
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(settings());
        let saved = WorldRules {
            version: 4,
            mode: 0,
            game_type: Vec::new(),
            rules: Vec::new(),
        };
        assert!(
            fog_value(&catalog, &saved, Precision::Double53)
                .unwrap_err()
                .contains("load mode")
        );
        catalog.native_load_mode = Some(4);
        assert_eq!(fog_value(&catalog, &saved, Precision::Double53).unwrap(), 1);
        catalog
            .settings
            .as_mut()
            .unwrap()
            .declarations
            .get_mut("fogofwar")
            .unwrap()[0]
            .field_type = Some("bool".into());
        assert!(fog_value(&catalog, &saved, Precision::Double53).is_err());
    }
    #[test]
    fn saved_action_settings_morale_source_values_match_original_setter() {
        let doc:Value=serde_json::from_slice(include_bytes!("../../../research/resume-20261016-saved-mind-actions/settings/native-morale-settings.json")).unwrap();
        let actors = actors(false);
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(settings());
        for case in doc["cases"].as_array().unwrap() {
            let precision = match case["precision"].as_str().unwrap() {
                "Single24" => Precision::Single24,
                "Double53" => Precision::Double53,
                "Extended64" => Precision::Extended64,
                _ => unreachable!(),
            };
            for state in case["states"].as_array().unwrap() {
                catalog
                    .settings
                    .as_mut()
                    .unwrap()
                    .declarations
                    .get_mut("moraleretreatpenalty")
                    .unwrap()[1]
                    .raw = state["value"].as_str().unwrap().into();
                assert_eq!(
                    morale_value(&catalog, precision).unwrap(),
                    state["bits"].as_u64().unwrap() as u32
                );
            }
        }
    }
    #[test]
    #[ignore = "requires AA_GAME_ROOT original archive and v14 overlay"]
    fn saved_action_settings_original_sources_and_all_map_rules() {
        let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut sources = aa_rules::sources::V14Sources::open(
            game.join("Data.rwd"),
            game.join("reconstruction/native-game/runtime-expanded-v14"),
            aa_rules::sources::SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let settings =
            crate::settings::Settings::load_v14(&mut sources, Limits::default()).unwrap();
        let actors = ActorCatalog::load_v14(&mut sources).unwrap();
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(settings);
        catalog.native_load_mode = Some(1);
        assert_eq!(morale_value(&catalog, Precision::Double53).unwrap(), 0);
        let doc: Value = serde_json::from_slice(include_bytes!(
            "../../../research/resume-20261016-saved-mind-actions/settings/native-fog-rules.json"
        ))
        .unwrap();
        let mut counts = [0usize; 3];
        for row in doc["original_map_corpus"].as_array().unwrap() {
            let path = std::path::PathBuf::from(row["path"].as_str().unwrap());
            let archive =
                aa_mapworld::MapArchive::from_bytes(std::fs::read(path).unwrap()).unwrap();
            let inventory = WorldInventory::decode(&archive, 256 * 1024 * 1024).unwrap();
            let actual =
                fog_value(&catalog, &rules(&inventory).unwrap(), Precision::Double53).unwrap();
            assert_eq!(
                actual,
                row["output"]["fog_integer"].as_u64().unwrap() as i32
            );
            assert_eq!(
                fog_of_war(&catalog, &inventory, Precision::Double53).unwrap(),
                actual != 0
            );
            assert_eq!(
                morale_retreat_penalty(&catalog, &inventory, Precision::Double53).unwrap(),
                0
            );
            counts[actual as usize] += 1;
        }
        assert_eq!(counts, [30, 50, 0]);
    }
}
