//! Storage transport only: constructor-controlled explicit owner fixtures.
use super::*;
struct Constructor;
impl aa_mind::construct::World for Constructor {
    fn random_word(&mut self) -> Result<u32> {
        Ok(0)
    }
    fn performer_ability_count(&self, _: u32) -> Result<usize> {
        Ok(0)
    }
    fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> Result<()> {
        unreachable!()
    }
}
fn handle() -> PayloadHandle {
    let made = aa_mind::construct::construct(
        Kind::Element,
        42,
        &mut Constructor,
        aa_numeric::Precision::Double53,
    )
    .unwrap();
    let bindings = made
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.offset, aa_mind::StateId(i as u32 + 1)))
        .collect();
    let mut arena = aa_mind::Arena::default();
    let mut installed = made
        .install(aa_mind::MindId(1), &bindings, &mut arena)
        .unwrap();
    let mut runtime = Runtime::default();
    let state = runtime.bind_element(&mut installed).unwrap();
    runtime.handle(state).unwrap()
}
#[test]
fn movement_payload_loans_share_one_allocation_and_refuse_reentry_without_waiting() {
    let value = handle();
    let alias = value.clone();
    assert!(Arc::ptr_eq(&value.0, &alias.0));
    let reader = value.read().unwrap();
    let other_reader = alias.read().unwrap();
    assert_eq!(reader.word78, other_reader.word78);
    assert!(value.write().is_err());
    drop(reader);
    drop(other_reader);
    let mut writer = value.write().unwrap();
    writer.word78 = 0x12345678;
    assert!(alias.write().is_err());
    assert!(alias.read().is_err());
    drop(writer);
    assert_eq!(alias.read().unwrap().word78, 0x12345678);
}
#[test]
fn poisoned_movement_payload_retains_writes_and_can_move_between_threads() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<PayloadHandle>();
    assert_sync::<PayloadHandle>();
    assert_send::<crate::mind_world::Runtime>();
    let value = handle();
    let alias = value.clone();
    let pointer = Arc::as_ptr(&value.0) as usize;
    let joined = std::thread::spawn(move || {
        let mut writer = alias.write().unwrap();
        writer.word78 = 0x89abcdef;
        panic!("explicit storage-poison probe, not a game callback");
    })
    .join();
    assert!(joined.is_err());
    assert_eq!(Arc::as_ptr(&value.0) as usize, pointer);
    assert_eq!(value.read().unwrap().word78, 0x89abcdef);
    value.write().unwrap().word78 = 7;
    assert_eq!(value.read().unwrap().word78, 7);
}
