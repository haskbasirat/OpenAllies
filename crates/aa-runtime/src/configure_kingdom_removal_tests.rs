use super::*;
use serde_json::{Value,json};

#[test]
fn native_positive_kingdom_owned_destructor_fields_all_pc() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-configure-kingdom-removal/native-kingdom-e0.json")).unwrap();
    let mut cases=0;
    for row in fixture["rows"].as_array().unwrap() {
        let c=&row["input"];
        let mut s=SpecialShell::kingdom(ActorId::from_raw(1677),7).unwrap();
        if c["strategic"]!=false {s.strategic=Some(aa_mapworld::kingdom_ai::StrategicState::new(true));s.words.remove(&0x330);}
        if c["tactical"]!=false {
            let mut t=aa_mapworld::kingdom_ai::TacticalState::new().unwrap();
            let n=c["table_nodes"].as_u64().unwrap_or(0) as u32;
            for table in &mut t.handles {
                for i in 0..n {table.buckets[0]=Some(Box::new(aa_mapworld::kingdom_ai::Entry {key:ActorId::from_raw(100+i),value:200+i,next:table.buckets[0].take()}));}
                table.count=n;
            }
            for table in &mut t.definitions {
                for i in 0..n {table.buckets[0]=Some(Box::new(aa_mapworld::kingdom_ai::Entry {key:aa_mapworld::restore::NamedDefinition {id:format!("lab{i}"),index:None},value:200+i,next:table.buckets[0].take()}));}
                table.count=n;
            }
            s.tactical=Some(t);s.words.remove(&0x344);
        }
        if c["commander"]!=false {
            let counts=c["summoned"].as_array().cloned().unwrap_or_else(||vec![json!(0),json!(0),json!(0)]);
            let abilities=counts.iter().enumerate().filter_map(|(i,n)|n.as_u64().map(|n| {
                let mut a=aa_combat::ability::Ability::new(i as u32,None,aa_body::F32::ZERO);
                a.summoned_ids=vec![0x10007;n as usize];a
            })).collect();
            s.commander=Some(aa_combat::commander::State {definition:2653,kingdom:1677,
                experience:aa_economy::tick::CommanderExperience {balance:aa_economy::F32Bits::ZERO,maximum:aa_economy::F32Bits::ZERO},abilities});
            s.words.remove(&0x230);
        }
        for i in 0..c["counts"].as_u64().unwrap_or(0) {s.definition_counts_mut().insert(100+i as u32,200+i as u32).unwrap();}
        for i in 0..c["recruitment"].as_u64().unwrap_or(0) {s.recruit_items.add_plain(666+i as u32);}
        for text in s.strings_utf16.values_mut(){text.extend("owned-lab-text".encode_utf16());}
        kingdom_owned_destructor_prefix(&mut s).unwrap();
        assert!(s.tactical.is_none()&&s.strategic.is_none()&&s.commander.is_none()&&s.definition_counts.is_none());
        assert!(s.recruit_items.entries().len()==0&&s.recruit_items.singles.is_empty());
        assert!(s.strings_utf16.values().all(Vec::is_empty));
        for (off,v) in &s.vectors {
            assert_eq!(json!([0,0,v.capacity_hint]),row["output"]["vector_headers"][format!("0x{off:x}")],"{row}");
            assert!(v.values.is_empty());
        }
        assert!(row["output"]["error"].is_null());
        cases+=1;
    }
    assert_eq!(cases,42);
}

#[test]
fn later_destructor_failure_keeps_prior_owned_mutations_and_remaining_commander() {
    let mut s=SpecialShell::kingdom(ActorId::from_raw(1677),7).unwrap();
    s.strategic=Some(aa_mapworld::kingdom_ai::StrategicState::new(true));s.words.remove(&0x330);
    s.tactical=Some(aa_mapworld::kingdom_ai::TacticalState::new().unwrap());s.words.remove(&0x344);
    s.commander=Some(aa_combat::commander::State {definition:2653,kingdom:1677,
        experience:aa_economy::tick::CommanderExperience {balance:aa_economy::F32Bits::ZERO,maximum:aa_economy::F32Bits::ZERO},abilities:vec![]});
    s.vectors.get_mut(&0x34c).unwrap().values.push(123);
    let e=kingdom_owned_destructor_prefix(&mut s).unwrap_err();
    assert!(e.contains("4E3ED0"));
    assert!(s.strategic.is_none()&&s.tactical.is_none());
    assert!(s.commander.is_some());
    assert_eq!(s.vectors[&0x34c].values,vec![123]);
    assert_eq!(s.vectors[&0x300].capacity_hint,5);
}
