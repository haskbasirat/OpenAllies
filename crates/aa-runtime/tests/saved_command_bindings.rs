//! Runtime binding of the original saved-command fixtures. The wire oracle
//! proves decoding; these tests cover typed source identities and raw storage.
use aa_mind::saved_command::{self, Command};
use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    native::actor_command::PREDEFINED_ACTOR_COMMANDS,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use aa_runtime::{catalog::Catalog, saved_commands::bind};
use serde_json::{Value, json};
use std::path::PathBuf;

fn fixture(path: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../research")
                .join(path),
        )
        .unwrap(),
    )
    .unwrap()
}
fn file(path: &str, text: &str) -> ResolvedFile {
    let bytes: Vec<_> = text
        .chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap())
        .collect();
    ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(&bytes),
        bytes,
    }
}
fn load(files: Vec<ResolvedFile>) -> ActorCatalog {
    load_files(files, Limits::default(), &mut NativeLoadReport::default())
}
fn utf16(s: &str) -> Option<Vec<u16>> {
    Some(s.encode_utf16().collect())
}
fn command() -> Command {
    Command {
        version: 3,
        definition: None,
        flags: 0,
        word10: 0xdead_beef,
        object_definition: None,
        position_bits: [0x8000_0000, 0x7fc0_1234],
        auxiliary_bits: [0xffff_ffff, 0x7f80_0000],
        scalar_bits: 0xff80_0000,
        parameter_definition: None,
        extra_definition: None,
        boolean: 0xfe,
        trailing: 0xff,
    }
}
fn key(c: &Catalog<'_>, id: u32) -> (String, String) {
    let bank = c.definition_bank.borrow();
    let k = &bank.entry(id).unwrap().key;
    (k.domain.clone(), k.name.clone())
}
fn source_catalog() -> ActorCatalog {
    source_catalog_with(None)
}
fn source_catalog_with(extra: Option<&str>) -> ActorCatalog {
    let doc = fixture("phase8-layouts/native-layout-data.json");
    let basic = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "basic")
        .unwrap();
    let mut files: Vec<_> = doc["templates"]
        .as_array()
        .unwrap()
        .iter()
        .chain(basic["files"].as_array().unwrap())
        .map(|f| file(f["path"].as_str().unwrap(), f["text"].as_str().unwrap()))
        .collect();
    files.push(file(
        "Fixture/template_translate.tgi",
        "[template Translate]\n{ string node\n string ids_old\n string ids_new\n}\n",
    ));
    files.push(file("Fixture/definitions.tgi",concat!(
        "[Thing]\n{ string IDS = usa_infantry\n string category = ACTOR\n}\n",
        "[Thing]\n{ string IDS = alpha\n string category = ACTOR\n}\n",
        "[Thing]\n{ string IDS = beta\n string category = OBJECT\n}\n",
        "[ActorCommand]\n{ string IDS = move\n}\n[ActorCommand]\n{ string IDS = stop\n}\n",
        "[Formation template=Formation]\n{ IDS = shared\n name = \"Shared\"\n organized = false\n aggressive = false\n icon = null\n}\n",
        "[Property template=SharedProperty]\n{ IDS = shared\n name = \"Shared\"\n icon = null\n}\n",
        "[Translate template=Translate]\n{ node = ActorCommand\n ids_old = old_move\n ids_new = move\n}\n",
        "[Translate template=Translate]\n{ node = ActorCommand\n ids_old = move\n ids_new = stop\n}\n",
        "[Translate template=Translate]\n{ node = Thing\n ids_old = old_actor\n ids_new = alpha\n}\n",
        "[Translate template=Translate]\n{ node = Thing\n ids_old = alpha\n ids_new = beta\n}\n",
        "[Translate template=Translate]\n{ node = Formation\n ids_old = formation\n ids_new = f_org\n}\n",
        "[Translate template=Translate]\n{ node = Formation\n ids_old = f_org\n ids_new = f_mob\n}\n",
        "[Translate template=Translate]\n{ node = Property\n ids_old = property\n ids_new = front\n}\n",
        "[Translate template=Translate]\n{ node = Property\n ids_old = front\n ids_new = support\n}\n",
    )));
    if let Some(extra) = extra {
        files.push(file("Fixture/last_translation.tgi", extra));
    }
    load(files)
}

#[test]
fn translated_missing_targets_never_fall_back_and_owned_parameters_are_refused() {
    for (domain, id, field) in [
        ("ActorCommand", "stop", 0),
        ("Thing", "usa_infantry", 1),
        ("Formation", "shared", 2),
        ("Property", "shared", 3),
    ] {
        let text = format!(
            "[Translate template=Translate]\n{{ node = {domain}\n ids_old = {id}\n ids_new = missing_target\n}}\n"
        );
        let actors = source_catalog_with(Some(&text));
        let catalog = Catalog::new(&actors);
        let mut saved = command();
        match field {
            0 => saved.definition = utf16(id),
            1 => saved.object_definition = utf16(id),
            2 => saved.parameter_definition = utf16(id),
            _ => saved.extra_definition = utf16(id),
        }
        assert!(
            bind(&catalog, &saved, Precision::Double53).is_err(),
            "{domain} used fallback for translated {id}"
        );
    }
    let actors = load(vec![]);
    let catalog = Catalog::new(&actors);
    let mut saved = command();
    saved.flags = 1 << 5;
    assert!(
        bind(&catalog, &saved, Precision::Double53)
            .unwrap_err()
            .contains("ParameterSet")
    );
}

#[test]
fn compiled_commands_need_no_authored_ui_roots_and_keep_dispatch_words() {
    let actors = load(vec![]);
    let catalog = Catalog::new(&actors);
    for d in PREDEFINED_ACTOR_COMMANDS {
        let mut saved = command();
        saved.definition = utf16(&d.id.to_ascii_uppercase());
        let bound = bind(&catalog, &saved, Precision::Double53).unwrap();
        assert_eq!(
            key(&catalog, bound.definition_id),
            ("actorcommand".into(), d.id.into())
        );
        assert_eq!(
            bound.definition,
            Some(aa_mind::command::Definition {
                component_slot: d.component_slot,
                mind_type: d.mind_type,
                behavior_flags: d.behavior_flags
            })
        );
    }
}

#[test]
fn references_translate_once_and_keep_definition_domains_distinct() {
    let actors = source_catalog();
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    let mut saved = command();
    saved.definition = utf16("old_move");
    saved.object_definition = utf16("old_actor");
    saved.parameter_definition = utf16("formation");
    saved.extra_definition = utf16("property");
    let bound = bind(&catalog, &saved, pc).unwrap();
    assert_eq!(bound.definition.unwrap().mind_type, 5);
    assert_eq!(
        key(&catalog, bound.definition_id),
        ("actorcommand".into(), "move".into())
    );
    assert_eq!(
        key(&catalog, bound.object_definition.unwrap()),
        ("actordefinition".into(), "alpha".into())
    );
    assert_eq!(
        key(&catalog, bound.parameter_definition.unwrap()),
        ("formation".into(), "f_org".into())
    );
    assert_eq!(
        key(&catalog, bound.extra_definition.unwrap()),
        ("property".into(), "front".into())
    );
    saved.definition = utf16("move");
    saved.object_definition = None;
    saved.parameter_definition = utf16("f_org");
    saved.extra_definition = utf16("front");
    let bound = bind(&catalog, &saved, pc).unwrap();
    assert_eq!(bound.definition.unwrap().mind_type, 6);
    assert_eq!(
        key(&catalog, bound.parameter_definition.unwrap()).1,
        "f_mob"
    );
    assert_eq!(key(&catalog, bound.extra_definition.unwrap()).1, "support");
    saved.parameter_definition = utf16("shared");
    saved.extra_definition = utf16("shared");
    let bound = bind(&catalog, &saved, pc).unwrap();
    assert_ne!(bound.parameter_definition, bound.extra_definition);
    assert_eq!(
        key(&catalog, bound.parameter_definition.unwrap()),
        ("formation".into(), "shared".into())
    );
    assert_eq!(
        key(&catalog, bound.extra_definition.unwrap()),
        ("property".into(), "shared".into())
    );
}

#[test]
fn nul_and_utf16_policy_is_explicit_and_actor_types_are_checked() {
    let actors = source_catalog();
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    let mut saved = command();
    saved.definition = utf16("old_move\0ignored");
    saved.object_definition = utf16("old_actor\0ignored");
    saved.parameter_definition = utf16("formation\0ignored");
    saved.extra_definition = utf16("property\0ignored");
    let bound = bind(&catalog, &saved, pc).unwrap();
    assert_eq!(bound.definition.unwrap().mind_type, 5);
    for bad in [
        vec![0],
        vec![0, 109],
        vec![0xd800],
        "missing".encode_utf16().collect(),
    ] {
        for field in 0..4 {
            let mut invalid = command();
            match field {
                0 => invalid.definition = Some(bad.clone()),
                1 => invalid.object_definition = Some(bad.clone()),
                2 => invalid.parameter_definition = Some(bad.clone()),
                _ => invalid.extra_definition = Some(bad.clone()),
            }
            assert!(
                bind(&catalog, &invalid, pc).is_err(),
                "field{field} accepted {bad:?}"
            );
        }
    }
    let mut invalid = command();
    invalid.object_definition = utf16("alpha");
    assert!(
        bind(&catalog, &invalid, pc)
            .unwrap_err()
            .contains("not ActorData")
    );
    // UTF16 after the first NUL is not part of the native lookup key.
    saved = command();
    saved.definition = Some(vec![115, 116, 111, 112, 0, 0xd800]);
    assert_eq!(
        bind(&catalog, &saved, pc)
            .unwrap()
            .definition
            .unwrap()
            .mind_type,
        6
    );
}

#[test]
fn native_saved_command_fixture_bits_and_nulls_survive_binding() {
    let actors = source_catalog();
    let catalog = Catalog::new(&actors);
    let doc = fixture("resume-20261016-mind-commands/native-commands.json");
    let mut checked = 0;
    for row in doc["cases"].as_array().unwrap() {
        if row["kind"] != "command" || row["native"]["outcome"] != "loaded" {
            continue;
        }
        let hex = row["payload"].as_str().unwrap();
        let bytes: Vec<_> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let (saved, used) = saved_command::decode(&bytes).unwrap();
        assert_eq!(used as u64, row["native"]["consumed"]);
        let bound = bind(&catalog, &saved, Precision::Double53).unwrap();
        let native = &row["native"]["fields"];
        let raw = json!({"flags":bound.flags,"word10":bound.word10,"position_bits":bound.position_bits,
            "auxiliary_bits":bound.auxiliary_bits,"scalar_bits":bound.scalar_bits,"boolean":bound.boolean,"trailing":bound.trailing});
        for (k, v) in raw.as_object().unwrap() {
            assert_eq!(v, &native[k], "{k}");
        }
        if saved.definition.is_none() {
            assert_eq!(bound.definition, None);
            assert_eq!(bound.definition_id, 0);
        }
        assert_eq!(
            bound.object_definition.is_some(),
            saved.object_definition.is_some()
        );
        assert_eq!(
            bound.parameter_definition.is_some(),
            saved.parameter_definition.is_some()
        );
        assert_eq!(
            bound.extra_definition.is_some(),
            saved.extra_definition.is_some()
        );
        checked += 1;
    }
    assert!(checked >= 100, "native fixture coverage {checked}");
    let null = command();
    let bound = bind(&catalog, &null, Precision::Double53).unwrap();
    assert_eq!(bound.definition_id, 0);
    assert!(bound.definition.is_none());
    assert_eq!(bound.position_bits, null.position_bits);
    assert_eq!(bound.auxiliary_bits, null.auxiliary_bits);
    assert_eq!(bound.scalar_bits, null.scalar_bits);
    assert_eq!(bound.boolean, 0xfe);
    assert_eq!(bound.trailing, 0xff);
}

#[test]
#[ignore = "requires original Data.rwd and v14 files via AA_GAME_ROOT"]
fn real_v14_actor_formation_and_property_sources_bind() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    let data = catalog.layout_data(pc).unwrap();
    let formation = &data.formations.first().unwrap().ids;
    let property = &data.properties.first().unwrap().ids;
    let mut saved = command();
    saved.definition = utf16("MOVE");
    saved.object_definition = utf16("usa_infantry");
    saved.parameter_definition = utf16(formation);
    saved.extra_definition = utf16(property);
    let bound = bind(&catalog, &saved, pc).unwrap();
    assert_eq!(bound.definition.unwrap().mind_type, 5);
    assert_eq!(
        key(&catalog, bound.object_definition.unwrap()),
        ("actordefinition".into(), "usa_infantry".into())
    );
    assert_eq!(
        key(&catalog, bound.parameter_definition.unwrap()),
        ("formation".into(), formation.to_ascii_lowercase())
    );
    assert_eq!(
        key(&catalog, bound.extra_definition.unwrap()),
        ("property".into(), property.to_ascii_lowercase())
    );
}
