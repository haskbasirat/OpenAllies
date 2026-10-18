//!4E0F90 over the same registered Kingdom/Team allocations after the final
//!ConfigureGWorld destruction pass. Every mask store remains committed if a
//!later current identity or relation slot is unavailable.
use aa_actors::{ActorId,IdentityRegistry};
use aa_mapworld::{restore::RestoredRecord,special_state::SpecialShell};
use crate::load_kingdom_init_world::KingdomRegistryState;
type Result<T>=std::result::Result<T,String>;

fn shell(registry:&IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&SpecialShell> {
    match &registry.get(id).ok_or("relation refresh has a stale current identity")?.value {
        RestoredRecord::Kingdom(v)=>v.runtime.as_ref().and_then(|v|v.derived.as_ref()),
        RestoredRecord::Team(v)=>v.runtime.as_ref().and_then(|v|v.derived.as_ref()),
        _=>None,
    }.ok_or_else(||"relation refresh requires an actual Kingdom/Team allocation".into())
}
fn shell_mut(registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&mut SpecialShell> {
    match registry.value_mut(id).ok_or("relation refresh has a stale mutable identity")? {
        RestoredRecord::Kingdom(v)=>v.runtime.as_mut().and_then(|v|v.derived.as_mut()),
        RestoredRecord::Team(v)=>v.runtime.as_mut().and_then(|v|v.derived.as_mut()),
        _=>None,
    }.ok_or_else(||"relation refresh requires an actual mutable Kingdom/Team allocation".into())
}
fn word(s:&SpecialShell,offset:u32)->Result<u32> {
    s.words.get(&offset).copied().ok_or_else(||format!("relation refresh current+{offset:X} word is absent"))
}
fn team(s:&SpecialShell)->Result<Option<ActorId>> {
    match s.identity_references.get(&0x1ec) {
        Some(value)=>Ok(*value),
        None if word(s,0x1ec)?==0=>Ok(None),
        None=>Err("relation refresh actual Kingdom+1EC owner is absent".into()),
    }
}
fn vector(s:&SpecialShell,offset:u32,index:u32)->Result<u32> {
    s.vectors.get(&offset).ok_or("relation refresh actual relation vector is absent")?
        .values.get(index as usize).copied().ok_or_else(||"relation refresh current native slot is absent".into())
}
///Actual4E5250/+138, including the pointer-self shortcut before table access.
fn related(registry:&IdentityRegistry<RestoredRecord>,id:ActorId,other:ActorId)->Result<bool> {
    if id==other{return Ok(true)}
    let current= shell(registry,id)?;
    let target=shell(registry,other)?;
    let current_team=team(current)?;
    let target_team=team(target)?;
    let (index,kingdom_offset,team_offset)=if let Some(t)=target_team {
        (word(shell(registry,t)?,0x38)?,0x1dc,0x58)
    }else {(word(target,0x188)?,0x1d0,0x4c)};
    let value=if let Some(t)=current_team {vector(shell(registry,t)?,team_offset,index)?}
        else {vector(current,kingdom_offset,index)?};
    Ok(value==1)
}
///559600 writes only the low WORD; high bytes may have independent owners.
fn supply(s:&SpecialShell)->Result<u16> {
    if let Some(value)=s.words.get(&0x22c){return Ok(*value as u16)}
    Ok(u16::from_le_bytes([
        *s.bytes.get(&0x22c).ok_or("relation refresh current supply low byte absent")?,
        *s.bytes.get(&0x22d).ok_or("relation refresh current supply high byte absent")?,
    ]))
}
fn store_supply(s:&mut SpecialShell,value:u16)->Result<()> {
    if let Some(word)=s.words.get_mut(&0x22c){*word=(*word&0xffff0000)|u32::from(value);return Ok(())}
    if !s.bytes.contains_key(&0x22c)||!s.bytes.contains_key(&0x22d){return Err("relation refresh actual supply WORD storage absent".into())}
    let [low,high]=value.to_le_bytes();s.bytes.insert(0x22c,low);s.bytes.insert(0x22d,high);Ok(())
}
///4E0F90: whole54EBA0 precedes whole559600, never a combined reduction.
pub(crate) fn refresh(registry:&mut IdentityRegistry<RestoredRecord>,kingdoms:&KingdomRegistryState,id:ActorId)->Result<()> {
    *shell_mut(registry,id)?.words.get_mut(&0x228).ok_or("relation refresh actual vision DWORD absent")?=0;
    if word(shell(registry,id)?,0x190)?==0 {
        for &other in kingdoms.majors().iter().flatten() {
            if related(registry,id,other)?&&word(shell(registry,other)?,0x190)?==0 {
                let major=word(shell(registry,other)?,0x18c)?;
                let value=word(shell(registry,id)?,0x228)?|1u32.wrapping_shl(major&31);
                *shell_mut(registry,id)?.words.get_mut(&0x228).unwrap()=value;
                if word(shell(registry,other)?,0x190)?==0 {
                    let value=word(shell(registry,id)?,0x228)?|
                        1u32.wrapping_shl(major.wrapping_add(aa_zones::tile::VISIBLE_SHIFT)&31);
                    *shell_mut(registry,id)?.words.get_mut(&0x228).unwrap()=value;
                }
            }
        }
    }
    store_supply(shell_mut(registry,id)?,0)?;
    if word(shell(registry,id)?,0x190)?==0 {
        for &other in kingdoms.majors().iter().flatten() {
            if related(registry,id,other)?&&word(shell(registry,other)?,0x190)?==0 {
                let bit=1u32.wrapping_shl(word(shell(registry,other)?,0x18c)?&31) as u16;
                let value=supply(shell(registry,id)?)?|bit;
                store_supply(shell_mut(registry,id)?,value)?;
            }
        }
    }
    Ok(())
}
