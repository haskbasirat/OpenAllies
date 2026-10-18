use super::*;
use serde_json::Value;

#[test]
fn native_current_fixed_cells_retain_reset_identity_and_raw_mutations() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-performer-value/native-current-combat-vars-v2.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 27);
    for row in rows {
        let input = &row["input"];
        let output = &row["output"];
        let words: [u32; 10] = std::array::from_fn(|i| input["source"][i].as_u64().unwrap() as u32);
        //Explicit synthetic source words match the native fixture inputs. The
        //separate actual-source test exercises the real private constructor.
        let mut store = crate::canonical_svars::Store::constructors();
        for (i, field) in FIELDS.iter().copied().enumerate() {
            if store.index(field.name()).is_none() {
                store
                    .add(
                        field.name().into(),
                        crate::canonical_svars::Value::Fixed(words[i]),
                        0,
                        [0xff7fffff, 0x7f7fffff],
                    )
                    .unwrap()
            } else {
                store.assign_word_raw(field.name(), words[i]);
            }
        }
        for (i, field) in FIELDS.iter().copied().enumerate() {
            assert_eq!(fixture["names"][i], field.name());
            assert_eq!(output["constructed"][i]["kind"], 2);
            assert_eq!(output["constructed"][i]["flags"], 0);
            assert_eq!(
                Values::borrow(&store).bits(field),
                output["reset"][i].as_u64().unwrap() as u32
            );
            store.assign_word_raw(field.name(), input["later"][i].as_u64().unwrap() as u32);
            assert_eq!(
                Values::borrow(&store).bits(field),
                output["current"][i].as_u64().unwrap() as u32
            );
        }
        assert_eq!(
            Values::borrow(&store).formula_settings().range_power.0,
            Values::borrow(&store).bits(Field::RangePower)
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT actual original sources/three maps and embedded reference"]
fn actual_current_combat_cells_move_with_state_and_survive_rule_publication() {
    use crate::{
        combat_source_receipt::VerifiedStartupReceipt,
        load_view_variables::{NativeOverrides, State},
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut sources, Limits::default()).unwrap());
    let proof = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut sources,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    catalog.native_load_mode = Some(1);
    let binding = combat_value_source_context::bind_numeric(&catalog, proof.context()).unwrap();
    let state = State::reset_from_fresh_source(&catalog, NativeOverrides::ControlledNone).unwrap();
    for field in FIELDS {
        let expected = if field == Field::ModifierClampAdd {
            clamp_source(&catalog, proof.context().precision()).unwrap()
        } else {
            binding.value(field.name()).unwrap().bits
        };
        assert_eq!(state.combat_values().bits(field), expected);
    }
    //Move the same current State; setters below change only its actual cells,
    //never the completed Bank's retained source-phase numeric snapshot.
    let mut state = Box::new(state);
    let address = &*state as *const State;
    for (i, field) in FIELDS.iter().copied().enumerate() {
        state.assign_combat_bits(field, ((i + 1) as f32 * 3.0).to_bits())
    }
    let live_words = FIELDS.map(|field| state.combat_values().bits(field));
    assert_ne!(
        live_words[Field::RangedMinimum as usize],
        binding.value(Field::RangedMinimum.name()).unwrap().bits
    );
    for name in [
        "Campaign/tutorial02.RMP",
        "DeveloperMaps/(3v3)Firefight.RMP",
        "(2v2)Berlin.RMP",
    ] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(
                game.join("reconstruction/assets/original/Data/Maps")
                    .join(name),
            )
            .unwrap(),
        )
        .unwrap();
        let inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
        let rules = crate::load_world_rules::Owner::load(&catalog, &inventory).unwrap();
        state.publish_loaded_rules(&catalog, &rules).unwrap();
        assert_eq!(
            state.fog_integer(),
            (rules.record("FogOfWar").unwrap().value() as i32).clamp(0, 2)
        );
        assert_eq!(
            state.stockpile_default_bits(),
            rules.record("ResourceStockpileDefault").unwrap().value()
        );
        assert_eq!(
            state.city_victory_ratio(),
            rules.record("CityVictoryRatio").unwrap().value() as i32
        );
        assert_eq!(&*state as *const State, address);
        assert_eq!(
            FIELDS.map(|field| state.combat_values().bits(field)),
            live_words,
            "{name}"
        );
    }
    state.assign_combat_bits(Field::RangedMinimum, 0x7FC01234);
    assert_eq!(state.combat_values().bits(Field::RangedMinimum), 0x7FC01234);
    state.validate_catalog(&catalog).unwrap(); //validation never silently resets.
    assert_eq!(state.combat_values().bits(Field::RangedMinimum), 0x7FC01234);
    let row = &mut catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .get_mut("modifierattributeclampadd")
        .unwrap()[0];
    let saved = std::mem::replace(&mut row.raw, "99".into());
    assert!(State::reset_from_fresh_source(&catalog, NativeOverrides::ControlledNone).is_err());
    assert_eq!(state.combat_values().bits(Field::RangedMinimum), 0x7FC01234);
    catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .get_mut("modifierattributeclampadd")
        .unwrap()[0]
        .raw = saved;
}
