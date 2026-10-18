//! Actual staged Updatable head/periodic insertion over the moved owner map.
//! Mover/Element/Economy embedded+8 is the native6A71D0 constant-true leaf. No owner is
//! registered here; only the real constructor's existing Target is accepted.
use crate::{actor_world::ObjectKey,restoration::Staged,scheduler_owners::Target};
use aa_actors::{actor_creation::{Error,Result},components::ComponentSlot as Slot};
use aa_numeric::Precision;
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
pub(crate) struct World<'a>{pub staged:&'a mut Staged,pub key:ObjectKey,pub slot:Slot,pub precision:Precision,owner:Option<aa_world::queues::Owner>}
impl<'a> World<'a>{
 pub fn new(staged:&'a mut Staged,key:ObjectKey,slot:Slot,precision:Precision)->Result<Self>{
  if !matches!(slot,Slot::Mover|Slot::Element|Slot::Economy|Slot::Denizen|Slot::Supply|Slot::Construction|Slot::Detection|Slot::Lair|Slot::Organization){return Err(error("stored scheduling unsupported actual embedded Updatable class"))}
  Ok(Self{staged,key,slot,precision,owner:None})
 }
 fn owner(&mut self)->Result<aa_world::queues::Owner>{
  if let Some(owner)=self.owner{return Ok(owner)}
  let owner=self.staged.owners.get(Target::Component{object:self.key,slot:self.slot.index()as u8}).map_err(error)?;
  self.owner=Some(owner);Ok(owner)
 }
 pub fn pending(&mut self)->Result<bool>{
  let owner=self.owner()?;
  if self.staged.queues.precision!=self.precision{return Err(error("stored component head queue/runtime precision differs"))}
  Ok(self.staged.queues.owner_head(owner).is_some())
 }
 pub fn add_periodic(&mut self,queue:u8)->Result<()>{
  let expected=if self.slot==Slot::Mover{0}else{1};
  if queue!=expected{return Err(error("stored component native periodic queue differs"))}
  let owner=self.owner()?;
  if self.staged.queues.precision!=self.precision{return Err(error("stored periodic queue/runtime precision differs"))}
  self.staged.queues.add_periodic(queue,owner,0,&mut self.staged.rng).map_err(error)?;Ok(())
 }
}
impl crate::load_detection_init_world::World for World<'_>{
 fn version(&mut self)->Result<u32>{Ok(self.staged.report.world_version)}
 fn pending(&mut self)->Result<bool>{self.pending()}
 fn add_periodic(&mut self)->Result<()>{self.add_periodic(1)}
}
