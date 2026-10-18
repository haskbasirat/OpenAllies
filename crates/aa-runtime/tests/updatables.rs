use aa_runtime::updatables::{Error, Table};
#[test]
fn original_registration_and_remove_order() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-runtime/native-updatables.json"
    ))
    .unwrap();
    for case in f["cases"].as_array().unwrap() {
        let mut table = Table::<u32>::new(case["count"].as_i64().unwrap() as i32).unwrap();
        for op in case["ops"].as_array().unwrap() {
            let owner = op["owner"].as_u64().unwrap() as u32;
            let ok = if op["operation"] == "load" {
                if op["version"] == 0 {
                    true
                } else {
                    table
                        .register(op["id"].as_i64().unwrap() as i32, owner)
                        .is_ok()
                }
            } else {
                table.remove(&owner)
            };
            assert_eq!(ok, op["ok"].as_bool().unwrap(), "{op}");
            assert_eq!(serde_json::to_value(table.slots()).unwrap(), op["slots"]);
        }
    }
}
#[test]
fn corrupt_table_boundary_does_not_reproduce_native_heap_overrun() {
    let mut table = Table::new(2).unwrap();
    table.register(0, 99).unwrap();
    assert_eq!(
        table.register(2, 17),
        Err(Error::Bounds { id: 2, count: 2 })
    );
    assert_eq!(table.slots(), &[Some(99), None]);
    assert_eq!(table.resolve(-1).unwrap(), None);
    assert!(table.resolve(-2).is_err());
    assert_eq!(table.resolve(1).unwrap(), None);
    assert!(Table::<()>::new(-1).is_err());
}
