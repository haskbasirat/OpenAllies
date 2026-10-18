//! Replays research/phase8-economy/native_restrictions.py: original kingdom/
//! team restriction checks 0x4E1910/0x4E1810 and 0x4E1890 over native string
//! maps (0x504560/0x504500/0x504530, 0x4154D0, 0x405030).
use aa_economy::restrictions::*;
use serde_json::Value;

fn entries(v: &Value) -> Vec<(String, bool)> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|e| (e[0].as_str().unwrap().to_owned(), e[1].as_bool().unwrap()))
        .collect()
}

#[test]
fn restrictions_match_original_x86() {
    let document: Value =
        serde_json::from_str(include_str!("fixtures/native-restriction-fixtures.json")).unwrap();
    let mut executions = 0;
    for fixture in document["fixtures"].as_array().unwrap() {
        let case = &fixture["case"];
        let profiles = RestrictionProfiles {
            profiles: case["profiles"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, p)| {
                    (
                        name.clone(),
                        RestrictionProfile {
                            actors: entries(&p["actors"]),
                            technologies: entries(&p["technologies"]),
                        },
                    )
                })
                .collect(),
        };
        let team = case["team"].as_str().and_then(|name| profiles.find(name));
        let kingdom = profiles.find(case["kingdom"].as_str().unwrap());
        for (op, expected) in case["ops"]
            .as_array()
            .unwrap()
            .iter()
            .zip(fixture["expected"].as_array().unwrap())
        {
            let id = op["id"].as_str().unwrap();
            let actual = match op["kind"].as_str().unwrap() {
                "actor" => actor_allowed(team, kingdom, id),
                _ => technology_allowed(team, kingdom, id),
            };
            assert_eq!(
                actual,
                expected.as_bool().unwrap(),
                "{} {op}",
                fixture["name"]
            );
            executions += 1;
        }
    }
    assert_eq!(executions, document["executions"].as_u64().unwrap());
}
