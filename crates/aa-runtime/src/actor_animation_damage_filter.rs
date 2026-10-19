//!461F70's death-Animation damage_filter check (4620A6..4620EC).
//!
//!Native order: 461F70 stores Animation+28 = GetInt("damage_filter") first.
//!For a death Animation (+8 == 3) whose filter is neither -1 nor 666 (0x29A)
//!it walks the DamageType DataManager [B36530]+3A4 (481410, vtable A0CFE4,
//!capacity 0xB, mode 0 as pushed at 48AA71) vector +24 for i in 0..+28 and
//!compares each DamageTypeData+2C with the filter. A hit jumps to 4620EF and
//!stores NOTHING else; a miss throws 410C40 "Animation '%s' for actor '%s'
//!specifies an invalid damage filter '%d'" (Animation+C, ActorData+8 IDS,
//!filter). Animation+28 keeps the authored value either way; no registry
//!index or DamageType handle is retained by this source step.
//!
//!DamageTypeData+2C is written only by its LoadData 4C0A80: 47D960 base
//!fields, 5B4C30 color, 5B54D0 glyph + 48D500 on the Glyph manager
//![B36530]+3BC (zero throws BEFORE +2C is stored; 4C0870 never initializes
//!+2C), then +2C = GetInt("damage_filter") and a negative value throws AFTER
//!the store. 481630 registers each entry (index +C = prior +28 count) before
//!4C0A80 runs, and a throw makes 48D660 abandon the rest of that file.
//!
//!ActorData LoadData 464BD0 (A05540[0]) runs from 4BDC50 while the Actor's
//!own file is processed, so the vector holds exactly the DamageTypes of files
//!loaded before that file. This reuses the canonical native-order generic
//!DataManager registry (`ActorCatalog::native_data_registry`) and the
//!catalog's file load order; every state it does not model is an error.
use aa_numeric::Precision;
use aa_rules::{actors::{ActorCatalog,native_load::NativeDataEntry},native::TypedValue,resources::Provenance};
type Result<T>=std::result::Result<T,String>;
#[cfg(test)]
#[path="actor_animation_damage_filter_tests.rs"]
mod tests;

///One registered DamageTypeData as 461F70 reads it.
#[derive(Clone,Debug,PartialEq,Eq)]
pub(crate) struct Entry{
    ///+8 IDS as authored by the registering node.
    pub ids:String,
    ///+C registry index written by 481630 (= +28 before the append).
    pub index:u32,
    ///+2C stored by 4C0A80.
    pub filter:i32,
}
///[B36530]+3A4 vector +24/+28 as seen by one ActorData's 464BD0.
#[derive(Clone,Debug,PartialEq,Eq)]
pub(crate) struct Registry{pub entries:Vec<Entry>}
impl Registry{
    ///The DamageType vector at the moment `actor`'s file is processed.
    pub(crate) fn at_actor_load(actors:&ActorCatalog,actor:&Provenance,pc:Precision)->Result<Self>{
        let manager=actors.native_data_registry("DamageType").ok_or("461F70 DamageType DataManager [B36530]+3A4 is unbound")?;
        if manager.mode!=Some(0){return Err("461F70 DamageType DataManager+8 differs from 48AA71 mode0".into())}
        let actor_file=file_position(actors,actor)?;
        let mut entries=Vec::new();let mut previous=0usize;
        for entry in &manager.entries{
            let file=file_position(actors,&entry.source)?;
            if file<previous{return Err("DamageType registration order differs from file load order".into())}
            previous=file;
            if file>actor_file{continue}
            if file==actor_file{return Err("DamageType declared in the Actor's own file: 4BDC50 root order is not modeled".into())}
            let filter=loaded_filter(actors,entry,file,pc)?;
            entries.push(Entry{ids:entry.ids.clone(),index:entries.len() as u32,filter});
        }
        Ok(Self{entries})
    }
    ///4620AE..4620D4. `true` is the 4620EF hit edge.
    pub(crate) fn contains(&self,filter:i32)->bool{lookup(self.entries.iter().map(|e|e.filter),filter)}
}
///4620BA..4620D4 over +2C values in +24 order: first equal value wins; an
///empty vector (+28 <= 0) misses without reading +24.
pub(crate) fn lookup(filters:impl IntoIterator<Item=i32>,filter:i32)->bool{filters.into_iter().any(|value|value==filter)}
///4620D6..4620E7 410C40 diagnostic (thrown; Animation stays appended).
pub(crate) fn invalid_filter(animation:&str,actor:&str,filter:i32)->String{
    format!("410C40 Animation '{animation}' for actor '{actor}' specifies an invalid damage filter '{filter}'")
}
///4C0A80 up to and including the +2C store, on the registered node.
fn loaded_filter(actors:&ActorCatalog,entry:&NativeDataEntry,file:usize,pc:Precision)->Result<i32>{
    //A mode0 duplicate throws in 481630 and is never re-loaded.
    let node=actors.native_data_node(entry).map_err(|e|e.to_string())?;
    match node.field("color",pc).map_err(|e|e.to_string())?{
        Some(TypedValue::Point{bits,..})if bits.len()==3=>{}
        _=>return Err(format!("4C0A80 DamageType '{}' requires native point3 color before +2C",entry.ids)),
    }
    let Some(TypedValue::String{value:glyph,..})=node.field("glyph",pc).map_err(|e|e.to_string())? else{
        return Err(format!("4C0A80 DamageType '{}' requires native String glyph before +2C",entry.ids))
    };
    if !glyph_registered(actors,&glyph,file)?{
        return Err(format!("4C0A80 Unknown special glyph '{glyph}' loading damage type '{}': +2C is never stored and the file is abandoned",entry.ids))
    }
    let Some(TypedValue::Integer{value,..})=node.field("damage_filter",pc).map_err(|e|e.to_string())? else{
        return Err(format!("4C0A80 DamageType '{}' requires native Int damage_filter",entry.ids))
    };
    if value<0{
        return Err(format!("4C0A80 Damage type '{}' specifies an invalid damage_filter '{value}': stored +2C then thrown; the abandoned file remainder is not modeled",entry.ids))
    }
    Ok(value)
}
///48D500 on [B36530]+3BC as it stood while the DamageType file loaded.
fn glyph_registered(actors:&ActorCatalog,glyph:&str,damage_file:usize)->Result<bool>{
    let glyphs=actors.native_data_registry("Glyph").ok_or("4C0A80 Glyph DataManager [B36530]+3BC is unbound")?;
    if glyphs.translations().iter().any(|(old,_)|old.eq_ignore_ascii_case(glyph)){
        return Err("4C0A80 Glyph translation timing relative to the DamageType file is not modeled".into())
    }
    let Some(found)=glyphs.find(glyph) else{return Ok(false)};
    let file=file_position(actors,&found.source)?;
    if file==damage_file{return Err("Glyph declared in the DamageType file: root order is not modeled".into())}
    Ok(file<damage_file)
}
///Position of a parsed source file in the catalog's 48D970 load order.
fn file_position(actors:&ActorCatalog,source:&Provenance)->Result<usize>{
    let mut found=actors.sources.iter().enumerate().filter(|(_,s)|s.path==source.path&&s.sha256==source.sha256).map(|(i,_)|i);
    let first=found.next().ok_or_else(||format!("{} is not a loaded native source file",source.path))?;
    if found.next().is_some(){return Err(format!("{} loaded more than once: registration time is not modeled",source.path))}
    Ok(first)
}
