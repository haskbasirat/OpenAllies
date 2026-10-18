//! Native76D6C0 diagnostic traversal over the actual retained LaunchSettings
//! allocations. Logging being disabled does not suppress the source reads.
//! CString reference counts, allocation failure and file-output are outside
//! this value traversal. Unsupported positive source companions stop lazily.
use crate::{catalog::Catalog, creator_labels, game_settings_value,
    live_clocked::SyncBinding, load_world_rules::CreatorRules,
    resource_creator::{Creator, launch_creators::Others}};
use aa_mapworld::header::WorldParameters;
type Result<T> = std::result::Result<T, String>;

///The same AppLaunchState allocation. Offset0C is never read by784560.
pub trait AppRead {
    fn word(&self, offset:u32)->Result<u32>;
    fn byte(&self, offset:u32)->Result<bool>;
    fn failure_text_8(&self)->Result<&[u16]>;
}
///Successful76E540 constructs each pointer below nonnull. An unbound retained
///owner must be refused, rather than inferred to be a native null pointer.
pub trait LaunchRead {
    fn session_state_0(&self)->Result<u32>;
    fn map_type_4(&self)->Result<u32>;
    fn filename_8(&self)->Result<&[u16]>;
    fn launch_type_0c(&self)->Result<u32>;
    fn header_14(&self)->Result<&game_settings_value::Owner>;
    fn load_1c(&self)->Result<&Creator>;
    fn others(&self)->Result<&Others>;
    fn app_2c(&self)->Result<&dyn AppRead>;
}
pub trait Sink {
    fn marker(&mut self,label:&str)->Result<()>;
    fn int(&mut self,label:&str,word:u32)->Result<()>;
    fn float(&mut self,label:&str,bits:u32)->Result<()>;
    fn boolean(&mut self,label:&str,value:bool)->Result<()>;
    fn text(&mut self,label:&str,units:&[u16])->Result<()>;
}
///Borrow the one SyncA. Its thread/application gate is evaluated separately
///at each logger, only after enabled/suspend; no second synchronizer is made.
pub struct SyncSink<'a,'b> { pub binding:&'a mut SyncBinding<'b> }
impl SyncSink<'_,'_> {
    fn with(&mut self,f:impl FnOnce(&mut aa_orders::net::sync::Synchronizer,bool))->Result<()> {
        fn log(owner:&mut aa_orders::net::sync::Synchronizer,gate:&mut dyn FnMut()->Result<bool>,f:impl FnOnce(&mut aa_orders::net::sync::Synchronizer,bool))->Result<()> {
            let open=if owner.enabled && owner.suspend<1 {gate()?}else{false};
            f(owner,open);Ok(())
        }
        match self.binding {
            SyncBinding::Direct(v)=>log(v.owner,v.gate,f),
            SyncBinding::Shared{owner,gate}=>{let mut loan=owner.try_borrow_mut().map_err(|e|format!("same launch-dump SyncA loan: {e}"))?;log(&mut loan,*gate,f)},
        }
    }
}
impl Sink for SyncSink<'_,'_> {
    fn marker(&mut self,l:&str)->Result<()> {self.with(|s,g|s.log_marker(g,l))}
    fn int(&mut self,l:&str,v:u32)->Result<()> {self.with(|s,g|s.log_int(g,l,v))}
    fn float(&mut self,l:&str,v:u32)->Result<()> {self.with(|s,g|s.log_float_bits(g,l,v))}
    fn boolean(&mut self,l:&str,v:bool)->Result<()> {self.with(|s,g|s.log_bool(g,l,v))}
    fn text(&mut self,l:&str,v:&[u16])->Result<()> {self.with(|s,g|s.log_text(g,l,v))}
}
fn count(n:usize)->Result<u32> {u32::try_from(n).map_err(|_|"native diagnostic count exceeds wire word".into())}
pub(crate) fn definition_index(c:&Catalog<'_>,manager:&str,ids:&[u16])->Result<u32> {
    if ids.is_empty() {return Ok(0)}
    let end=ids.iter().position(|&v|v==0).unwrap_or(ids.len());
    let name=String::from_utf16(&ids[..end]).map_err(|e|e.to_string())?;
    let registry=c.actors.native_data_registry(manager).ok_or_else(||format!("diagnostic {manager} manager absent"))?;
    if !matches!(registry.mode,Some(0|1)) {return Err(format!("diagnostic {manager} predefined/unbound Data+C producer"))}
    let entry=registry.find(&name).ok_or_else(||format!("diagnostic {manager} reference {name} is absent"))?;
    c.actors.native_data_node(entry).map_err(|e|e.to_string())?;
    count(registry.entries.iter().position(|v|std::ptr::eq(v,entry)).ok_or("diagnostic canonical entry outside manager")?)
}
fn rules(c:&Catalog<'_>,r:&CreatorRules,s:&mut impl Sink,label:&str)->Result<()> {
    s.marker(label)?;
    s.int("_wr.m_map_type",r.mode())?;
    s.int("_wr.m_game_type",definition_index(c,"GameType",r.game_type())?)?;
    s.int("(int)size",count(r.records().count())?)?;
    for record in r.records() {
        s.marker("(*iter)")?;
        s.int("_wr.data",definition_index(c,"WorldRule",&record.ids().encode_utf16().collect::<Vec<_>>())?)?;
        s.boolean("_wr.locked",record.locked()!=0)?;
        match record.kind() {
            0=>s.boolean("_wr.value_bool",record.value() as u8!=0)?,
            1=>s.int("_wr.value_int",record.value())?,
            2=>s.float("_wr.value_fixed",record.value())?,
            3=>s.int("_wr.value_enum",record.value())?,
            _=>return Err("native WorldRule diagnostic kind outside proven descriptor domain".into()),
        }
    }
    Ok(())
}
fn parameters(c:&Catalog<'_>,p:&WorldParameters,l:&creator_labels::State,closed30:&[u8],s:&mut impl Sink,label:&str)->Result<()> {
    s.marker(label)?;
    s.int("(int)size",count(p.teams.len())?)?;
    for (i,t) in p.teams.iter().enumerate() {
        s.marker("(*iter)")?;s.text("_ti.ids",&t.id)?;
        s.text("_ti.name",l.team_pair(p,i)?.text())?;
    }
    s.int("(int)size",count(p.kingdoms.len())?)?;
    for (i,k) in p.kingdoms.iter().enumerate() {
        s.marker("(*iter)")?;s.text("_ki.ids",&k.id)?;
        s.text("_ki.name",l.kingdom_pair(p,i)?.text())?;
        s.boolean("_ki.independent",k.byte_10!=0)?;s.text("_ki.team_ids",&k.string_14)?;
        for (label,manager,ids) in [("_ki.nation","Nation",&k.nation),("_ki.faction","Faction",&k.faction),("_ki.color","KingdomColor",&k.color),("_ki.handicap","Handicap",&k.handicap)] {
            s.int(label,definition_index(c,manager,ids)?)?;
        }
        s.boolean("_ki.playable",k.byte_28!=0)?;s.boolean("_ki.required",k.byte_29!=0)?;
        s.text("_ki.sai_default_ids",&k.string_2c)?;
        s.boolean("_ki.closed",*closed30.get(i).ok_or("actual KingdomParameter+30 companion index differs")?!=0)?;
        s.int("(int)size",count(k.localized_entries.len())?)?;
        for index in 0..k.localized_entries.len() {s.text("(*iter)",l.objective_pair(p,i,index)?.text())?;}
        s.int("(int)size",count(k.definitions.len())?)?;
        for ids in &k.definitions {s.int("(*iter)",definition_index(c,"Property",ids)?)?;}
        s.text("_ki.merge_into_ids",&k.string_58)?;
        s.int("(int)size",count(k.properties.len())?)?;
        if !k.properties.is_empty() {return Err("50D640 actual additional-Actor ThingData+C owner is unbound".into())}
        s.boolean("_ki.remove_kingdom",k.byte_31!=0)?;s.boolean("_ki.is_sai",k.byte_32!=0)?;
        s.int("(int)size",count(k.resources.len())?)?;
        for (_,bits) in &k.resources {s.float("(*iter)",*bits)?;}
    }
    Ok(())
}
fn common(c:&Catalog<'_>,v:&Creator,s:&mut impl Sink)->Result<()> {
    s.marker("(KKC_WorldCreator &)_wc")?;
    rules(c,v.rules(),s,"_wc.m_world_rules")?;
    parameters(c,v.parameters(),v.labels(),v.closed30(),s,"_wc.m_world_parameters")?;
    s.float("_wc.m_width",v.extents_bits()[0])?;s.float("_wc.m_height",v.extents_bits()[1])?;
    s.int("_wc.m_biome",v.diagnostic_reference_index(c,"Biome")?)?;
    s.int("_wc.m_weather",v.diagnostic_reference_index(c,"Weather")?)?;Ok(())
}
fn header(c:&Catalog<'_>,v:&game_settings_value::Owner,s:&mut impl Sink)->Result<()> {
    s.text("_map_header.m_name",v.name_pair()?.text())?;
    s.text("_map_header.m_description",v.description_pair()?.text())?;
    s.int("(int)_map_header.m_checksum",v.words_0c_10()[1])?;
    s.int("(int)_map_header.m_version",v.words_0c_10()[0])?;
    parameters(c,v.parameters(),v.labels(),v.closed30(),s,"_map_header.m_world_parameters.GetRef()")?;
    rules(c,v.rules(),s,"_map_header.m_world_rules.GetRef()")?;
    s.float("_map_header.m_map_width",v.display_extents_bits()?[0])?;
    s.float("_map_header.m_map_height",v.display_extents_bits()?[1])?;
    if v.words_0c_10()[0]>1 {
        s.text("_map_header.m_campaign_ids",v.string_3c())?;
        s.int("_map_header.m_campaign_mission_number",v.word_40()?)?;
        s.text("_map_header.m_source_filename",v.string_38())?;
        s.int("(int)_map_header.m_time_low",v.words_18_1c()[0])?;
        s.int("(int)_map_header.m_time_high",v.words_18_1c()[1])?;
        for entry in v.entries() {
            s.boolean("p->isSAI",entry.flag!=0)?;
            s.text("p->kingdom_ids",&entry.string_20)?;s.text("p->player_name",&entry.string_18)?;
            s.boolean("p->is_local_player",entry.flag_v5.ok_or("Header entry+5 old-version constructor producer is unbound")?!=0)?;
        }
    }Ok(())
}
fn app(v:&dyn AppRead,s:&mut impl Sink)->Result<()> {
    s.int("_app_data.m_app_state",v.word(0)?)?;
    s.boolean("_app_data.m_bypass_staging",v.byte(4)?)?;
    s.text("_app_data.m_create_failed_notification_message",v.failure_text_8()?)?;
    s.int("_app_data.m_session_type",v.word(0x10)?)?;
    s.boolean("_app_data.m_returning",v.byte(0x14)?)?;
    s.int("_app_data.m_end_mission",v.word(0x24)?)?;
    s.boolean("_app_data.m_multiplayer",v.byte(0x15)?)?;s.boolean("_app_data.m_internet",v.byte(0x16)?)?;
    s.int("_app_data.m_next_app_state",v.word(0x18)?)?;
    s.int("_app_data.m_interface_mode",v.word(0x1c)?)?;
    s.boolean("_app_data.m_from_test",v.byte(0x20)?)?;Ok(())
}
///76D6C0 itself does not log its incoming label. All sibling pointers in this
///successful-construction domain are real/non-null and traversed even if
///SyncA is disabled. Failure retains every preceding sink effect.
pub fn dump(c:&Catalog<'_>,v:&impl LaunchRead,s:&mut impl Sink,_label:&str)->Result<()> {
    s.int("_settings.m_session_state",v.session_state_0()?)?;
    s.int("_settings.m_map_type",v.map_type_4()?)?;s.text("_settings.m_filename",v.filename_8()?)?;
    header(c,v.header_14()?,s)?;
    s.marker("*_settings.m_wcblank")?;common(c,v.others()?.blank().creator(),s)?;
    s.boolean("_wc.m_doodify",v.others()?.blank().doodify())?;
    s.marker("*_settings.m_wcload")?;let load=v.load_1c()?;common(c,load,s)?;
    match load.source_resource() {Some(source)=>s.text("_wc.m_map_resource->KKC_Resource::GetFilename()",source.ids())?,None=>s.text("g_empty_string",&[])?}
    s.marker("*_settings.m_wcrandom")?;let random=v.others()?.random();common(c,random.creator(),s)?;
    if !random.config_is_null() {return Err("Random actual configuration IDS owner is unbound".into())}
    s.text("g_empty_string",&[])?;s.marker("_wc.m_rmc_settings")?;
    s.float("_settings.m_total_terrain_weight",random.total_terrain_weight_bits())?;
    for values in [random.terrain_weights(),random.object_counts()] {
        s.int("(int)size",count(values.len())?)?;
        for (ids,bits) in values {s.marker("(*iter)")?;s.text("_value.ids",ids)?;s.float("_value.value",*bits)?;}
    }
    s.int("_settings.m_num_heroes",random.num_heroes())?;
    s.marker("*_settings.m_wcpreset")?;common(c,v.others()?.preset().creator(),s)?;
    s.marker("*_settings.m_wcmg")?;common(c,v.others()?.metagame().creator(),s)?;
    s.int("_settings.m_launch_type",v.launch_type_0c()?)?;app(v.app_2c()?,s)
}
#[cfg(test)]
#[path="launch_settings_dump_tests.rs"]
mod tests;
