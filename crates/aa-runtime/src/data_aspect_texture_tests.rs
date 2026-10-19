use super::super::rename::{TextureRename, TextureRenames};
use super::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/mapload-20261018/aspect-extra-textures/native-aspect-extra-textures.json"
    ))
    .unwrap();
    assert_eq!(
        doc["original_sha256"],
        "5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26"
    );
    assert_eq!(
        doc["v14_sha256"],
        "3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1"
    );
    assert_eq!(doc["precisions"].as_array().unwrap().len(), 3);
    assert_eq!(doc["images"].as_array().unwrap().len(), 2);
    doc
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn text(v: &[u16]) -> String {
    String::from_utf16(v).unwrap()
}

#[test]
fn native_5a79f0_search_order_and_final_lookup_match() {
    let doc = fixture();
    let rows = doc["search"].as_array().unwrap();
    assert_eq!(rows.len(), 13);
    let mut refused = 0;
    for row in rows {
        let (input, native) = (&row["input"], &row["native"]);
        let hits: Vec<&str> = input["hits"].as_array().unwrap().iter().map(|h| h.as_str().unwrap()).collect();
        let paths: Option<Vec<String>> = input["paths"]
            .as_array()
            .map(|a| a.iter().map(|p| p.as_str().unwrap().to_owned()).collect());
        let mut calls = vec![];
        let result = request_5a79f0(&wide(input["name"].as_str().unwrap()), paths.as_deref(), |name, flags| {
            calls.push(json!(["5a77e0", text(name), flags]));
            Ok(if flags == 0 {
                Some(input["final"].as_u64().unwrap())
            } else {
                hits.iter()
                    .position(|h| *h == text(name))
                    .map(|i| 0x7e100000 + 0x10 * i as u64)
            })
        });
        let expected = native["calls"].as_array().unwrap();
        match expected.iter().position(|c| c[0] == "85a030") {
            //The native continues through 85A030; Rust stops before it.
            Some(stop) => {
                assert!(result.is_err(), "{input}");
                assert_eq!(calls, expected[..stop].to_vec(), "{input}");
                refused += 1;
            }
            None => {
                assert_eq!(calls, *expected, "{input}");
                assert_eq!(json!(result.unwrap()), native["result"], "{input}");
            }
        }
    }
    assert_eq!(refused, 1);
}

struct Fake<'a> {
    models: &'a Value,
    model: String,
    folder: String,
    events: Vec<Value>,
    textures: usize,
}
impl ExtraServices for Fake<'_> {
    type Texture = String;
    fn model_texture_count(&mut self) -> Result<i32> {
        Ok(self.models[&self.model].as_array().unwrap().len() as i32)
    }
    fn model_texture(&mut self, index: usize) -> Result<(Vec<u16>, Vec<u16>)> {
        let pair = &self.models[&self.model][index];
        Ok((wide(pair[0].as_str().unwrap()), wide(pair[1].as_str().unwrap())))
    }
    fn folder(&mut self) -> Result<String> {
        Ok(self.folder.clone())
    }
    fn request_5a79f0(&mut self, name: &[u16]) -> Result<Option<String>> {
        //The oracle seam owner check: Aspect+44 is the ObjectData+B4 list.
        self.events.push(json!(["5a79f0", text(name), true]));
        let t = format!("tex{}", self.textures);
        self.textures += 1;
        Ok(Some(t))
    }
    fn request_10(&mut self, texture: &String, delta: u32, mode: u32) -> Result<()> {
        self.events.push(json!(["vf10", texture, delta, mode]));
        Ok(())
    }
}
fn models() -> Value {
    let dir = "Buildings/GBR/GBRBrigadeEngineer/";
    let pair = |full: String, raw: &str| json!([full, raw]);
    json!({
        "gbr": [pair(format!("{dir}GBRBrigadeEngineerBase.dds"), "GBRBrigadeEngineerBase.dds"),
                pair(format!("{dir}GBRBrigadeEngineerGround.dds"), "GBRBrigadeEngineerGround.dds")],
        "one": [pair("Units/X/one.dds".into(), "one.dds")],
        "three": [pair("Units/X/a.dds".into(), "a.dds"), pair("Units/X/b.dds".into(), "b.dds"),
                  pair("Units/X/c.dds".into(), "c.dds")],
        "five": (0..5).map(|i| pair(format!("Units/X/t{i}.dds"), &format!("t{i}.dds"))).collect::<Vec<_>>(),
        "nine": (0..9).map(|i| pair(format!("U/n{i}.dds"), &format!("n{i}.dds"))).collect::<Vec<_>>(),
        "none": [],
        "rename": [pair("Units/X/DeathArcticBase.dds".into(), "DeathArcticBase.dds"),
                   pair("Units/X/Base.dds".into(), "Base.dds")],
        "absolute": [pair("/Shared/abs.dds".into(), "abs.dds")],
        "empty": [pair("Units/X/a.dds".into(), "a.dds"), pair(String::new(), "")],
    })
}
fn input(renames_head_first: &Value) -> Input {
    let mut children: Vec<TextureRename> = renames_head_first
        .as_array()
        .unwrap()
        .iter()
        .map(|r| TextureRename {
            from: wide(r[0].as_str().unwrap()),
            to: wide(r[1].as_str().unwrap()),
        })
        .collect();
    children.reverse();
    Input {
        ids: "aspect_normal".into(),
        sources: vec![],
        model: vec![],
        visual_radius: 0,
        skin_paths: [None, None],
        texture_paths: vec![],
        resource_prefix: "GBRBrigadeEngineer/GBRBrigadeEngineer".into(),
        aspect_prefix: String::new(),
        renames: TextureRenames::from_source_order(children),
        folder: "Buildings/GBR".into(),
    }
}

#[test]
fn native_46ae20_vector_names_and_vf10_sequences_match() {
    let doc = fixture();
    let sequences = doc["extra"].as_array().unwrap();
    assert_eq!(sequences.len(), 9);
    let models = models();
    let mut faults = 0;
    for seq in sequences {
        let (inp, native) = (&seq["input"], seq["native"].as_array().unwrap());
        let input = input(&inp["renames"]);
        let mut fake = Fake {
            models: &models,
            model: String::new(),
            folder: inp["folder"].as_str().unwrap().into(),
            events: vec![],
            textures: 0,
        };
        let mut extras = Extras::<String>::constructed();
        let (mut count, mut requested) = (0u32, false);
        for step in native {
            let s = &step["step"];
            if s["reset"] == true {
                count = 0;
                requested = false;
            }
            fake.model = s["model"].as_str().unwrap().into();
            fake.events.clear();
            let result = extra_46ae20(
                &mut extras,
                &mut count,
                &mut requested,
                &input,
                &mut fake,
                s["delta"].as_u64().unwrap() as u32,
                s["mode"].as_u64().unwrap() as u32,
            );
            //The seam renders format calls; Rust's 4763C0 is the proven helper.
            let expected: Vec<Value> = step["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e[0] != "format_405d60")
                .cloned()
                .collect();
            assert_eq!(fake.events, expected, "{}", inp["label"]);
            if step["outcome"] == "null_texture_vf10" {
                assert!(result.unwrap_err().contains("NULL"));
                faults += 1;
                continue;
            }
            result.unwrap();
            assert_eq!(count, step["count"], "{}", inp["label"]);
            assert_eq!(json!(extras.capacity), step["capacity"]);
            assert_eq!(json!(u8::from(requested)), step["requested"]);
            assert_eq!(extras.data.is_some(), step["data"] == true);
            let elements: Vec<Value> = extras
                .data
                .iter()
                .flatten()
                .map(|e| {
                    let kind = match &e.texture {
                        Slot::Texture(t) => t.clone(),
                        Slot::Null => "null".into(),
                        Slot::Unwritten => "unwritten".into(),
                    };
                    json!([kind, text(&e.name)])
                })
                .collect();
            assert_eq!(Value::Array(elements), step["elements"], "{}", inp["label"]);
        }
    }
    assert_eq!(faults, 1);
}

struct Block<'a> {
    c: &'a Value,
    requested: bool,
    latch: bool,
    events: Vec<Value>,
}
impl BlockServices for Block<'_> {
    fn model_loaded(&mut self) -> Result<bool> {
        Ok(self.c["loaded"] == 1)
    }
    fn block_18(&mut self) -> Result<()> {
        self.events.push(json!(["vf18", true]));
        Ok(())
    }
    fn requested_54(&mut self) -> Result<bool> {
        Ok(self.requested)
    }
    fn count_18(&mut self) -> Result<u32> {
        Ok(self.c["count"].as_u64().unwrap() as u32)
    }
    fn extra_46ae20(&mut self, delta: u32, mode: u32) -> Result<()> {
        self.events.push(json!(["46ae20", true, delta, mode]));
        if self.c["sets"] == true {
            self.requested = true;
        }
        Ok(())
    }
    fn assertion_latched(&mut self) -> Result<bool> {
        Ok(self.latch)
    }
    fn assert_extra_requested(&mut self) -> Result<()> {
        self.events.push(json!(["855d30", "m_extra_textures_requested", 0x1c9]));
        Ok(())
    }
}

#[test]
fn native_46af70_block_request_and_assertion_order_match() {
    let doc = fixture();
    let cases = doc["block"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    for case in cases {
        let c = &case["input"];
        let mut w = Block {
            c,
            requested: c["requested"] == 1,
            latch: c["latch"] == 1,
            events: vec![],
        };
        block_46af70(&mut w).unwrap();
        assert_eq!(Value::Array(w.events), case["native"]["events"], "{c}");
        assert_eq!(json!(u8::from(w.requested)), case["native"]["requested"]);
    }
}

#[test]
fn path_split_matches_native_85a550_859b70_rows() {
    for (name, dir, file) in [
        ("Units/USA/usa_infantry.tgi", "Units/USA", "usa_infantry.tgi"),
        ("translate.tgi", "", "translate.tgi"),
        ("Units\\GBR\\test.tgi", "Units\\GBR", "test.tgi"),
        ("C:c.dds", "C:", "c.dds"),
    ] {
        let (d, f) = split_85a550_859b70(&wide(name)).unwrap();
        assert_eq!((text(&d), text(&f)), (dir.to_owned(), file.to_owned()));
    }
    assert!(split_85a550_859b70(&[97, 0, 98]).is_err());
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources and completed Bank; actual source Aspect, GameData+4 and Texture lookups, no Model decode"]
fn actual_gbr_brigade_engineer_extra_textures_resolve_through_5a79f0() {
    use crate::{
        load_view_variables::NativeOverrides, resource_depot::Resources, resource_objects::Handle,
    };
    use aa_mapworld::restore::RestoreDefinitions;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog, &mut files,
    )
    .unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    //48D970 ends the index chain with Data-root translate.tgi: GameData+4 "".
    assert_eq!(catalog.actors.sources.last().unwrap().path, "translate.tgi");
    assert_eq!(game_data_folder_4(&catalog).unwrap(), "");
    let pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let definition = catalog.resolve(&wide("gbr_brigade_engineer"), pc).unwrap();
    let mut application = crate::resource_depot::Application::from_sources(files);
    application
        .install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone)
        .unwrap();
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    struct Real<'a, 'b, 'm> {
        pairs: Vec<(Vec<u16>, Vec<u16>)>,
        catalog: &'a Catalog<'b>,
        bank: &'a crate::definitions::Bank,
        resources: &'a mut Resources<'m>,
        search: &'a [String],
        requested: Vec<(String, u32, u32)>,
    }
    impl ExtraServices for Real<'_, '_, '_> {
        type Texture = Handle;
        fn model_texture_count(&mut self) -> Result<i32> {
            Ok(self.pairs.len() as i32)
        }
        fn model_texture(&mut self, index: usize) -> Result<(Vec<u16>, Vec<u16>)> {
            Ok(self.pairs[index].clone())
        }
        fn folder(&mut self) -> Result<String> {
            game_data_folder_4(self.catalog)
        }
        fn request_5a79f0(&mut self, name: &[u16]) -> Result<Option<Handle>> {
            let Self {
                catalog,
                bank,
                resources,
                search,
                ..
            } = self;
            request_5a79f0(name, Some(*search), |n, f| {
                resources.texture_5a77e0_with_policy(
                    catalog,
                    bank,
                    n,
                    f,
                    &mut super::super::RequiredTexturePolicy,
                )
            })
        }
        fn request_10(&mut self, t: &Handle, delta: u32, mode: u32) -> Result<()> {
            self.requested.push((text(t.logical_ids()), delta, mode));
            Ok(())
        }
    }
    application
        .with_data_pre_request_resources(|state, resources| {
            state.prepare_object_aspects(&catalog, &bank, &definition, resources)?;
            let data = state.bind_object(&catalog, &bank, &definition)?;
            let row = &state.entry(&data)?.aspects.as_ref().unwrap().rows[0];
            let input = row.input.as_ref().unwrap();
            assert_eq!(
                text(&input.model),
                "Buildings/GBR/GBRBrigadeEngineer/GBRBrigadeEngineer.nif"
            );
            assert!(input.texture_paths.iter().all(|p| p == "Shared/SharedTextures"));
            //The shipped GBRBrigadeEngineer.nif names exactly these two
            //external textures; 5A8A40 pairs them with the Model directory.
            let pairs = ["GBRBrigadeEngineerBase.dds", "GBRBrigadeEngineerGround.dds"]
                .map(|n| crate::model_texture_path::pair(&input.model, &wide(n)).unwrap())
                .to_vec();
            let mut real = Real {
                pairs,
                catalog: &catalog,
                bank: &bank,
                resources,
                search: &input.texture_paths,
                requested: vec![],
            };
            let mut extras = Extras::constructed();
            let (mut count, mut requested) = (0u32, false);
            let read = extra_46ae20(&mut extras, &mut count, &mut requested, input, &mut real, 1, 0)?;
            assert!(read == 2 && requested && count == 2 && extras.capacity == 2);
            let dir = "Buildings/GBR/GBRBrigadeEngineer/";
            assert_eq!(
                real.requested,
                [
                    (format!("{dir}GBRBrigadeEngineerBase.dds"), 1, 0),
                    (format!("{dir}GBRBrigadeEngineerGround.dds"), 1, 0)
                ]
            );
            assert_eq!(text(&extras.element(1)?.name), "GBRBrigadeEngineerGround.dds");
            //A second request with the same count only repeats vf10.
            real.requested.clear();
            extra_46ae20(&mut extras, &mut count, &mut requested, input, &mut real, 2, 1)?;
            assert_eq!(real.requested.len(), 2);
            Ok(())
        })
        .unwrap();
}
