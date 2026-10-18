//! Peer-generated native vectors use the same explicit fixture World contract.
//! Run peer_ independently; the included producer drivers are not modified.
mod driver {
    include!("ability_motion_native.rs");

    #[test]
    fn peer_motion_ties_and_callback_changes_match_original() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/phase7-ability-motion/independent-extra-motion.json"
        ))
        .unwrap();
        for row in fixture["cases"].as_array().unwrap() {
            let input = &row["input"];
            let (mut arena, installed, mut decision, mut host) = setup(input);
            let precision = serde_json::from_value(input["precision"].clone()).unwrap();
            if input["operation"] == "wait" {
                wait(&installed, &mut decision, &mut arena, &mut host).unwrap();
            } else {
                pursue(&installed, &mut decision, &mut arena, &mut host, precision).unwrap();
            }
            assert_eq!(
                host.result(&decision, &arena),
                row["output"],
                "{}",
                input["name"]
            );
            assert!(host.c["mutations"].as_array().unwrap().is_empty());
        }
    }
}
