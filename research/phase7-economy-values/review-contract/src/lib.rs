#![cfg(test)]
include!("../../../../crates/aa-actors/tests/economy_values_native.rs");

#[test]
fn reviewer_extra_native_transport_layout_modifiers_and_supply_boundaries() {
    let doc:Value=serde_json::from_str(include_str!("../../review-extra-fixtures.json")).unwrap();
    assert_eq!(doc["cases"].as_array().unwrap().len(),51);
    for row in doc["cases"].as_array().unwrap() {
        let c=&row["input"];let mut host=Host{input:c.clone()};let actor=ActorId::from_raw(1);
        let p=serde_json::from_value(c["precision"].clone()).unwrap();
        let mut out:Vec<F32>=serde_json::from_value(c["initial_output"].clone()).unwrap();
        let predicate=match c["operation"].as_str().unwrap() {
            "supplied"=>Some(economy_values::supplied(&host.actor(actor).unwrap())),
            "Production"=>{economy_values::production(&mut out,Some(actor),c["cache_members"]==true,&mut host,p).unwrap();None},
            name=>{economy_values::cost_or_upkeep(&mut out,Some(actor),serde_json::from_value(json!(name)).unwrap(),&host,p).unwrap();None}
        };
        assert_eq!(json!({"predicate":predicate,"values":out,"caches":host.input["caches"]}),row["output"],"{}",c["name"]);
    }
}
struct FailingCache {host:Host,writes:Vec<u32>}
impl GateWorld for FailingCache {
    type Definition=u32;
    fn actor(&self,a:ActorId)->Result<ActorSnapshot<u32>>{self.host.actor(a)}
    fn definition(&self,d:&u32)->Result<Definition<u32>>{self.host.definition(d)}
}
impl World for FailingCache {
    fn resource_count(&self)->Result<usize>{self.host.resource_count()}
    fn actor_modifier(&self,a:ActorId)->Result<Option<u32>>{self.host.actor_modifier(a)}
    fn modifier(&self,c:u32)->Result<Modifier>{self.host.modifier(c)}
    fn transport_vector(&self,t:u32,k:VectorKind)->Result<Vec<F32>>{self.host.transport_vector(t,k)}
    fn layout_slot_count(&self,o:u32)->Result<usize>{self.host.layout_slot_count(o)}
    fn layout_slot_definition(&self,o:u32,i:usize)->Result<Option<u32>>{self.host.layout_slot_definition(o,i)}
    fn member_count(&self,o:u32)->Result<usize>{self.host.member_count(o)}
    fn member(&self,o:u32,i:usize)->Result<Option<ActorId>>{self.host.member(o,i)}
    fn store_member_production(&mut self,c:u32,v:&[F32])->Result<()> {
        self.writes.push(c);self.host.store_member_production(c,v)?;
        if c==3 {Err(economy_values::Error::World("review failure after cache write".into()))}else{Ok(())}
    }
}
#[test]
fn reviewer_failed_cache_retains_committed_prefix_and_stops_traversal() {
    let f=fixture();let mut c=f["cases"][0]["input"].clone();
    c["actors"]["1"]["organization"]=json!(1);
    c["organizations"]["1"]["members"]=json!([2,3,4]);
    let before=c["caches"].clone();
    let mut w=FailingCache{host:Host{input:c},writes:vec![]};let mut out=vec![];
    assert!(economy_values::production(&mut out,Some(ActorId::from_raw(1)),true,&mut w,aa_numeric::Precision::Double53).is_err());
    assert_eq!(w.writes,[2,3]);assert_ne!(w.host.input["caches"]["2"],before["2"]);assert_ne!(w.host.input["caches"]["3"],before["3"]);assert_eq!(w.host.input["caches"]["4"],before["4"]);
    assert_eq!(out.len(),3);assert!(f32::from_bits(out[0].0)>10.);
}
