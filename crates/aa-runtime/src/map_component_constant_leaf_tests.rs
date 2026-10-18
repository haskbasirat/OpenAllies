use super::*;

#[test]
fn original_v14_all_class_tables_and_pure_leaves_match() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-fresh-component-constants/native-constants.json")).unwrap();
    for row in fixture["tables"].as_array().unwrap() {
        let kind=match row["kind"].as_str().unwrap() {
            "4org"=>aa_actors::init_create::FactoryKind::OrganizationMind,
            "4air"=>aa_actors::init_create::FactoryKind::AircraftMind,
            "4elem"=>aa_actors::init_create::FactoryKind::ElementMind,
            slot=>aa_actors::init_create::FactoryKind::Standard(ComponentSlot::from_index(slot.parse().unwrap()).unwrap()),
        };
        let leaves=component_virtuals::leaves(kind);
        assert_eq!(leaves.init_existing as u64,row["init_existing"].as_u64().unwrap());
        assert_eq!(leaves.vf14 as u64,row["after_init"].as_u64().unwrap());
    }
    for row in fixture["rows"].as_array().unwrap() {
        for (_,result) in row["native"].as_object().unwrap() {
            let leaf=result["leaf"].as_u64().unwrap();
            let expected=match leaf {0x6a71d0=>0xa5a5a501,0x4b2830=>0xa5a5a5ee,_=>panic!("nonconstant leaf executed")};
            assert_eq!(result["eax"].as_u64().unwrap(),expected);
        }
    }
}

#[test]
fn actual_typed_economy_ret_and_hq_true_preserve_owner_checks() {
    let owner=ActorId::from_raw(1688);
    let economy=RuntimeComponent::Economy(Box::new(aa_actors::ground_states::EconomyState::new(owner,3).unwrap()));
    let leaf=checked(&economy,owner,ComponentSlot::Economy,1<<9).unwrap();
    assert_eq!(leaf.vf14,0x4b2830);
    assert_ne!(leaf.init_existing,0x6a71d0); //56F280 must still execute.
    assert!(checked(&economy,ActorId::from_raw(1689),ComponentSlot::Economy,1<<9).is_err());
    assert!(checked(&economy,owner,ComponentSlot::Economy,0).is_err());
    assert!(checked(&economy,owner,ComponentSlot::Character,1<<2).is_err());
    let hq=RuntimeComponent::Hq(Box::new(aa_actors::hq_state::HqState::new(owner,0)));
    let leaf=checked(&hq,owner,ComponentSlot::Hq,1<<30).unwrap();
    assert_eq!(leaf.init_existing,0x6a71d0);
    assert_eq!(leaf.vf14,0x4b2830);
}

#[test]
fn nontrivial_city_after_init_is_not_a_return_leaf() {
    let owner=ActorId::from_raw(1688);
    let city=RuntimeComponent::City(Box::new(aa_actors::lair_city_state::CityState::new(owner)));
    let leaf=checked(&city,owner,ComponentSlot::City,1<<29).unwrap();
    assert_eq!(leaf.init_existing,0x6a71d0);
    assert_eq!(leaf.vf14,0x563fe0);
}
