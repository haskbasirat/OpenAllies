#[path="../../aa-runtime/src/zone_city_candidate.rs"]
mod production;
use std::cell::RefCell;
struct World<'a>{input:&'a serde_json::Value,reads:RefCell<Vec<&'static str>>}
impl production::Read for World<'_> {
    type Error=&'static str;
    fn flags(&self,_:u32)->Result<u32,Self::Error>{self.reads.borrow_mut().push("flags");Ok(self.input["flags"].as_u64().unwrap()as u32)}
    fn owner(&self,_:u32)->Result<u32,Self::Error>{self.reads.borrow_mut().push("owner");Ok(if self.input["owner"].as_bool().unwrap(){7}else{0})}
    fn nonmajor(&self,k:u32)->Result<u32,Self::Error>{self.reads.borrow_mut().push("nonmajor");assert_eq!(k,7);Ok(self.input["nonmajor"].as_u64().unwrap()as u32)}
}
#[test]
fn actual_city_rejections_and_first_required_child_match_native_read_order() {
    let f:serde_json::Value=serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-city-after-init/native-city-candidate.json")).unwrap();
    let rows=f["rows"].as_array().unwrap();assert_eq!(rows.len(),96);
    for row in rows {
        let i=&row["input"];let w=World{input:i,reads:RefCell::new(vec![])};
        let target=match i["target"].as_str().unwrap(){"null"=>0,"self"=>1,_=>2};
        let needs=production::needs_redirect(&w,1,target).unwrap();
        assert_eq!(needs,row["native"]["terminal"]=="redirect_required");
        assert_eq!(serde_json::json!(w.reads.into_inner()),row["native"]["reads"]);
        assert!(row["native"]["writes"].as_array().unwrap().is_empty());
    }
}
