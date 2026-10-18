use aa_triggers::{decode::SavedManager,manager::Manager};

#[test]
fn native_constructor_fields_and_same_owner_load() {
    let mut owner=Manager::from_native_constructor();
    let initial=owner.snapshot();
    assert_eq!(initial.next_id,1); //54CEE0+0C
    assert_eq!(initial.context_word_10,0);
    assert!(initial.triggers.is_empty() && initial.variables.is_empty() && initial.strings.is_empty());
    assert!(initial.subjects.is_empty() && initial.deferred.is_empty());
    assert_eq!(initial.condition_kingdom,None);
    assert_eq!(initial.event_kingdom,None);
    for index in 0..8 {assert!(owner.register_text(index).unwrap().is_empty());}
    owner.add_subject(17);
    owner.set_context_word(29);
    owner.set_condition_kingdom(Some(41));
    let before=owner.snapshot();
    let address=std::ptr::from_ref(&owner);
    owner.install_saved_on_fresh(SavedManager {next_id:23,triggers:vec![],variables:vec![],
        strings:vec!["actual payload".encode_utf16().collect()]}).unwrap();
    assert_eq!(std::ptr::from_ref(&owner),address);
    let after=owner.snapshot();
    assert_eq!(after.next_id,23);
    assert_eq!(after.context_word_10,before.context_word_10);
    assert_eq!(after.subjects,before.subjects);
    assert_eq!(after.condition_kingdom,before.condition_kingdom);
    assert!(owner.saved_payload_installed());
    assert!(owner.install_saved_on_fresh(SavedManager {next_id:99,triggers:vec![],variables:vec![],strings:vec![]}).is_err());
    assert_eq!(owner.snapshot(),after);
}
