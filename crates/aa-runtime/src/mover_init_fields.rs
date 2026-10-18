//! Native576700 ordering over borrowed current Mover/world/state fields.
//! No owner or field is synthesized; callers supply actual stored services.
use aa_actors::actor_creation::Result;
pub(crate) trait Fields{
 fn falling(&self)->Result<u8>;
 fn version(&mut self)->Result<u32>;
 fn pending(&mut self)->Result<bool>;
 fn add_periodic(&mut self)->Result<()>;
 fn activate_falling(&mut self)->Result<()>;
}
pub(crate) fn initialize(fields:&mut impl Fields)->Result<bool>{
 if fields.falling()?==0{return Ok(true)}
 if fields.version()?<22&&!fields.pending()?{fields.add_periodic()?;}
 fields.activate_falling()?;
 Ok(true)
}
