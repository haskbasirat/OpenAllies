//! Exact shared production relation kernel, without a heavyweight runtime link.
#[path="../../aa-runtime/src/relation_query.rs"]
mod production;
use production::Read;
use serde_json::Value;
struct World<'a>(&'a Value);
type Result<T>=std::result::Result<T,String>;
impl World<'_> {
    fn row(&self,kind:&str,id:u32)->Result<&Value> {
        self.0[kind].get(id.to_string()).ok_or_else(||"missing actual owner".into())
    }
    fn word(&self,kind:&str,id:u32,team:bool,slot:u32)->Result<u32> {
        self.row(kind,id)?[if team{"teams"}else{"kingdoms"}].get(slot as usize)
            .and_then(Value::as_u64).map(|n|n as u32).ok_or_else(||"missing current vector slot".into())
    }
}
impl Read for World<'_> {
    type Id=u32;type Error=String;
    fn kingdom_team(&self,id:u32)->Result<Option<u32>> {Ok(self.row("kingdoms",id)?["team"].as_u64().map(|v|v as u32))}
    fn kingdom_slot(&self,id:u32)->Result<u32>{Ok(self.row("kingdoms",id)?["slot"].as_u64().unwrap()as u32)}
    fn team_slot(&self,id:u32)->Result<u32>{Ok(self.row("teams",id)?["slot"].as_u64().unwrap()as u32)}
    fn kingdom_word(&self,id:u32,team:bool,slot:u32)->Result<u32>{self.word("kingdoms",id,team,slot)}
    fn team_word(&self,id:u32,team:bool,slot:u32)->Result<u32>{self.word("teams",id,team,slot)}
    fn orphan_relation(&self)->Result<u32>{self.0["defaults"][0].as_u64().map(|v|v as u32).ok_or_else(||"orphan global unbound".into())}
    fn team_null_relation(&self)->Result<u32>{self.0["defaults"][1].as_u64().map(|v|v as u32).ok_or_else(||"team null global unbound".into())}
}
#[test]
fn freshly_executed_both_pe_all_pc_relations_match_shared_kernel() {
    let fixture:Value=serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-city-after-init/native-zone-relations.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),96);
    let mut count=0;
    for row in rows {
        let w=World(&row["input"]);
        for query in row["queries"].as_array().unwrap() {
            count+=1;let q=&query["question"];let n=&query["native"];
            let id=|s:&str|q[s].as_u64().map(|v|v as u32);let other=id("other");
            let value=match q["kind"].as_str().unwrap(){
                "actor"=>production::actor(&w,id("owner"),other),
                "kingdom"=>production::kingdom(&w,id("id").unwrap(),other),
                _=>production::team_relation(&w,id("id").unwrap(),other),
            }.unwrap();
            assert_eq!(value,n["relation"].as_u64().unwrap()as u32);
            if n.get("accepts").is_some() {
                let same=if q["kind"]=="actor"{id("owner")==other}else{Some(id("id").unwrap())==other};
                assert_eq!(same||value==1,n["accepts"].as_bool().unwrap());
                assert_eq!(value==3,n["enemy"].as_bool().unwrap());
            }
        }
    }
    assert_eq!(count,3456);
    //Pointer equality precedes any table/global read, even if absent.
    let empty=serde_json::json!({});let missing=World(&empty);
    assert_eq!(production::actor(&missing,None,None).unwrap(),0);
    assert_eq!(production::actor(&missing,Some(9),Some(9)).unwrap(),0);
    assert_eq!(production::kingdom(&missing,9,Some(9)).unwrap(),0);
    assert!(production::actor(&missing,None,Some(9)).is_err());
    assert!(production::kingdom(&missing,9,None).is_err());
    assert!(production::team_relation(&missing,9,None).is_err());
}
