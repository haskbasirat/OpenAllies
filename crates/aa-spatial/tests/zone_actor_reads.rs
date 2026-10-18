//! Exact host read helpers plus the already ported native geometry kernel.
#[path="../../aa-runtime/src/zone_actor_reads.rs"]
mod production;
#[test]
fn native_class_geometry_and_cached_coordinate_leaves() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-city-after-init/native-actor-zone-reads.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),1008);
    for row in rows {
        let i=&row["input"];let n=&row["native"];
        let class=i["class"].as_u64().unwrap() as u32;let is_actor=matches!(class,3|4);
        assert_eq!(production::is_actor(class,if is_actor{2}else{1},true,is_actor).unwrap(),n["b4"].as_bool().unwrap());
        let pc=match row["precision"].as_str().unwrap(){"Single24"=>aa_numeric::Precision::Single24,"Double53"=>aa_numeric::Precision::Double53,_=>aa_numeric::Precision::Extended64};
        let flags=i["flags"].as_u64().unwrap()as u32;
        assert_eq!(aa_spatial::geometry::marks_blocker(flags,i["radius"].as_u64().unwrap()as u32,i["cell"].as_u64().unwrap()as u32,pc).unwrap(),n["multi"].as_bool().unwrap());
        let xy=i["xy"].as_array().unwrap().iter().map(|v|v.as_i64().unwrap()as u32).collect::<Vec<_>>().try_into().unwrap();
        assert_eq!(production::cached_tile(i["width"].as_i64().unwrap()as i32,xy).unwrap(),n["flat"].as_i64().unwrap());
    }
    assert!(production::is_actor(3,1,true,true).is_err());
    assert!(production::is_actor(3,2,true,false).is_err());
    assert!(production::is_actor(2,1,false,false).is_err());
    assert!(production::is_actor(8,2,true,true).is_err());
    assert!(production::cached_tile(0,[0,0]).is_err());
}
