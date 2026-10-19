use super::*;
use aa_mapworld::restore::RuntimeDefinition;
///The 51 definitions whose pre-World source preparation stopped at the
///49CF80 stub (qa-output/development10/lead-gui/census02/berlin/census.json).
const PLAY_SFX_OWNERS:&[&str]=&["kesselring_portrait","manstein_portrait","rommel_portrait","rundstedt_portrait",
    "montgomery_portrait","mountbatten_portrait","wavell_portrait","wingate_portrait","kuribayashi_portrait",
    "mikawa_portrait","nagumo_portrait","yamamoto_portrait","chuikov_portrait","konev_portrait",
    "rokossovski_portrait","zhukov_portrait","arnold_portrait","eisenhower_portrait","nimitz_portrait",
    "patton_portrait","arctic_grasstall","temperate_high_dry_grass","temperate_high_grass","audio_artillery",
    "birds","birds_temperate","branches","cicada","city_noise","crickets","crows","farm","frogs","gun_shots",
    "insect_buzz","insect_tropical","machine_gun","monkeys","owl","parrots","peacock","rain","seagulls",
    "thunder","waves_crashing","wind_arctic","wind_blizzard","wind_desert","wind_temperate","wind_tropical","yelling"];
struct Lab<'a,'s>{catalog:&'a Catalog<'s>,bank:&'a Bank,pc:Precision,actors:&'a aa_rules::actors::ActorCatalog}
///Actual original/v14 sources, completed Bank and the SAME Application Data
///pre-request owner, as in data_aspect_tests; no World/renderer is created.
fn with_actual_sources(call:impl FnOnce(&Lab<'_,'_>,&mut crate::data_pre_request::State,&mut Resources<'_>)->Result<()>){
    use aa_rules::{sources::{SourceLimits,V14Sources},tgi::Limits};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let actors=aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog=Catalog::new(&actors);
    catalog.settings=Some(crate::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
    let receipt=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
    crate::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    let mut app=crate::resource_depot::Application::from_sources(files);
    let lab=Lab{catalog:&catalog,bank:&bank,pc,actors:&actors};
    app.with_data_pre_request_resources(|state,resources|call(&lab,state,resources)).unwrap();
}
///data_aspect's View resource_prefix, source folder and Aspect Animation nodes.
fn view_inputs<'a>(lab:&Lab<'a,'_>,d:&RuntimeDefinition)->Result<(String,String,Vec<ResolvedNode<'a>>)>{
    let root=lab.actors.resolved_actor(aa_rules::actors::ThingRef::registered(&d.proof.id)).map_err(error)?;
    let view=root.child("View").map_err(error)?.ok_or("View absent")?;
    let prefix=string(&view,"resource_prefix",lab.pc)?;
    let normalized=d.proof.source.path.replace('\\',"/");
    let folder=normalized.rsplit_once('/').map_or("",|(f,_)|f).to_owned();
    let mut aspects=view.child("AspectNormal").map_err(error)?.into_iter().collect::<Vec<_>>();
    aspects.extend(view.children("Aspect").map_err(error)?);
    let mut animations=vec![];
    for aspect in &aspects{animations.extend(aspect.children("Animation").map_err(error)?)}
    Ok((prefix,folder,animations))
}
#[test]
#[ignore = "requires AA_GAME_ROOT actual original/v14 sources and completed Bank; source-only lab, no playback/World readiness"]
fn actual_positive_play_sfx_binds_registered_audio_at_owner_position(){
    let mut bound=0;
    with_actual_sources(|lab,state,resources|{
        let (catalog,bank,pc)=(lab.catalog,lab.bank,lab.pc);
        for id in PLAY_SFX_OWNERS{
            let d=catalog.resolve_registered(id,pc)?;
            let data=state.bind_object(catalog,bank,&d)?;
            let (prefix,folder,animations)=view_inputs(lab,&d)?;
            let mut positive=0;
            for animation in &animations{
                let authored=animation.children("PlaySFX").map_err(error)?;
                if authored.is_empty(){continue}
                let mut set=Set::from_native_constructor(&data);
                set.load_source_node(catalog,bank,animation,&prefix,&folder,resources)
                    .map_err(|e|format!("{id}: {e}"))?;
                let loaded=set.core.animations.last().ok_or_else(||format!("{id}: NULL Sequence skipped the Animation"))?;
                assert_eq!(loaded.sfx.len(),authored.len(),"{id}");
                for (sfx,node) in loaded.sfx.iter().zip(&authored){
                    let ids=string(node,"sfx_ids",pc)?;
                    let audio=sfx.audio.as_ref().ok_or("stored AudioData absent")?;
                    //The SAME registered Audio identity as the FX lookup, before the owner.
                    let entry=lab.actors.native_data_find("Audio",&ids).ok_or("registered Audio absent")?;
                    assert!(audio.id.eq_ignore_ascii_case(&entry.ids),"{id}: {ids} -> {}",audio.id);
                    assert_eq!(audio.source.first(),Some(&entry.source),"{id}");
                    assert_eq!(sfx.looping,boolean(node,"looping",pc)?,"{id}");
                }
                positive+=1;
            }
            assert!(positive>0,"{id} has no positive PlaySFX Animation");
            bound+=positive;
            //The real Aspect caller no longer stops at the former49CF80 stub.
            if let Err(e)=state.prepare_object_aspects(catalog,bank,&d,resources){
                assert!(!e.contains("49CF80")&&!e.contains("Unknown sfx")&&!e.contains("Unknown IDS"),"{id}: {e}");
                println!("{id}: later Aspect source child stops at: {e}");
            }
        }
        Ok(())
    });
    println!("{} owners, {bound} positive PlaySFX Animations bound registered Audio at owner position",PLAY_SFX_OWNERS.len());
}
///Explicit lab input for the current UVar61 values read only after a real
///5130A0 miss; this is NOT a claimed installed-profile value.
struct Compatibility{mode:u8,warning:u8}
impl crate::resource_objects::TexturePolicy for Compatibility{
    fn data_compatibility_mode_current(&mut self)->Result<u8>{Ok(self.mode)}
    fn data_missing_resource_warning_current(&mut self)->Result<u8>{Ok(self.warning)}
}
///5A80D0 catches the missing Sequence: mode nonzero -> NULL -> 49DA40 skips
///the whole ObjectAnimation (no 49C020 allocation, reads or appends); mode
///zero rethrows; no caller policy stays the explicit required error.
#[test]
#[ignore = "requires AA_GAME_ROOT actual original/v14 sources and completed Bank; source-only lab, no playback/World readiness"]
fn actual_missing_sequence_skips_object_animation_only_under_compatibility_mode(){
    let mut skipped=0;
    with_actual_sources(|lab,state,resources|{
        let (catalog,bank,pc)=(lab.catalog,lab.bank,lab.pc);
        for id in ["tropical_bamboothicket","tropical_grasstall"]{
            let d=catalog.resolve_registered(id,pc)?;
            let data=state.bind_object(catalog,bank,&d)?;
            let (prefix,folder,animations)=view_inputs(lab,&d)?;
            let mut owner_skips=0;
            for animation in &animations{
                let mut required=Set::from_native_constructor(&data);
                let required=required.load_source_node(catalog,bank,animation,&prefix,&folder,resources);
                let mut set=Set::from_native_constructor(&data);
                set.load_source_node_with_policy(catalog,bank,animation,&prefix,&folder,resources,&mut Compatibility{mode:1,warning:1})
                    .map_err(|e|format!("{id}: {e}"))?;
                if set.core.animations.is_empty(){
                    assert!(set.core.pending.is_none()&&set.core.by_type.iter().all(Vec::is_empty),"{id}");
                    assert!(required.as_ref().is_err_and(|e|e.contains("DataCompatibilityMode")),"{id}: {required:?}");
                    let mut strict=Set::from_native_constructor(&data);
                    let strict=strict.load_source_node_with_policy(catalog,bank,animation,&prefix,&folder,resources,&mut Compatibility{mode:0,warning:1});
                    assert!(strict.is_err_and(|e|e.contains("rethrows")),"{id}");
                    owner_skips+=1;
                }else{
                    required.map_err(|e|format!("{id}: present Sequence must not read UVar61: {e}"))?;
                }
            }
            assert!(owner_skips>0,"{id}: no missing Sequence Animation");
            skipped+=owner_skips;
        }
        Ok(())
    });
    println!("{skipped} ObjectAnimations skipped by 5A80D0 NULL under explicit DataCompatibilityMode=1");
}
