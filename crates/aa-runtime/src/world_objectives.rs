//! Actual World+250/254 MapString objects, loaded from OBJECTIVES between
//! TEAMS and KINGDOMS. These are distinct from MAPSTRINGS and Trigger strings.
use crate::{catalog::Catalog,creator_labels::LocalizedPairRef};
use aa_mapworld::{Chunk,MapArchive};
use std::sync::{Arc,Weak};
type Result<T> = std::result::Result<T,String>;
struct Objective {key:Vec<u16>,display_key:Vec<u16>,display_text:Vec<u16>,flag:u8}
impl Objective {fn constructor()->Self {Self {key:Vec::new(),display_key:Vec::new(),display_text:Vec::new(),flag:0}}}
enum Phase {Constructed,Loading,Loaded,Failed}
///One move-only native array plus an unpublished allocated object's retained
///prefix. Capacity five is a native allocation hint, never an objective limit.
pub struct State {
    records:Vec<Objective>,word_25c:u32,inflight:Option<Objective>,phase:Phase,
    input:Option<std::result::Result<Option<Chunk>,String>>,
    identity:Arc<()>,
}

#[cfg(test)]
#[path="world_objectives_tests.rs"]
mod tests;
pub struct Handle {identity:Weak<()>,index:usize}
pub struct Reference<'a> {row:&'a Objective}
impl Reference<'_> {
    pub fn key(&self)->&[u16] {&self.row.key}
    pub fn display(&self)->LocalizedPairRef<'_> {LocalizedPairRef::new(&self.row.display_key,&self.row.display_text)}
    pub fn flag(&self)->u8 {self.row.flag}
}
fn prefix(s:&[u16])->&[u16] {&s[..s.iter().position(|&v|v==0).unwrap_or(s.len())]}
struct Reader<'a> {data:&'a[u8],at:usize}
impl Reader<'_> {
    fn bytes(&mut self,n:usize)->Result<&[u8]> {
        let end=self.at.checked_add(n).ok_or("OBJECTIVES stream position overflow")?;
        let bytes=self.data.get(self.at..end).ok_or("OBJECTIVES stream is truncated")?;
        self.at=end;Ok(bytes)
    }
    fn word(&mut self)->Result<u32> {Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))}
    fn string(&mut self)->Result<Vec<u16>> {
        let count=u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()) as usize;
        Ok(self.bytes(count*2)?.chunks_exact(2).map(|b|u16::from_le_bytes([b[0],b[1]])).collect())
    }
}
impl State {
    pub(crate) fn from_world_constructor()->Self {
        Self {records:Vec::new(),word_25c:0,inflight:None,phase:Phase::Constructed,input:None,identity:Arc::new(())}
    }
    ///Immutable IO/validation result only. Any error is retained until native
    ///4FCB3B is reached; this does not publish or initialize an objective.
    pub(crate) fn bind_source(&mut self,map:&MapArchive,world_version:u32)->Result<()> {
        if self.input.is_some()||!matches!(self.phase,Phase::Constructed){return Err("OBJECTIVES input repeated".into())}
        self.input=Some(if world_version<6 {Ok(None)} else {map.decode_chunk("OBJECTIVES",u64::MAX).map(Some).map_err(|e|e.to_string())});
        Ok(())
    }
    pub(crate) fn load_after_teams(&mut self,catalog:&Catalog<'_>)->Result<()> {
        self.load_with_lookup(|key| {
            let bank=catalog.definition_bank.try_borrow().map_err(|_|"OBJECTIVES actual source Bank borrowed")?;
            let source=bank.combat_source()?;
            source.completed_means()?;
            if source.source_identities()!=catalog.actors.sources {return Err("OBJECTIVES source Bank/Catalog differ".into())}
            catalog.with_current_text(|text|text.lookup_bound(key,catalog.actors,source.source_precision())
                .map_err(|e|e.to_string())?
                .ok_or_else(||"49AD70 objective missing-key fallback requires actual TextManager".into()))
        })
    }
    fn load_with_lookup(&mut self,mut lookup:impl FnMut(&[u16])->Result<Vec<u16>>)->Result<()> {
        let result=self.load_inner(&mut lookup);
        if result.is_err(){self.phase=Phase::Failed}
        result
    }
    fn load_inner(&mut self,lookup:&mut impl FnMut(&[u16])->Result<Vec<u16>>)->Result<()> {
        if !matches!(self.phase,Phase::Constructed){return Err("OBJECTIVES load repeated or failed".into())}
        self.phase=Phase::Loading;
        let input=self.input.take().ok_or("actual OBJECTIVES resource input absent")??;
        if let Some(chunk)=input {
            chunk.verify().map_err(|e|e.to_string())?;
            if chunk.directory.name!="OBJECTIVES" {return Err("OBJECTIVES resource identity differs".into())}
            let mut reader=Reader {data:&chunk.bytes,at:0};
            let count=reader.word()? as i32;
            for _ in 0..count.max(0) {
                self.inflight=Some(Objective::constructor());
                //502FC0 accepts version0; later versions need their actual
                //8589B0 diagnostic behavior instead of silently guessing.
                if reader.word()?!=0 {return Err("MapString objective version requires native diagnostic".into())}
                let current=self.inflight.as_mut().unwrap();
                current.key=reader.string()?;
                current.display_key=reader.string()?;
                current.display_text=if current.display_key.is_empty(){Vec::new()}else {
                    lookup(&current.display_key)?
                };
                current.flag=reader.bytes(1)?[0];
                //4FCBB2 appends only after the entire502FC0 returns.
                self.records.push(self.inflight.take().unwrap());
            }
            self.word_25c=reader.word()?;
        }
        self.phase=Phase::Loaded;Ok(())
    }
    pub fn require_loaded(&self)->Result<()> {
        if matches!(self.phase,Phase::Loaded){Ok(())}else {Err("actual OBJECTIVES load has not completed".into())}
    }
    pub fn count(&self)->Result<usize>{self.require_loaded()?;Ok(self.records.len())}
    ///4F1AD0 uses case-sensitive wcscmp, first actual allocation in array order.
    pub fn find(&self,key:&[u16])->Result<Option<Reference<'_>>> {
        self.require_loaded()?;
        Ok(self.records.iter().find(|r|prefix(&r.key)==prefix(key)).map(|row|Reference {row}))
    }
    pub fn find_handle(&self,key:&[u16])->Result<Option<Handle>> {
        self.require_loaded()?;
        Ok(self.records.iter().position(|r|prefix(&r.key)==prefix(key))
            .map(|index|Handle {identity:Arc::downgrade(&self.identity),index}))
    }
    pub fn get(&self,handle:&Handle)->Result<Reference<'_>> {
        self.require_loaded()?;
        if !handle.identity.upgrade().is_some_and(|id|Arc::ptr_eq(&id,&self.identity)) {
            return Err("OBJECTIVES reference belongs to another or destroyed World owner".into());
        }
        Ok(Reference {row:self.records.get(handle.index).ok_or("OBJECTIVES reference allocation absent")?})
    }
    pub fn word_25c(&self)->Result<u32>{self.require_loaded()?;Ok(self.word_25c)}
}

