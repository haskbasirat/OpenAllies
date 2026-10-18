use aa_mind::{Arena,MindId,StateId,construct::{self,Kind}};
use aa_numeric::Precision;
use aa_runtime::movement_world::Runtime;
use std::collections::BTreeMap;
struct ConstructorWorld;
impl construct::World for ConstructorWorld {
    fn random_word(&mut self)->aa_mind::Result<u32>{Ok(0x12345678)}
    fn performer_ability_count(&self,_:u32)->aa_mind::Result<usize>{Ok(0)}
    fn set_ability_jitter(&mut self,_:u32,_:usize,_:u32)->aa_mind::Result<()> {panic!("no fixture abilities")}
}
#[test]
fn move_payload_transfers_native_fields_once_and_keeps_one_live_allocation() {
    let constructor=construct::construct(Kind::Element,0x10007,&mut ConstructorWorld,Precision::Double53).unwrap();
    let ids:BTreeMap<_,_>=constructor.states.iter().enumerate().map(|(n,s)|(s.offset,StateId(n as u32+1))).collect();
    let mut arena=Arena::default();
    let mut installed=constructor.install(MindId(1),&ids,&mut arena).unwrap();
    let before=installed.clone();
    let mut runtime=Runtime::default();
    let state=runtime.bind_element(&mut installed).unwrap();
    let first=runtime.handle(state).unwrap();let second=runtime.handle(state).unwrap();
    assert_eq!(first.read().unwrap().plan.movement.auxiliary_bits,[before.words[&0x98],before.words[&0x9c]]);
    assert_eq!(first.read().unwrap().collision_vector,[before.words[&0xc8],before.words[&0xcc]]);
    assert_eq!(first.read().unwrap().plan.movement.path.word18,before.words[&0xec]);
    assert!(!installed.words.contains_key(&0x8c));assert!(!installed.references.contains_key(&0xd4));
    assert_eq!(installed.words[&0x198],before.words[&0x198]);
    {
        let mut payload=first.write().unwrap();payload.plan.movement.phase=7;
        assert!(second.write().is_err());assert!(second.read().is_err());
    }
    assert_eq!(second.read().unwrap().plan.movement.phase,7);
    assert_eq!(arena.states[&state].status,0);
    assert!(runtime.bind_element(&mut installed).is_err());
}
#[test]
fn unresolved_native_path_and_uninitialized_words_fail_without_consuming_source() {
    for missing in [true,false] {
        let constructor=construct::construct(Kind::Element,0x10007,&mut ConstructorWorld,Precision::Double53).unwrap();
        let ids=constructor.states.iter().enumerate().map(|(n,s)|(s.offset,StateId(n as u32+1))).collect();
        let mut installed=constructor.install(MindId(1),&ids,&mut Arena::default()).unwrap();
        if missing {installed.words.remove(&0xcc);} else {installed.words.insert(0xd8,0x1234);}
        let before=installed.words.clone();let mut runtime=Runtime::default();
        assert!(runtime.bind_element(&mut installed).is_err());assert_eq!(installed.words,before);
        assert!(runtime.handle(installed.state_bindings[&0x7c]).is_err());
    }
}
