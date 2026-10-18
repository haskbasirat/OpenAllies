use aa_orders::{
    commands::CommandWire,
    payload::{Field, Registry},
};
use aa_runtime::player_order_decree::{self as decree, Decoded};
use serde_json::Value;
fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261016-player-orders/decree/native-decree.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn fields(c: &Value) -> Vec<Field> {
    c["output"]["reads"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let n = r[1].as_u64().unwrap() as u32;
            let value = r[2].as_u64().unwrap() as u32;
            if r[0] == "bounded" {
                if n == 0x800 {
                    Field::Bounded { n, value }
                } else {
                    assert_eq!(n, 40);
                    Field::Def {
                        registry: Registry(0xb3aa58),
                        index: value,
                    }
                }
            } else {
                Field::Uint {
                    width: n as u8,
                    value,
                }
            }
        })
        .collect()
}
#[test]
fn flat_decoded_fields_preserve_native_scalar_inputs_and_command_null_sentinel() {
    let mut checked = 0;
    for row in fixture()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "decoded")
    {
        let Decoded::Selected(wire) = decree::from_fields(86, &fields(row)).unwrap() else {
            panic!()
        };
        let native = &row["output"]["fields"];
        assert_eq!(u64::from(wire.flags), native["flags"]);
        assert_eq!(serde_json::json!(wire.command), native["command"]);
        assert_eq!(u64::from(wire.word), native["word"]);
        assert_eq!(serde_json::json!(wire.position), native["position"]);
        assert_eq!(serde_json::json!(wire.auxiliary), native["auxiliary"]);
        assert_eq!(u64::from(wire.scalar), native["scalar"]);
        assert_eq!(u64::from(wire.boolean), native["boolean"]);
        assert_eq!(u64::from(wire.trailing), native["trailing"]);
        checked += 1;
    }
    assert_eq!(checked, 1248);
    assert!(decree::from_fields(84, &[]).is_err());
    assert!(
        decree::from_fields(
            86,
            &[Field::Bounded {
                n: 0x800,
                value: 0x800
            }]
        )
        .is_err()
    );
    assert!(
        decree::from_fields(86, &[Field::Bounded { n: 0x800, value: 2 }])
            .unwrap_err_text()
            .contains("B39F58")
    );
}
// Keep the assertion's message extraction independent of Decoded implementing Debug.
trait ErrorText {
    fn unwrap_err_text(self) -> String;
}
impl ErrorText for aa_runtime::player_order_world::Result<Decoded> {
    fn unwrap_err_text(self) -> String {
        match self {
            Ok(_) => panic!("expected required service"),
            Err(e) => e.to_string(),
        }
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn native_decoded_commands_allocate_in_the_same_bank_and_command_arena() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        catalog::Catalog, combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt, mind_world, settings::Settings,
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
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    let receipt =
        VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    completion::install(
        &catalog,
        &mut sources,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    let mut runtime = mind_world::Runtime::default();
    let mut checked = 0;
    for row in fixture()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "decoded")
    {
        let Decoded::Selected(wire) = decree::from_fields(86, &fields(row)).unwrap() else {
            panic!()
        };
        let command = decree::allocate_decoded(&catalog, &mut runtime, &wire).unwrap();
        let plain = runtime.commands.value(command).unwrap();
        let native = &row["output"]["fields"];
        assert_eq!(u64::from(plain.flags), native["flags"]);
        assert_eq!(u64::from(plain.word10), native["word"]);
        assert_eq!(serde_json::json!(plain.position_bits), native["position"]);
        assert_eq!(serde_json::json!(plain.auxiliary_bits), native["auxiliary"]);
        assert_eq!(u64::from(plain.scalar_bits), native["scalar"]);
        assert_eq!(u64::from(plain.boolean), native["boolean"]);
        assert_eq!(u64::from(plain.trailing), native["trailing"]);
        assert!(
            plain.object_definition.is_none()
                && plain.parameter_definition.is_none()
                && plain.extra_definition.is_none()
        );
        if let Some(index) = wire.command {
            let d = aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS[index as usize];
            assert_eq!(plain.definition.unwrap().mind_type, d.mind_type);
            let bank = catalog.definition_bank.borrow();
            assert_eq!(
                bank.require_usable(plain.definition_id).unwrap().key.name,
                d.id
            );
            assert_eq!(bank.token("ActorCommand", d.id), Some(plain.definition_id));
        } else {
            assert_eq!(plain.definition_id, 0);
            assert!(plain.definition.is_none());
        }
        assert_eq!(runtime.commands.references(command).unwrap(), 1);
        runtime.commands.release(command).unwrap();
        assert!(runtime.commands.value(command).is_err());
        checked += 1;
    }
    assert_eq!(checked, 1248);
    let rejected = CommandWire {
        flags: 0x42,
        command: Some(5),
        object: Some(1),
        ..Default::default()
    };
    assert!(decree::allocate_decoded(&catalog, &mut runtime, &rejected).is_err());
}
