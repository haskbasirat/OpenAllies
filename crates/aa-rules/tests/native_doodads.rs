use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
#[test]
fn original_doodad_component_stores_mask_and_last_slot_selection() {
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-doodads.json"
    ))
    .unwrap();
    for row in proof["cases"].as_array().unwrap() {
        let mut src = String::from(
            "[Template Base]{[list Component]{}}\n[Thing template=Base]{\nIDS=test\nstring category=DYNAMIC_DOODAD\n",
        );
        for (i, kind) in row["types"].as_array().unwrap().iter().enumerate() {
            let kind = kind.as_u64().unwrap();
            src.push_str(&format!(
                "[Component]{{\nstring type={}\nfixed update_time={}\n",
                if kind == 0 {
                    "DISTURBABLE"
                } else {
                    "BREAKABLE"
                },
                0.1 + i as f64
            ));
            if kind == 0 {
                src.push_str(&format!("fixed affect_radius={}\nfixed recharge_time_min={}\nfixed recharge_time_max={}\n",2+i,3+i,5+i));
            } else {
                src.push_str(&format!("fixed break_impulse={}\nbool toppled=true\nfixed topple_duration=.7\nfixed bounce_duration=.4\nfixed bounce_percentage=.25\nbool regrow=true\nfixed regrow_dead_duration=8\nfixed regrow_sink_duration=1.5\nfixed regrow_grow_duration=2\nstring trampled_aspect_ids=NULL\n",11+i));
            }
            src.push_str("}\n");
        }
        src.push_str("}\n");
        let cat = ActorCatalog::from_files(
            vec![ResolvedFile {
                logical_path: "doodad.tgi".into(),
                origin: Origin::V14Loose,
                sha256: sha256(src.as_bytes()),
                bytes: src.as_bytes().to_vec(),
            }],
            Limits::default(),
        );
        let out = cat.native_dynamic_doodad("test", Precision::Double53);
        if row["types"].as_array().unwrap().is_empty() {
            assert!(out.is_err());
            continue;
        }
        let out = out.unwrap();
        assert_eq!(
            u64::from(out.component_mask),
            row["output"]["mask"].as_u64().unwrap()
        );
        for (slot, component) in out.components.iter().enumerate() {
            let expected = &row["output"]["components"][slot];
            let Some(component) = component else {
                assert!(expected.is_null());
                continue;
            };
            let names: &[(u32, &str)] = if slot == 0 {
                &[
                    (0x10, "update_time"),
                    (0x14, "affect_radius"),
                    (0x18, "recharge_time_min"),
                    (0x1c, "recharge_time_max"),
                ]
            } else {
                &[
                    (0x10, "update_time"),
                    (0x14, "break_impulse"),
                    (0x1c, "topple_duration"),
                    (0x20, "bounce_duration"),
                    (0x24, "bounce_percentage"),
                    (0x34, "regrow_dead_duration"),
                    (0x38, "regrow_sink_duration"),
                    (0x3c, "regrow_grow_duration"),
                ]
            };
            for (offset, name) in names {
                assert_eq!(
                    u64::from(component.fields[*name].fixed_bits().unwrap()),
                    expected[format!("0x{offset:x}")].as_u64().unwrap()
                );
            }
            if slot == 1 {
                assert!(component.fields["toppled"].boolean().unwrap());
                assert!(component.fields["regrow"].boolean().unwrap());
            }
        }
    }
}
#[test]
#[ignore = "requires original/v14 files via AA_GAME_ROOT"]
fn original_dynamic_doodad_sources_preserve_separate_component_slots() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut checked = 0;
    let mut errors = Vec::new();
    for actor in &catalog.actors {
        if catalog
            .native_category(&actor.id)
            .is_ok_and(|c| c.native_kind == 21)
        {
            match catalog.native_dynamic_doodad(&actor.id, Precision::Double53) {
                Ok(def) => {
                    assert!(def.component_mask > 0 && def.component_mask < 4);
                    assert_eq!(
                        catalog
                            .native_final_flags(&actor.id, Precision::Double53)
                            .unwrap(),
                        catalog
                            .native_base_flags(&actor.id, false, Precision::Double53)
                            .unwrap()
                            .0
                    );
                    checked += 1;
                }
                Err(e) => errors.push((actor.id.clone(), e.to_string())),
            }
        }
    }
    println!("{}", serde_json::json!({"checked":checked,"errors":errors}));
    assert!(checked > 0);
    assert!(errors.is_empty());
}
