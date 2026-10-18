//!Immutable47E0F0 Color+10 projection from the SAME source Bank and retained
//!definition-time text. MAPSTRINGS/current text is not consulted here.
use crate::{actor_world::{ActorStore,Source,SpecialSource},catalog::Catalog,definitions::Bank,localization::Localization};
use aa_actors::ActorId;
type Result<T>=std::result::Result<T,String>;

pub fn kingdom_color_name(catalog:&Catalog<'_>,bank:&Bank,actors:&ActorStore,text:&Localization,id:ActorId)->Result<Vec<u16>> {
    catalog.require_source_owner()?;
    let source=bank.combat_source()?;source.completed_means()?;
    if source.source_identities()!=catalog.actors.sources{return Err("7515A0 Color Bank/Catalog source owner differs".into());}
    let key=actors.key(id).ok_or("7515A0 current Kingdom generation is absent")?;
    let entity=actors.entity(key).map_err(|e|e.to_string())?;
    let Source::Special(s)=&entity.source else{return Err("7515A0 requires actual stored Kingdom allocation".into())};
    if !matches!(&s.source,SpecialSource::Kingdom(k)if k.serialized_id==id.raw())||!s.derived_payload_loaded {
        return Err("7515A0 actual Kingdom derived allocation is incomplete".into());
    }
    let object=actors.object(key).map_err(|e|e.to_string())?;
    let resolved=aa_mapworld::bootstrap::resolve_definition(catalog.actors,&object.thing.definition.id.encode_utf16().collect::<Vec<_>>(),source.source_precision()).map_err(|e|e.to_string())?;
    let a=&object.thing.definition;let b=&resolved.definition;
    let runtime=&object.definition.proof;
    if object.kind!=aa_actors::loading::LoadKind::Actor||object.definition.native_class!=4||resolved.native_class!=4
        ||object.actor.is_none()||object.object.is_none()||a.id!=b.id||a.source!=b.source||a.template_chain!=b.template_chain
        ||a.category!=b.category||a.base_flag_inputs!=b.base_flag_inputs
        ||runtime.id!=a.id||runtime.source!=a.source||runtime.template_chain!=a.template_chain
        ||runtime.category!=a.category||runtime.base_flag_inputs!=a.base_flag_inputs {
        return Err("7515A0 actual Kingdom class/source proof differs".into());
    }
    let shell=s.derived.as_ref().ok_or("7515A0 current Kingdom fields absent")?;
    let color=shell.references.get(&0x1e8).ok_or("7515A0 current Color1E8 is unbound")?.as_ref().ok_or("7515A0 would dereference null Color1E8")?;
    let registry=catalog.actors.native_data_registry("KingdomColor").ok_or("7515A0 native Color registry absent")?;
    let mut matches=registry.entries.iter().enumerate().filter(|(_,entry)|entry.ids.eq_ignore_ascii_case(&color.id));
    let(index,entry)=matches.next().ok_or("7515A0 Color source allocation is absent")?;
    if matches.next().is_some()||!entry.reloads.is_empty()||color.index.is_some_and(|n|n as usize!=index){
        return Err("7515A0 Color allocation is ambiguous/reloaded or has a foreign ordinal".into());
    }
    let node=catalog.actors.native_data_node(entry).map_err(|e|e.to_string())?;
    //Color47E0F0 and Thing47D960 call the same5B5620(name) primitive.
    aa_rules::native::display_name::native_stored_name(&node,&mut|key|{
        let value=text.lookup_bound(key,catalog.actors,source.source_precision()).map_err(|e|aa_rules::native::Error::Unsupported(e.to_string()))?;
        value.map(Some).ok_or_else(||aa_rules::native::Error::Unsupported("7515A0 definition-time Color name requires actual4B9F40 missing-key handling".into()))
    }).map_err(|e|e.to_string())
}
