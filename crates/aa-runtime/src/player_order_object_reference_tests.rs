//! Replays native-object-reference.json (original 4B4E60 / 76A960 / 76A880).
use super::*;
use aa_rules::actors::native_load::NativeThingRegistry;

fn fixture() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/recruit/native-object-reference.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn player_order_object_reference_native_rows_replay() {
    let doc = fixture();
    // The 24 class vtables: vt+38 accepts exactly kinds 2 and 19.
    let kinds = doc["kind_vtables"].as_array().unwrap();
    assert_eq!(kinds.len(), 24);
    assert_eq!(kinds[2]["vtable"], "0xa05540");
    assert_eq!(kinds[19]["vtable"], "0xa09b30");
    let (mut registers, mut reads, mut writes, mut accepted) = (0, 0, 0, 0);
    for row in doc["rows"].as_array().unwrap() {
        let input = &row["input"];
        let out = &row["output"];
        match row["part"].as_str().unwrap() {
            "register" => {
                let mut reg = NativeThingRegistry::default();
                for (i, r) in out["registrations"].as_array().unwrap().iter().enumerate() {
                    let ids = input[i].as_str().unwrap();
                    assert_eq!(r["ids"], ids);
                    match reg.register(ids) {
                        Ok(slot) => assert_eq!(r["index_0c"].as_u64(), Some(slot as u64), "{row}"),
                        Err(_) => assert_eq!(r["thrown"], true, "{row}"),
                    }
                }
                let vector: Vec<_> = out["vector"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
                for (slot, ids) in vector.iter().enumerate() {
                    assert_eq!(reg.find(ids), Some(slot), "{row}");
                }
                registers += 1;
            }
            "read" => {
                let count = input["count"].as_u64().unwrap() as usize;
                let value = input["value"].as_u64().unwrap() as u32;
                let kinds = input["kinds"].as_array().unwrap();
                let mut asked = None;
                let result = reader_76a960::<String>(value, count, |v| {
                    asked = Some(v);
                    Ok(kinds[v as usize].as_u64().map(|k| k as u32))
                })
                .unwrap();
                assert_eq!(result.map(|v| v as u64), out["resolved"].as_u64(), "{row}");
                assert_eq!(asked.is_some(), (value as usize) < count, "{row}");
                // 411FA0 reads exactly the 40C7C0(count+1) width.
                let bits = aa_orders::registry_bits(count + 1).unwrap() as u64;
                assert_eq!(input["bits"].as_u64().unwrap(), bits);
                let end = input["bit"].as_u64().unwrap() + bits;
                assert_eq!(out["byte"].as_u64().unwrap() * 8 + out["bit"].as_u64().unwrap(), end, "{row}");
                // Values above the null sentinel are asserted (855D30) once.
                let asserted = out["events"].as_array().unwrap().iter().any(|e| e[0] == "assert");
                assert_eq!(asserted, (value as usize) > count, "{row}");
                accepted += usize::from(result.is_some());
                reads += 1;
            }
            "write" => {
                let count = input["count"].as_u64().unwrap() as usize;
                let index = input["index_0c"].as_u64().map(|v| v as u32);
                let (value, bits) = writer_76a880(count, index).unwrap();
                let e = &out["events"][0];
                assert_eq!(e[0], "write");
                assert_eq!(e[1].as_u64().unwrap(), u64::from(value), "{row}");
                assert_eq!(e[2].as_u64().unwrap(), u64::from(bits), "{row}");
                writes += 1;
            }
            p => panic!("{p}"),
        }
    }
    assert_eq!((registers, reads, writes), (36, 756, 150));
    assert!(accepted > 0);
}

/// Real v14 sources: the registry count is the native Thing registration
/// count and the Berlin recruit targets resolve as ActorData.
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn player_order_object_reference_v14_registry_resolves_rus_corps_recruits() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut files = aa_rules::sources::V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        aa_rules::sources::SourceLimits { max_member_bytes: u64::MAX },
    )
    .unwrap();
    let actors = std::sync::Arc::new(aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap());
    let catalog = Catalog::new(&actors);
    let count = object_registry_count_b39f58(&catalog).unwrap();
    assert_eq!(count, actors.actors.len());
    for ids in ["rus_corps", "rus_truck_engineer", "regiment_rus_hq_infantry"] {
        let (value, bits) = object_reference_76a880(&catalog, Some(ids)).unwrap();
        assert_eq!(bits, aa_orders::registry_bits(count + 1).unwrap());
        let d = resolve_object_76a960(&catalog, value).unwrap().expect("ActorData");
        assert!(d.id.eq_ignore_ascii_case(ids));
    }
    assert_eq!(object_reference_76a880(&catalog, None).unwrap().0 as usize, count);
    assert!(resolve_object_76a960(&catalog, count as u32).unwrap().is_none());
}
