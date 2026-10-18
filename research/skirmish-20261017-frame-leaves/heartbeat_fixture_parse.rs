extern crate aa_rules;
fn main(){
    let bytes=b"[Vars]\n{\n string GoalPriorityCSV = null\n}\n".to_vec();
    let d=aa_rules::tgi::Document::parse_native(bytes,aa_rules::tgi::Limits::default()).unwrap();
    assert_eq!(d.items.len(),1);
    println!("native multiline heartbeat constructor fixture parses PASS");
}
