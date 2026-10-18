//! The oracle executes the actual CRT loop and direct registration bodies in
//! each PE. Its input inventory comes from CRT machine-code CFGs, not Rust IDs.
use aa_orders::{Cursor, Position, flow, native, payload};
use serde_json::Value;

fn report(relative: &str) -> Value {
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    serde_json::from_slice(&std::fs::read(root.join(relative)).unwrap()).unwrap()
}

#[test]
fn both_executed_crt_sequences_include_the_unexported_initializer() {
    let f=report("research/skirmish-20261017-order-registry-correction/native-registry-audit.json");
    assert_eq!(f["missing_from_calls_tsv"],serde_json::json!(["0x9e65f0"]));
    let machines=f["results"].as_array().unwrap();
    assert_eq!(machines.len(),2);
    assert_ne!(machines[0]["sha256"],machines[1]["sha256"]);
    for m in machines {
        assert_eq!(m["audit"].as_array().unwrap().len(),3634);
        let actual=m["registrations"].as_array().unwrap();
        assert_eq!(actual.len(),358);
        for (row,d) in actual.iter().zip(native::DESCRIPTORS) {
            assert_eq!(row["id"],d.id);
            assert_eq!(row["name"],d.name);
            assert_eq!(row["handler"],format!("{:#x}",d.handler_va));
        }
        assert_eq!(actual[323]["initializer"],"0x9e65f0");
        assert_eq!(actual[323]["descriptor"],"0xb40af0");
    }
    let old=report("research/skirmish-20261017-order-registry-correction/historical-357/research/phase4-orders/registry-exploration.json");
    for row in old.as_array().unwrap() {
        let old_id=row["id"].as_u64().unwrap() as usize;
        let id=old_id+usize::from(old_id>=323);
        assert_eq!(row["name"],native::DESCRIPTORS[id].name);
        assert_eq!(row["handler"],format!("{:#x}",native::DESCRIPTORS[id].handler_va));
    }
}

#[test]
fn acknowledgement_is_not_fabricated_as_a_request_decree_template() {
    assert_eq!(native::DESCRIPTORS[323].name,"OrderVersionAckRequest");
    assert_eq!(native::DESCRIPTORS[323].handler_va,0x766cf0);
    assert!(flow::shape(323).is_err());
    assert!(payload::schema(323).is_ok()); // no payload: native thunk -> Admin
    let mut input=Cursor::with_bounds(&[0],Position{byte:0,bit:3},3).unwrap();
    struct Empty;
    impl payload::SchemaContext for Empty {
        fn registry_count(&self,_:payload::Registry)->aa_orders::Result<usize>{unreachable!()}
        fn team_command_flags(&self,_:u32)->aa_orders::Result<u32>{unreachable!()}
        fn definition_kind(&self,_:payload::Registry,_:u32)->aa_orders::Result<u32>{unreachable!()}
        fn zero_width_list_budget(&self)->usize{0}
    }
    assert!(payload::decode(323,&mut input,&Empty).unwrap().is_empty());
    assert_eq!(input.position(),Position{byte:0,bit:3});
    assert_eq!(native::DESCRIPTORS[324].name,"KKC_UpdateKingdomNameOrderRequest");
    assert_eq!(flow::shape(324).unwrap().decree,328);
    assert_eq!(flow::shape(357).unwrap().descriptor,357);
    assert_eq!(payload::SCHEMAS,358);
}

#[test]
fn direct_version_header_matches_both_pe_full_registry_executions() {
    let f=report("research/skirmish-20261017-order-registry-correction/native-version-ack-header.json");
    let registry=native::registry().unwrap();
    for m in f["machines"].as_array().unwrap() {
        assert_eq!(m["count"],358);assert_eq!(m["id"],323);
        for row in m["rows"].as_array().unwrap() {
            let start:Position=serde_json::from_value(row["start"].clone()).unwrap();
            let mut writer=aa_orders::commands::WireWriter::new(start).unwrap();
            // The fixture also preserves bytes/bits before the current cursor.
            // WireWriter's fresh allocation starts those bits at0, so compare
            // the native header after masking only the pre-existing prefix.
            native::write_version_ack_request(&registry,&mut writer).unwrap();
            let mut raw:Vec<u8>=serde_json::from_value(row["raw"].clone()).unwrap();
            for v in &mut raw[..start.byte]{*v=0;}
            raw[start.byte]&=!((1u16<<start.bit)-1) as u8;
            assert_eq!(writer.position(),serde_json::from_value::<Position>(row["end"].clone()).unwrap());
            assert_eq!(writer.bytes(),raw);
        }
    }
    let mut writer=aa_orders::commands::WireWriter::new(Position::PACKET_START).unwrap();
    let before=writer.position();
    let incomplete=aa_orders::Registry::new(native::DESCRIPTORS[..323].iter().map(|d|d.name.to_owned()).collect()).unwrap();
    assert!(native::write_version_ack_request(&incomplete,&mut writer).is_err());
    assert_eq!(writer.position(),before);
}
