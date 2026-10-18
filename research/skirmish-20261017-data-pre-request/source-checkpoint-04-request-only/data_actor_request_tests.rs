use super::*;
#[test]
fn unprepared_foreign_destroyed_actor_source_never_mints_empty_lists(){
    let mut state=State::from_native_constructor();
    let p=Provenance{path:"identity-lab-only.tgi".into(),sha256:"identity-lab-only".into(),span:aa_rules::tgi::Span{start:0,end:1}};
    let data=state.bind_canonical("actor",&p,2,&[],aa_numeric::Precision::Double53).unwrap();
    assert!(state.actor_ability_count(&data,false).is_err());
    assert!(state.actor_ability_count(&data,true).is_err());
    assert!(state.actor_extra_ability_2a4(&data).is_err());
    assert!(state.actor_settlement_534(&data).is_err());
    let foreign=State::from_native_constructor();assert!(foreign.actor_ability_count(&data,false).is_err());
    drop(state);assert!(foreign.actor_extra_ability_2a4(&data).is_err());
}

#[test]
#[ignore="requires actual source files/completed Bank; source graphs only, not World readiness"]
fn actual_farm_and_ger_corps_request_graphs_keep_source_pc_and_allocation(){
    use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
    catalog.settings=Some(crate::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
    let receipt=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
    crate::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());let pc=bank.combat_source().unwrap().source_precision();
    let mut graphs=0;let mut positive_health=false;
    for id in ["weurope_farm6x8_01","ger_corps"]{
        let binding=catalog.actor_bindings(id,pc).unwrap();let definition=&binding.definition;let node=root(&catalog,definition).unwrap();
        for group in [Group::Mover,Group::Body,Group::Portal]{
            if let Some(scope)=node.child(group.component().unwrap()).unwrap(){
                let graph=fx_graph::actor_request_source::compile(&catalog,&bank,&definition,&scope,group).unwrap();
                graph.require_in_bank(&catalog,&bank).unwrap();assert_eq!(graph.source_precision(),pc);
                assert!(graph.require_in_bank(&catalog,&Bank::default()).is_err());
                for(i,name)in group.names().iter().enumerate(){
                    let roots=graph.actor_request_roots(group.base()+8*i as u32).unwrap();
                    assert!(roots.len()>=scope.children(name).unwrap().len());
                    if id=="weurope_farm6x8_01"&&*name=="WhenHealth"{positive_health=!roots.is_empty();}
                }
                assert!(fx_graph::actor_request_source::compile(&catalog,&bank,&definition,&node,group).is_err());graphs+=1;
            }
        }
        for name in ["PerformerComponent","DefenderComponent"]{
            if let Some(scope)=node.child(name).unwrap(){for ability in scope.children("Ability").unwrap(){
                let Some(d)=ability.child("AbilityDefinition").unwrap()else{continue};
                let graph=fx_graph::actor_request_source::compile(&catalog,&bank,&definition,&d,Group::Ability).unwrap();
                for(i,name)in Group::Ability.names().iter().enumerate(){assert_eq!(graph.actor_request_roots(0x70+8*i as u32).unwrap().len(),d.children(name).unwrap().len());}
                graphs+=1;
            }}
        }
    }
    assert!(graphs>=3);assert!(positive_health,"actual Farm positive health FX required");
}
