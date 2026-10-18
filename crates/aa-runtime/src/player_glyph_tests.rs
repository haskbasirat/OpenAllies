use super::*;
use serde_json::{Value, json};
struct Probe<'a> {
    input: &'a Value,
    trace: Vec<String>,
}
impl Probe<'_> {
    fn step(&mut self, name: &str) -> Result<()> {
        self.trace.push(name.into());
        if self.input["cutoff"].as_str() == Some(name) {
            Err(name.into())
        } else {
            Ok(())
        }
    }
    fn flag(&self, name: &str, default: bool) -> bool {
        self.input[name].as_bool().unwrap_or(default)
    }
}
impl World for Probe<'_> {
    type Variable = Slot;
    type Glyph = u32;
    fn lookup_string(&mut self, slot: Slot) -> Result<Slot> {
        self.step(match slot {
            Slot::Kingdom => "lookup_kingdom",
            Slot::Observer => "lookup_observer",
        })?;
        Ok(slot)
    }
    fn ai(&mut self) -> Result<bool> {
        self.step("ai")?;
        Ok(self.input["ai"].as_u64().unwrap_or(0) != 0)
    }
    fn online_manager(&mut self) -> Result<bool> {
        self.step("online_manager")?;
        Ok(self.flag("online", false))
    }
    fn peer(&mut self) -> Result<bool> {
        self.step("peer")?;
        Ok(self.flag("peer", false))
    }
    fn online_glyph(&mut self) -> Result<Option<u32>> {
        self.step("online_glyph")?;
        Ok(self.flag("online_result", false).then_some(1))
    }
    fn session(&mut self) -> Result<u32> {
        self.step("session")?;
        Ok(self.input["session"].as_u64().unwrap_or(2) as u32)
    }
    fn kingdom(&mut self) -> Result<bool> {
        self.step("kingdom")?;
        Ok(self.flag("kingdom", true))
    }
    fn variable_value(&mut self, slot: Slot) -> Result<Vec<u16>> {
        self.step("variable_value")?;
        Ok(match slot {
            Slot::Kingdom => self.input["kingdom_text"].as_str().unwrap_or("ch_gem"),
            Slot::Observer => "ch_observer",
        }
        .encode_utf16()
        .collect())
    }
    fn find(&mut self, ids: &[u16]) -> Result<Option<u32>> {
        let ids = &ids[..ids.iter().position(|&v| v == 0).unwrap_or(ids.len())];
        if ids.is_empty() {
            return Ok(None);
        }
        self.trace
            .push(format!("find:{}", String::from_utf16(ids).unwrap()));
        if self.input["cutoff"].as_str() == Some("find") {
            Err("find".into())
        } else {
            Ok(Some(1))
        }
    }
}
#[test]
fn native_full_selection_cache_order_and_required_prefixes_replay() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/creator-labels/native-player-glyph.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 156);
    for row in fixture["rows"].as_array().unwrap() {
        let input = &row["input"];
        let flags = input["flags"].as_u64().unwrap_or(0) as u32;
        let mut cache = Cache {
            flags,
            kingdom: (flags & 1 != 0).then_some(Slot::Kingdom),
            observer: (flags & 2 != 0).then_some(Slot::Observer),
        };
        let mut world = Probe {
            input,
            trace: vec![],
        };
        let value = run(&mut cache, &mut world);
        let (returned, error) = match value {
            Ok(v) => (Some(v.is_some()), None),
            Err(e) => (None, Some(e)),
        };
        assert_eq!(
            json!({"flags":cache.flags,"kingdom_pointer":cache.kingdom.is_some(),
            "observer_pointer":cache.observer.is_some(),"returned_glyph":returned,"error":error,"trace":world.trace}),
            row["output"],
            "{row}"
        );
    }
    for row in fixture["index_rows"].as_array().unwrap() {
        assert_eq!(
            row["input"].as_u64().unwrap() as u16,
            row["index_word"].as_u64().unwrap() as u16
        );
        assert_eq!(row["following_byte"], 0xa5);
    }
    for row in fixture["global_image"].as_array().unwrap() {
        assert_eq!(row["initial_word"], 0);
    }
}

struct AiProbe {
    kingdom: bool,
}
impl PlayerRead for AiProbe {
    type Player = ();
    fn ai_flag_0c(&mut self, _: &()) -> Result<bool> {
        Ok(true)
    }
    fn online_manager_present(&mut self) -> Result<bool> {
        panic!("native AI bypasses online owner")
    }
    fn peer_present_04(&mut self, _: &()) -> Result<bool> {
        panic!("native AI bypasses PeerLink")
    }
    fn online_peer_glyph(&mut self, _: &(), _: &Catalog<'_>, _: &Bank) -> Result<Option<Glyph>> {
        panic!("native AI bypasses online glyph")
    }
    fn session_state_e4(&mut self) -> Result<u32> {
        Ok(2)
    }
    fn kingdom_present_28(&mut self, _: &()) -> Result<bool> {
        Ok(self.kingdom)
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT actual original/v14 source files"]
fn actual_glyph_variables_and_registered_indices_use_verified_source_bank() {
    use crate::{
        combat_source_completion, combat_source_receipt::VerifiedStartupReceipt, settings::Settings,
    };
    use aa_rules::{actors::ActorCatalog, sources::SourceLimits};
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
    let mut glyphs =
        State::from_fresh_sources(&catalog, &mut files, NativeOverrides::ControlledNone).unwrap();
    assert_eq!(
        glyphs.kingdom.value,
        "ch_gem".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(
        glyphs.observer.value,
        "ch_observer".encode_utf16().collect::<Vec<_>>()
    );
    for (kingdom, index, ids) in [(true, 132, "ch_gem"), (false, 140, "ch_observer")] {
        //This is an explicit laboratory AI/+28 loan for the source/selection
        //test, not evidence that BeginPlaying/765E10 has run on a real player.
        let glyph = glyphs
            .glyph(&catalog, &bank, &mut AiProbe { kingdom }, &())
            .unwrap()
            .unwrap();
        assert_eq!(glyph.index(), index);
        assert_eq!(glyph.ids(), ids);
        assert!(glyph.source().path.contains("special_glyphs_AA.tgi"));
    }
    let changed = ActorCatalog::from_files(vec![], Limits::default());
    assert!(
        glyphs
            .glyph(
                &Catalog::new(&changed),
                &bank,
                &mut AiProbe { kingdom: true },
                &()
            )
            .is_err()
    );
    let report = json!({"variables":{"kingdom":glyphs.kingdom.assignments,"observer":glyphs.observer.assignments},
        "files":glyphs.source_files(),"cache_flags":glyphs.cache.flags,"source_pc":bank.combat_source().unwrap().source_precision()});
    let path =
        root.join("rust-rebuild/research/development10/creator-labels/actual-glyph-inputs.json");
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}
