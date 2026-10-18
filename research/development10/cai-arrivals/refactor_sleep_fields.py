from pathlib import Path
p=Path('crates/aa-runtime/src/stored_regiment_sleep.rs');s=p.read_text();a=s.index('    fn sleep_522f60(');b=s.index('\n}\npub(super) fn think(',a)
s=s[:a]+'''    fn sleep_522f60(&mut self,_:StateId,a:&mut Arena)->aa_mind::Result<()> {
        aa_mind::cai_sleep::fields::refresh_members(&mut Refresh{sleep:self,arena:a})
    }
    fn sleep_523000(&mut self,_:StateId,a:&mut Arena)->aa_mind::Result<()> {
        aa_mind::cai_sleep::fields::retry_refresh(&mut Refresh{sleep:self,arena:a})
    }
''' +s[b:]
s+='''
struct Refresh<'call,'a,'b,'store,'source>{sleep:&'call mut Sleep<'a,'b,'store,'source>,arena:&'call mut Arena}
impl aa_mind::cai_sleep::fields::RefreshFields for Refresh<'_,'_,'_,'_,'_>{
 type Member=ActorId;
 fn retry(&mut self)->aa_mind::Result<u32>{self.sleep.read(0x48)}
 fn store_retry(&mut self,v:u32)->aa_mind::Result<()>{self.sleep.write(0x48,v)}
 fn next_attached(&mut self,index:&mut usize)->aa_mind::Result<Option<ActorId>>{self.sleep.next_member(index,true).map(|v|v.map(|(_,m)|m))}
 fn mode(&mut self)->aa_mind::Result<u32>{self.sleep.read(0x28c)}
 fn primary(&mut self,member:ActorId)->aa_mind::Result<bool>{
  let Some(RuntimeComponent::Element(e))=self.sleep.actors.component(member,Slot::Element).map_err(error)?else{return required("570650 actual current member Element absent")};
  if let Some(owner)=e.organization {match self.sleep.actors.component(owner,Slot::Organization).map_err(error)?{Some(RuntimeComponent::Organization(org))if org.header.owner==owner=>Ok(org.words.get(&0x44).copied().ok_or_else(||error("570650 actual Organization primary+44 unwritten"))?==member.raw()),_=>required("570650 current Element Organization absent")}}else{Ok(false)}
 }
 fn flag(&mut self)->aa_mind::Result<u8>{self.sleep.byte(0x4c)}
 fn dispatch(&mut self,m:ActorId,f:u8)->aa_mind::Result<()>{self.sleep.arrived(self.sleep.binding.mind,m.raw(),f,self.arena)}
 fn store_pending(&mut self,v:u8)->aa_mind::Result<()>{self.sleep.write_byte(0x44,v)}
}
impl aa_mind::cai_sleep::fields::RetryFields for Refresh<'_,'_,'_,'_,'_>{
 fn any_active(&mut self)->aa_mind::Result<bool>{let mut i=0;while let Some((_,m))=self.sleep.next_member(&mut i,true)?{if crate::stored_member_command::member_active_on_owners(self.sleep.actors,m)?{return Ok(true)}}Ok(false)}
 fn all_at_slots(&mut self)->aa_mind::Result<bool>{self.sleep.all_at_slots()}
 fn retry(&mut self)->aa_mind::Result<u32>{self.sleep.read(0x48)}
 fn store_pending(&mut self,v:u8)->aa_mind::Result<()>{self.sleep.write_byte(0x44,v)}
 fn refresh(&mut self)->aa_mind::Result<()>{aa_mind::cai_sleep::fields::refresh_members(self)}
}
''';p.write_text(s)
