//! The exact production helper, independently replayed without a runtime link.
#[path="../../aa-runtime/src/zone_effects_gate.rs"]
mod production;
use std::cell::RefCell;
#[test]
fn native_gate_values_and_lazy_owner_reads() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-city-after-init/native-effects-gate.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),120);
    for row in rows {
        let reads=RefCell::new(vec![]);
        let result=production::enabled::<&str>(|| {
            reads.borrow_mut().push("session_e4");Ok(row["input"]["e4"].as_u64().unwrap() as u32)
        },|| {
            reads.borrow_mut().push("world48");Ok(row["input"]["world48"].as_u64().unwrap() as u32)
        }).unwrap();
        assert_eq!(serde_json::json!(result),row["native"]["enabled"]);
        assert_eq!(serde_json::json!(reads.into_inner()),row["native"]["reads"]);
    }
    assert_eq!(production::enabled(||Err("session absent"),||panic!("World read after failed Session")),Err("session absent"));
    assert_eq!(production::enabled(||Ok(2),||Err("unbound World")),Ok(true));
    assert_eq!(production::enabled(||Ok(3),||Err("unbound World")),Err("unbound World"));
    //The same getter observes later stores, without a cached gate word.
    let current=std::cell::Cell::new(0u32);
    assert!(!production::enabled::<()>(||Ok(3),||Ok(current.get())).unwrap());
    current.set(0x80000000);
    assert!(production::enabled::<()>(||Ok(3),||Ok(current.get())).unwrap());
}
