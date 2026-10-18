use super::*;
use serde_json::{Value, json};
struct Probe<'a> {
    input: &'a Value,
    trace: Vec<String>,
}
impl World for Probe<'_> {
    type Variable = ();
    type Player = usize;
    fn minor(&mut self) -> Result<bool> {
        Ok(self.input["minor"].as_u64().unwrap_or(0) != 0)
    }
    fn variable(&mut self) -> Result<()> {
        self.trace.push("lookup_kingdom".into());
        Ok(())
    }
    fn find_player(&mut self) -> Result<Option<usize>> {
        let default = json!([{"display":"Kingdom","link":true}]);
        let players = self
            .input
            .get("players")
            .unwrap_or(&default)
            .as_array()
            .unwrap();
        let state = self.input["session"].as_u64().unwrap_or(2);
        Ok(players.iter().position(|p| {
            if state == 2 {
                self.input["world"].as_bool().unwrap_or(true)
                    && p["link"].as_bool().unwrap_or(false)
            } else {
                p["display"].as_str().unwrap().split('\0').next()
                    == self.input["name"]
                        .as_str()
                        .unwrap_or("Kingdom")
                        .split('\0')
                        .next()
            }
        }))
    }
    fn player_glyph(&mut self, p: &usize) -> Result<Option<u16>> {
        self.trace.push(format!("player_glyph:{p}"));
        Ok(self.input["selected_nonnull"]
            .as_bool()
            .unwrap_or(true)
            .then_some(self.input["selected"].as_u64().unwrap_or(132) as u16))
    }
    fn fallback(&mut self, _: &()) -> Result<u16> {
        self.trace.push("find:ch_gem".into());
        Ok(self.input["fallback"].as_u64().unwrap_or(140) as u16)
    }
    fn rgb(&mut self) -> Result<[u32; 3]> {
        Ok(self.input["rgb"]
            .as_array()
            .map(|v| {
                v.iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            })
            .unwrap_or([0, 0x3e99999a, 0x3f800000]))
    }
    fn formatted(&mut self, rgb: [u32; 3], glyph: u16) -> Result<Vec<u16>> {
        self.trace.push("format".into());
        format_color(rgb, glyph)
    }
}
#[test]
fn native_selection_exact_promotions_and_vc71_general_format_replay() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/development10/kingdom-glyph/native-kingdom-glyph.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 84);
    for row in doc["rows"].as_array().unwrap() {
        let mut p = Probe {
            input: &row["input"],
            trace: vec![],
        };
        let flags = p.input["flags"].as_u64().unwrap_or(0) as u32;
        let mut cache = Cache {
            flags,
            variable: (flags & 1 != 0).then_some(()),
        };
        let text = run(&mut cache, &mut p).unwrap();
        assert_eq!(json!(text), row["output"]["text"], "{row}");
        assert_eq!(json!(p.trace), row["output"]["trace"], "{row}");
        assert_eq!(json!(cache.flags), row["output"]["flags"], "{row}");
        if row["output"]["format_args"].is_object() {
            let rgb = p.rgb().unwrap();
            let promoted = rgb.map(|v| f64::from(f32::from_bits(v)).to_bits());
            assert_eq!(json!(promoted), row["output"]["format_args"]["double_bits"]);
        }
    }
    for row in doc["format_rows"].as_array().unwrap() {
        assert_eq!(
            general(row["bits"].as_u64().unwrap() as u32).unwrap(),
            row["text"].as_str().unwrap(),
            "{row}"
        );
    }
    for row in doc["color_rows"].as_array().unwrap() {
        assert_eq!(row["input"], row["stored"]);
        assert_eq!(row["initialized"], 1);
    }
    assert!(
        format_color([0, 0, 0], u16::MAX)
            .unwrap_err()
            .contains("WEOF")
    );
    assert!(general(0x7f800000).unwrap_err().contains("nonfinite"));
    let characters: Value = serde_json::from_str(include_str!(
        "../../../research/development10/kingdom-glyph/native-glyph-character.json"
    ))
    .unwrap();
    for row in characters["rows"].as_array().unwrap() {
        let char = row["char"].as_u64().unwrap() as u16;
        if row["return"] == u32::MAX {
            assert!(format_color([0; 3], char).is_err());
        } else {
            let text = format_color([0; 3], char).unwrap();
            assert_eq!(json!(&text[text.len() - 1..]), row["text"]);
        }
    }
    for row in characters["global_image"].as_array().unwrap() {
        assert_eq!(row["initial_word"], 0);
    }
}

#[test]
fn minor_path_and_required_lookup_keep_native_prefix() {
    struct Cutoff {
        minor: bool,
    }
    impl World for Cutoff {
        type Variable = ();
        type Player = ();
        fn minor(&mut self) -> Result<bool> {
            Ok(self.minor)
        }
        fn variable(&mut self) -> Result<()> {
            Err("lookup cutoff".into())
        }
        fn find_player(&mut self) -> Result<Option<()>> {
            panic!("lookup cutoff precedes Session read")
        }
        fn player_glyph(&mut self, _: &()) -> Result<Option<u16>> {
            unreachable!()
        }
        fn fallback(&mut self, _: &()) -> Result<u16> {
            unreachable!()
        }
        fn rgb(&mut self) -> Result<[u32; 3]> {
            unreachable!()
        }
        fn formatted(&mut self, _: [u32; 3], _: u16) -> Result<Vec<u16>> {
            unreachable!()
        }
    }
    let mut cache = Cache {
        flags: 0,
        variable: None,
    };
    assert!(
        run(&mut cache, &mut Cutoff { minor: true })
            .unwrap()
            .is_empty()
    );
    assert_eq!(cache.flags, 0);
    assert_eq!(
        run(&mut cache, &mut Cutoff { minor: false }).unwrap_err(),
        "lookup cutoff"
    );
    assert_eq!(cache.flags, 1);
    assert!(cache.variable.is_none());
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources"]
fn actual_source_palette_and_same_current_variable_owner() {
    use crate::{
        combat_source_completion, combat_source_receipt::VerifiedStartupReceipt, settings::Settings,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    combat_source_completion::compute(&catalog).unwrap();
    let bank = catalog.definition_bank.borrow();
    let first = player_glyph::State::from_fresh_sources(
        &catalog,
        &mut files,
        crate::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    let second = player_glyph::State::from_fresh_sources(
        &catalog,
        &mut files,
        crate::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    let witness = first.kingdom_variable(&catalog).unwrap();
    assert_eq!(
        first
            .kingdom_fallback(&catalog, &bank, &witness)
            .unwrap()
            .unwrap()
            .index(),
        132
    );
    assert!(
        second
            .kingdom_fallback(&catalog, &bank, &witness)
            .err()
            .unwrap()
            .contains("another current-variable owner")
    );
    let pc = bank.combat_source().unwrap().source_precision();
    let mut rows = Vec::new();
    let registry = actors.native_data_registry("KingdomColor").unwrap();
    for (index, entry) in registry.entries.iter().enumerate() {
        let color = aa_mapworld::restore::NamedDefinition {
            id: entry.ids.clone(),
            index: Some(index as u32),
        };
        let rgb = current_rgb(&catalog, &bank, &color).unwrap();
        let text = format_color(rgb, 132).unwrap();
        rows.push(
            json!({"id":entry.ids,"source":entry.source,"ordinal":index,"rgb":rgb,"text":text}),
        );
        let forged = aa_mapworld::restore::NamedDefinition {
            id: entry.ids.clone(),
            index: Some(index as u32 + 1),
        };
        assert!(current_rgb(&catalog, &bank, &forged).is_err());
    }
    assert!(!rows.is_empty());
    if let Some(path) = std::env::var_os("AA_KINGDOM_GLYPH_CAPTURE") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(
            file,
            &json!({"source_precision":pc,"sources":actors.sources,"palette":rows,"glyph":132}),
        )
        .unwrap();
    }
}
