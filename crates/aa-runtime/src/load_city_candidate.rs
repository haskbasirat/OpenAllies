//! Actual held/restored component reads for City admission. These helpers lend
//! fields from the same allocations; none constructs an actor or list snapshot.
use aa_actors::{ActorId,components::{ComponentSlot as Slot,Components}};
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::{Binary,Precision};
use crate::zone_city_admission::ValueComparison;

pub(crate) fn organization(components:&Components<RestoredComponent>,owner:ActorId)->Result<Option<ActorId>,String>{
    match components.get(Slot::Element){
        None=>Ok(None),
        Some(RestoredComponent::Element(v)) if v.header.owner==owner=>Ok(v.organization),
        _=>Err("4549C0 actual Element allocation/owner differs".into()),
    }
}

/// Native4D01E0's component order and unspilled result. Finite arithmetic reuses
/// the existing shared aa-body port; infinities/NaNs are classified only for
/// the native immediately-following comparison (no NaN payload is returned).
pub fn compare_fields(fields:&aa_body::experience::ValueFields,pc:Precision)->Result<ValueComparison,String>{
    let mut infinity=None;
    for field in [fields.regiment,fields.wall,fields.combat,fields.component88].into_iter().flatten(){
        if field.0&0x7fff_ffff>0x7f80_0000{return Ok(ValueComparison::Unordered)}
        if field.0&0x7fff_ffff==0x7f80_0000{
            let negative=field.0>>31!=0;
            if infinity.is_some_and(|old|old!=negative){return Ok(ValueComparison::Unordered)}
            infinity=Some(negative);
        }
    }
    if let Some(negative)=infinity{return Ok(if negative{ValueComparison::Less}else{ValueComparison::Greater})}
    let value=aa_body::experience::actor_value(fields,pc).map_err(|e|e.to_string())?;
    Ok(match value.compare(&Binary::from_u32(0)){
        std::cmp::Ordering::Less=>ValueComparison::Less,
        std::cmp::Ordering::Equal=>ValueComparison::Equal,
        std::cmp::Ordering::Greater=>ValueComparison::Greater,
    })
}

pub(crate) fn value(components:&Components<RestoredComponent>,owner:ActorId,pc:Precision)->Result<ValueComparison,String>{
    let regiment=match components.get(Slot::Organization){
        None=>None,Some(RestoredComponent::Organization(v))if v.header.owner==owner=>Some(aa_body::F32(*v.words.get(&0x6c).ok_or("578C00 actual Organization+6C uninitialized")?)),
        _=>return Err("4D01E0 actual Organization owner differs".into()),
    };
    //No Settlement allocation is admitted by the current shared payload enum.
    //A positive slot cannot be silently treated as an absent component.
    let wall=match components.get(Slot::Settlement){None=>None,Some(_)=>return Err("required native service:4D01E0 typed Settlement+34 owner".into())};
    let combat=match components.get(Slot::Performer){
        None=>None,Some(RestoredComponent::Performer(v))if v.state.owner==owner.raw()=>Some(aa_body::F32(v.word60)),
        _=>return Err("4D01E0 actual Performer owner differs".into()),
    };
    let component88=match components.get(Slot::Denizen){
        None=>None,Some(RestoredComponent::Denizen(v))if v.header.owner==owner=>Some(aa_body::F32(*v.words.get(&0x14).ok_or("4D01E0 actual Denizen+14 uninitialized")?)),
        _=>return Err("4D01E0 actual Denizen owner differs".into()),
    };
    compare_fields(&aa_body::experience::ValueFields{regiment,wall,combat,component88},pc)
}

pub(crate) fn body_present(components:&Components<RestoredComponent>,owner:ActorId)->Result<bool,String>{
    match components.get(Slot::Body){None=>Ok(false),Some(RestoredComponent::Body(v))if v.header.owner==owner=>Ok(true),_=>Err("564110 actual Body allocation/owner differs".into())}
}

pub(crate) fn source_byte(catalog:&crate::catalog::Catalog<'_>,definition:&aa_mapworld::restore::RuntimeDefinition,offset:u32)->Result<u8,String>{
    let source=crate::live_object_init::Source::bind(catalog,definition).map_err(|e|e.to_string())?;
    let body=catalog.actors.native_ground_definition(&definition.proof.id,source.source_precision())
        .map_err(|e|e.to_string())?.attributes.body.ok_or("City actual Body source fields unavailable")?;
    let field=match offset{0x290=>body.captureable,0x291=>body.destructible,_=>return Err("City unknown reached source byte".into())};
    match field{aa_rules::native::TypedValue::Boolean{value,..}=>Ok(u8::from(value)),_=>Err("City actual Body flag source type differs".into())}
}
