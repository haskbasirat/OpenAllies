//! Replays research/phase8-ai/native-saidata-profile.json (original
//! constructor 4AA430 + KKC_SAIData::LoadData 4AE3E0 under Unicorn, on the
//! shipped [SAI] profiles and mutations) against aa_mind::sai::profile.
mod common;
use aa_mind::sai::profile::{SaiProfile, load};
use aa_numeric::Precision;
use common::data_host::AnswerHost;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-saidata-profile.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn native_sai_profile_load_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 175);
    let (mut faults, mut shipped) = (0, 0);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = AnswerHost::new(c["answers"].clone());
        let mut profile = SaiProfile::new(c["ids"].as_str().unwrap());
        let root = c["root"].as_u64().unwrap() as u32;
        let result = load(&mut host, root, &mut profile, p);
        assert_eq!(json!(host.events), o["events"], "{name}");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}");
            faults += 1;
            continue;
        }
        result.unwrap_or_else(|e| panic!("{name}: {e}"));
        if name.as_str().unwrap().starts_with("shipped_") {
            shipped += 1;
        }
        let mut nations = profile.nations.clone();
        nations.sort();
        let mut factions = profile.factions.clone();
        factions.sort();
        let got = json!({
            "name": profile.name,
            "loaded": u8::from(profile.loaded),
            "difficulty": profile.difficulty,
            "public": u8::from(profile.public),
            "command_max_delay_ms": profile.command_max_delay_ms,
            "command_max_total_delay_ms": profile.command_max_total_delay_ms,
            "max_ge_commands": profile.max_ge_commands as u32,
            "max_rai_commands": profile.max_rai_commands as u32,
            "ego_fuzziness": profile.ego_fuzziness,
            "ego_refs": profile.ego_refs,
            "nations": nations,
            "nation_count": profile.nations.len(),
            "factions": factions,
            "faction_count": profile.factions.len(),
            "first_nation": profile.first_nation,
            "first_faction": profile.first_faction,
        });
        let mut want = o.clone();
        want.as_object_mut().unwrap().remove("events");
        assert_eq!(got, want, "{name}");
    }
    assert_eq!(shipped, 25);
    assert_eq!(faults, 84);
}
