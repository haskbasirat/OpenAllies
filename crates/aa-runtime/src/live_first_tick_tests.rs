use super::*;
use serde_json::{Value, json};

struct Laboratory {
    input: Value,
    events: Vec<Value>,
}
impl World for Laboratory {
    type Player = ();
    type Kingdom = ();
    type Nation = ();
    type Audio = ();
    fn local_player(&mut self) -> Result<Option<()>> {
        self.events.push(json!(["local_player"]));
        Ok(self.input["player"].as_bool().unwrap().then_some(()))
    }
    fn player_kingdom(&mut self, player: Option<&()>) -> Result<Option<()>> {
        self.events.push(json!(["kingdom"]));
        if !self.input["world"].as_bool().unwrap() {
            return Ok(None);
        }
        if player.is_none() {
            return Err("NullPlayerDereference".into());
        }
        Ok(self.input["kingdom"].as_bool().unwrap().then_some(()))
    }
    fn nation(&mut self, _: &()) -> Result<Option<()>> {
        self.events.push(json!(["nation"]));
        Ok(self.input["nation"].as_bool().unwrap().then_some(()))
    }
    fn nation_music(&mut self, _: &()) -> Result<Option<()>> {
        self.events.push(json!(["nation_music"]));
        Ok(self.input["nation_music"].as_bool().unwrap().then_some(()))
    }
    fn observer_music(&mut self) -> Result<Vec<u16>> {
        self.events.push(json!(["observer"]));
        if self.input["error"] == "observer" {
            return Err("RequiredObserverError".into());
        }
        Ok(self.input["observer"]
            .as_str()
            .unwrap()
            .encode_utf16()
            .collect())
    }
    fn find_audio(&mut self, id: &[u16]) -> Result<Option<()>> {
        let id = String::from_utf16(id).unwrap();
        self.events.push(json!(["lookup", id]));
        Ok((id == "audio_fixture").then_some(()))
    }
    fn unknown_audio(&mut self, _: &[u16]) -> String {
        self.events.push(json!(["fatal"]));
        "UnknownAudioFatal".into()
    }
    fn play(&mut self, _: ()) -> Result<()> {
        self.events.push(json!(["play"]));
        if self.input["error"] == "audio" {
            Err("RequiredAudioError".into())
        } else {
            Ok(())
        }
    }
}
#[test]
fn actual_original_v14_first_tick_selection_order_and_required_errors_all_pcs() {
    let f: Value = serde_json::from_slice(include_bytes!(
        "../../../research/development10/live-first-tick/native-first-tick.json"
    ))
    .unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 72);
    for row in rows {
        let mut w = Laboratory {
            input: row["input"].clone(),
            events: vec![],
        };
        assert_eq!(
            json!(first_tick(&mut w).err()),
            row["native"]["outcome"],
            "{row}"
        );
        assert_eq!(json!(w.events), row["native"]["events"], "{row}");
    }
}
fn provenance() -> Provenance {
    Provenance {
        path: "laboratory".into(),
        sha256: "not-source-proof".into(),
        span: aa_rules::tgi::Span { start: 0, end: 0 },
    }
}
fn text(value: &str) -> TypedValue {
    TypedValue::String {
        value: value.into(),
        source: provenance(),
        declaration_source: provenance(),
    }
}
fn audio() -> AudioDefinition {
    AudioDefinition {
        id: "fixture".into(),
        source: provenance(),
        sources: vec![provenance()],
        category: text("music"),
        files: clips(text("Music/fixture.mp3")).unwrap(),
        leadin: clips(text("")).unwrap(),
        fallout: clips(text("")).unwrap(),
    }
}
struct Output {
    result: Result<Playback>,
    calls: usize,
}
impl AudioOutput for Output {
    fn play_747630(&mut self, request: &MusicRequest) -> Result<Playback> {
        assert!(matches!(request.delivery, Delivery::Pending));
        self.calls += 1;
        self.result.clone()
    }
}
#[test]
fn request_storage_requires_the_actual_callback_and_retains_failure_prefix() {
    let mut requests = MusicRequests::new();
    for result in [
        Ok(Playback::Accepted),
        Ok(Playback::Declined),
        Err("required backend".into()),
    ] {
        let mut output = Output {
            result: result.clone(),
            calls: 0,
        };
        let observed = requests.play(
            audio(),
            Origin::Observer {
                source: provenance(),
            },
            &mut output,
        );
        assert_eq!(observed, result.clone().map(|_| ()));
        assert_eq!(output.calls, 1);
        let record = requests.pop_finished().unwrap();
        match result {
            Ok(p) => assert!(matches!(record.delivery,Delivery::Finished(q) if p==q)),
            Err(e) => assert!(matches!(record.delivery,Delivery::Failed(q) if e==q)),
        }
    }
    assert!(requests.entries().is_empty());
}

struct BoundNullLaboratory;
impl crate::load_world_globals::SessionPhase for BoundNullLaboratory {
    fn current_phase(&self) -> Result<u32> {
        Err("not used by first-tick selection".into())
    }
}
impl SessionRead for BoundNullLaboratory {
    type Player = ();
    fn local_player(&self) -> Result<Option<()>> {
        Ok(Some(()))
    }
    fn player_name_24(&self, _: &()) -> Result<&[u16]> {
        Err("FirstTick must not infer a Kingdom from player name".into())
    }
    fn player_kingdom_28(&self, _: &()) -> Result<Option<ActorId>> {
        Ok(None)
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT source files"]
fn actual_audio_and_observer_source_bindings_with_lab_null_player_binding() {
    use crate::{
        combat_source_completion, combat_source_receipt::VerifiedStartupReceipt,
        live_clocked::FirstTick,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
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
    let actors = std::sync::Arc::new(ActorCatalog::load_v14(&mut files).unwrap());
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(Settings::load_v14(&mut files, aa_rules::tgi::Limits::default()).unwrap());
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
    let input = Source::bind(
        &catalog,
        &bank,
        catalog.settings.as_ref().unwrap(),
        &mut files,
    )
    .unwrap();
    let foreign=std::sync::Arc::new(actors.as_ref().clone());
    let input=match input.into_shared(foreign) {
        Err((same,error))=>{assert!(error.contains("actual bound allocation"));same},
        Ok(_)=>panic!("equal source values must not replace the bound allocation"),
    };
    let input=match input.into_shared(std::sync::Arc::clone(&actors)) {
        Ok(input)=>input,Err((_,error))=>panic!("{error}"),
    };
    input.validate_bank(&bank).unwrap();
    let mut rows = vec![];
    for id in ["nation_usa", "nation_gbr", "nation_ger", "nation_rus", "nation_jpn"] {
        let entry = actors.native_data_find("Nation", id).unwrap();
        let node = actors.native_data_node(entry).unwrap();
        let field = node.field("game_music", input.precision).unwrap().unwrap();
        let music = input.audio(string(&field).unwrap()).unwrap().unwrap();
        assert_eq!(music.id(), format!("{}_ambient", id.strip_prefix("nation_").unwrap()));
        assert!(!music.files().paths.is_empty());
        rows.push(
            json!({"nation":id,"nation_source":entry.source,"music_field":field,"audio":music}),
        );
    }
    let mut requests = MusicRequests::new();
    let mut output = Output {
        result: Ok(Playback::Accepted),
        calls: 0,
    };
    let mut context = Context {
        session: &BoundNullLaboratory,
        source: &input,
        bank: &bank,
        settings: catalog.settings.as_ref().unwrap(),
        requests: &mut requests,
        output: &mut output,
    };
    context.run(&ActorStore::default()).unwrap();
    assert_eq!(output.calls, 1);
    assert_eq!(requests.entries().len(), 1);
    let record = &requests.entries()[0];
    assert_eq!(record.audio.id(), "menu_background_music");
    assert_eq!(record.audio.files().paths.len(), 6);
    // This is a laboratory callback, not an audible/native-session certificate.
    assert!(matches!(
        record.delivery,
        Delivery::Finished(Playback::Accepted)
    ));
    let actual_row = observer_row(catalog.settings.as_ref().unwrap())
        .unwrap()
        .clone();
    catalog
        .settings
        .as_mut()
        .unwrap()
        .app_variables
        .get_mut("observergamemusic")
        .unwrap()[0]
        .raw = "forged".into();
    assert!(
        Source::bind(
            &catalog,
            &bank,
            catalog.settings.as_ref().unwrap(),
            &mut files
        )
        .is_err()
    );
    let mut context = Context {
        session: &BoundNullLaboratory,
        source: &input,
        bank: &bank,
        settings: catalog.settings.as_ref().unwrap(),
        requests: &mut requests,
        output: &mut output,
    };
    assert!(
        context
            .run(&ActorStore::default())
            .unwrap_err()
            .contains("setter/override")
    );
    assert_eq!(output.calls, 1);
    assert_eq!(requests.entries().len(), 1);
    catalog
        .settings
        .as_mut()
        .unwrap()
        .app_variables
        .insert("observergamemusic".into(), vec![actual_row]);
    if let Some(path) = std::env::var_os("AA_FIRST_TICK_SOURCE_REPORT") {
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"nation_inputs":rows,"observer_request":requests.entries(),
            "scope":"real source inputs; laboratory bound-null SessionRead and output callback; no live session/audio device/playback claim"})).unwrap()).unwrap();
    }
}
