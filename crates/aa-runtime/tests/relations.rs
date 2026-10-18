use aa_actors::ActorId;
use aa_runtime::relations::Relations;
use serde_json::Value;

#[test]
fn original_actor_kingdom_and_team_relation_precedence() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-runtime/native-relations.json"
    ))
    .unwrap();
    assert_eq!(f["query_count"], 576);
    for case in f["cases"].as_array().unwrap() {
        let r = Relations {
            orphan_relation: case["defaults"][0].as_u64().unwrap() as u32,
            team_null_relation: case["defaults"][1].as_u64().unwrap() as u32,
            teams: serde_json::from_value(case["teams"].clone()).unwrap(),
            kingdoms: serde_json::from_value(case["kingdoms"].clone()).unwrap(),
        };
        for q in case["queries"].as_array().unwrap() {
            let id = |name: &str| q[name].as_u64().map(|v| ActorId::from_raw(v as u32));
            let value = match q["kind"].as_str().unwrap() {
                "actor" => r.actor_relation(id("owner"), id("other")).unwrap(),
                "kingdom" => r.kingdom_relation(id("id").unwrap(), id("other")).unwrap(),
                "team" => r.team_relation(id("id").unwrap(), id("other")).unwrap(),
                _ => panic!(),
            };
            assert_eq!(value, q["relation"].as_u64().unwrap() as u32, "{q}");
            if q["kind"] == "actor" {
                assert_eq!(Relations::hostile(value), q["hostile"].as_bool().unwrap());
                assert_eq!(Relations::allied(value), q["allied"].as_bool().unwrap());
            }
        }
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original map corpus"]
fn all_original_saved_relation_tables_bind_to_actual_ids() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut directories = vec![game.join("reconstruction/assets/original/Data/Maps")];
    let mut maps = 0;
    let mut kingdoms = 0;
    while let Some(dir) = directories.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                directories.push(p);
                continue;
            }
            if !p.extension().is_some_and(|s| s.eq_ignore_ascii_case("RMP")) {
                continue;
            }
            let map = aa_mapworld::MapArchive::from_bytes(std::fs::read(&p).unwrap()).unwrap();
            let input = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
            let relations = Relations::from_inventory(&input, 2, 2)
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            for &a in relations.kingdoms.keys() {
                for &b in relations.kingdoms.keys() {
                    relations
                        .kingdom_relation(a, Some(b))
                        .unwrap_or_else(|e| panic!("{} {a:?}->{b:?}: {e}", p.display()));
                }
            }
            kingdoms += relations.kingdoms.len();
            maps += 1;
        }
    }
    assert_eq!((maps, kingdoms), (80, 328));
}
