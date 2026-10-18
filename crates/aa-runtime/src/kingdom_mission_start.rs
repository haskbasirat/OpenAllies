//! Native4E6DB0 over the actual retained SpecialShell stockpile vectors.
//! The no-Commander / null-start-profile / empty-start-technology path is
//! concrete. Reached positive property/profile services remain required.
//! Source validation is preflight; writes after it preserve the native prefix.
use crate::{catalog::Catalog,definitions::Bank,load_view_variables};
use aa_economy::{F32Bits,X87Precision};
use aa_mapworld::special_state::SpecialShell;
use aa_numeric::Precision;

type Result<T>=std::result::Result<T,String>;

trait Values {
    fn starting_stockpile(&mut self)->Result<u32>;
    fn maximum(&mut self)->Result<u32>;
    fn commander_properties(&mut self,_definition:u32)->Result<Vec<u32>> {Err("actual Commander Faction property list is required".into())}
    fn add_property(&mut self,_kingdom:&mut SpecialShell,_definition:u32)->Result<()> {Err("4E3AD0 actual staged Kingdom Property grant is required".into())}
    fn faction_allowed(&mut self,_kingdom:&SpecialShell)->Result<bool> {Err("4730D0 actual Commander/Nation definitions are required".into())}
    fn invalid_faction(&mut self)->Result<()> {Err("401B00/4E4BC0 actual invalid-Faction localization/message service is required".into())}
    fn starting_experience(&mut self)->Result<u32> {Err("actual current StartingExperience134 SVar required".into())}
}
struct Current<'a,'source>{current:&'a load_view_variables::State,catalog:&'a Catalog<'source>,bank:&'a Bank}
impl Values for Current<'_,'_> {
    fn starting_stockpile(&mut self)->Result<u32> {Ok(self.current.stockpile_default_bits())}
    fn maximum(&mut self)->Result<u32> {Ok(self.current.stockpile_maximum_bits())}
    fn commander_properties(&mut self,definition:u32)->Result<Vec<u32>> {crate::kingdom_start_source::properties(self.catalog,self.bank,definition)}
    fn faction_allowed(&mut self,kingdom:&SpecialShell)->Result<bool> {
        let nation=match kingdom.references.get(&0x234){Some(v)=>v.as_ref(),None if kingdom.words.get(&0x234)==Some(&0)=>None,_=>return Err("Kingdom actual Nation pointer is unbound".into())};
        crate::kingdom_start_source::allowed(self.catalog,self.bank,nation,kingdom.commander.as_ref().map(|v|v.definition))
    }
    fn starting_experience(&mut self)->Result<u32> {Ok(self.current.starting_experience_bits())}
}
fn byte(shell:&SpecialShell,offset:u32)->Result<u8> {
    if let Some(word)=shell.words.get(&(offset&!3)) {return Ok((word>>((offset&3)*8))as u8)}
    shell.bytes.get(&offset).copied().ok_or_else(||format!("Kingdom StartMission actual+{offset:X} byte absent"))
}
fn store_byte(shell:&mut SpecialShell,offset:u32,value:u8) {
    if let Some(word)=shell.words.get_mut(&(offset&!3)) {
        let shift=(offset&3)*8;*word=(*word&!(0xff<<shift))|((value as u32)<<shift);
    }else {shell.bytes.insert(offset,value);}
}
fn pc(p:Precision)->X87Precision {match p {Precision::Single24=>X87Precision::Single24,Precision::Double53=>X87Precision::Double53,Precision::Extended64=>X87Precision::Extended64}}

/// SAME mutable stockpile allocation, SAME current SVar owner and SAME source
/// Bank. No Economy clone or copy-back can lose intervening effects.
pub fn start_existing(catalog:&Catalog<'_>,bank:&Bank,current:&load_view_variables::State,kingdom:&mut SpecialShell,precision:Precision)->Result<()> {
    current.validate_in_bank(catalog,bank)?;
    current.require_published()?;
    start(kingdom,&mut Current{current,catalog,bank},precision)
}

fn start(kingdom:&mut SpecialShell,values:&mut impl Values,precision:Precision)->Result<()> {
    if byte(kingdom,0x23a)?!=0 {
        // Native reads+110 once, then+1B8 anew AFTER each first stock store.
        let raw=values.starting_stockpile()?;
        let initial=if f32::from_bits(raw)>0.0 {raw}else{0};
        let mut i=0;
        while i<kingdom.vectors.get(&0x1c4).ok_or("Kingdom stockpile owner is absent")?.values.len() {
            kingdom.vectors.get_mut(&0x1c4).unwrap().values[i]=initial;
            let maximum=values.maximum()?;
            let value=kingdom.vectors.get(&0x1c4).unwrap().values[i];
            kingdom.vectors.get_mut(&0x1c4).unwrap().values[i]=aa_economy::clamp_stockpile_upper(F32Bits(value),F32Bits(maximum)).0;
            i+=1;
        }
    }
    // Native reads the Commander's source property list before credit258.
    if let Some(commander)=&kingdom.commander {
        for property in values.commander_properties(commander.definition)? {values.add_property(kingdom,property)?;}
    } else if kingdom.words.get(&0x230)!=Some(&0) {
        return Err("Kingdom Commander pointer is neither a retained owner nor proved constructor null".into());
    }
    let mut i=0;
    while i<kingdom.vectors.get(&0x1c4).ok_or("Kingdom stockpile owner is absent")?.values.len() {
        let delta=*kingdom.vectors.get(&0x258).ok_or("Kingdom starting-resources owner is absent")?.values.get(i)
            .ok_or("Kingdom starting-resource vector is shorter than reached stockpile vector")?;
        let old=kingdom.vectors.get(&0x1c4).unwrap().values[i];
        let first=aa_economy::credit_stockpile_sum(F32Bits(old),F32Bits(delta),pc(precision)).map_err(|e|e.to_string())?;
        kingdom.vectors.get_mut(&0x1c4).unwrap().values[i]=first.0;
        let maximum=values.maximum()?;
        let current=kingdom.vectors.get(&0x1c4).unwrap().values[i];
        kingdom.vectors.get_mut(&0x1c4).unwrap().values[i]=aa_economy::clamp_stockpile_upper(F32Bits(current),F32Bits(maximum)).0;
        i+=1;
    }
    match kingdom.references.get(&0x244) {
        Some(None)=>{},
        Some(Some(_))=>return Err("4E6DB0 positive start profile requires actual resource/property owner".into()),
        None=>match kingdom.words.get(&0x244) {
            Some(0)=>{},
            Some(_)=>return Err("4E6DB0 positive start profile requires actual resource/property owner".into()),
            None=>return Err("Kingdom actual start-profile pointer is unbound".into()),
        },
    }
    let technologies=kingdom.vectors.get(&0x24c).ok_or("Kingdom actual start-technology vector is absent")?;
    if technologies.values.iter().any(|&v|v!=0) {
        return Err("4E6DB0 positive start technology requires actual5032D0/property grant".into());
    }
    if kingdom.commander.is_some() {
        if !values.faction_allowed(kingdom)? {values.invalid_faction()?;}
        let experience=values.starting_experience()?;
        //The existing4CDF10 kernel updates the SAME Commander+0C owner.
        aa_economy::kingdom_start::set_commander_experience(&mut kingdom.commander.as_mut().unwrap().experience,F32Bits(experience));
    }
    kingdom.words.insert(0x19c,0);
    store_byte(kingdom,0x23b,1);
    Ok(())
}

#[cfg(test)]
#[path="kingdom_mission_start_tests.rs"]
mod tests;
