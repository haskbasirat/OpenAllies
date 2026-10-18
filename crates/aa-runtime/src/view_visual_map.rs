//!Logical process-global visual TerrainMap B36518, distinct from GWorld+30.
//!Native43F0A0 publishes721330 late, after431BC0;43E400 clears it. The modern
//!backend publishes here after its real GPU terrain stage, before camera tail.
//!This deliberately does not claim legacy721330/723E00 GPU or ABI parity.
use crate::{live_world_frame::World,view_heightfield,view_init_world,world_revision::{ConstructedWorldRevision,Witness}};
type Result<T>=std::result::Result<T,String>;
struct Current{world:Witness,height:view_heightfield::SampleWitness,dimensions:[i32;2]}
///Retain once per application. Initial B36518 is an actual raw PE DWORD0 in
///both targets (not an inference from the presence of a simulation World).
pub struct Global{current:Option<Current>}
impl Global{
 pub fn from_fresh_process_image()->Self{Self{current:None}}
 ///700E30 sees NULL until actual visual-map publication. A caller must lend
 ///the SAME VWorld height allocation for a positive current-map lookup.
 pub fn dimensions_for_world(&self,world:&ConstructedWorldRevision,height:Option<&view_heightfield::State>)->Result<Option<[i32;2]>>{
  let Some(current)=&self.current else{return Ok(None)};
  if !current.world.is_for(world){return Err("current visual TerrainMap belongs to a foreign or retired World; actual teardown required".into())}
  let height=height.ok_or("current visual TerrainMap height allocation is absent")?;
  if !current.height.is_for(height,world){return Err("current visual TerrainMap names another heightfield allocation".into())}
  Ok(Some(current.dimensions))
 }
 ///Explicit modern terrain mapping: only an actually installed/validated
 ///terrain-stage receipt may publish dimensions. This is NOT viewAD/readiness.
 ///A failed old positive map requires real teardown; it is never overwritten.
 pub fn publish_installed_terrain<V,A>(&mut self,world:&World<'_>,view:&view_init_world::State<V,A>)->Result<()>{
  if self.current.is_some(){return Err("visual TerrainMap already published; actual43E400/backend teardown required".into())}
  let height=view.terrain_for_visual_map(world)?;
  let parts=world.presentation_parts()?;
  let extents=parts.creator_extents.ok_or("visual TerrainMap requires actual World3C/40 extent fields")?;
  let dimensions=dimensions_7241ce(extents.bits())?;
  let current=Current{world:parts.revision.witness(),height:height.witness_for_world(parts.revision)?,dimensions};
  self.current=Some(current);Ok(())
 }
}
///7241CE..724205: each stored binary32 World extent is exactly promoted to
///binary64, CRT ceil96D4C0 then _ftol96CC4C, before writing74 then78.
///The accepted finite signed32 domain has no PC-dependent rounding. Native
///NaN/infinite/indefinite conversion remains explicitly outside this adapter.
fn dimensions_7241ce(extents:[u32;2])->Result<[i32;2]>{
 let mut out=[0;2];
 for(index,bits)in extents.into_iter().enumerate(){
  let value=f64::from(f32::from_bits(bits));
  if !value.is_finite(){return Err("visual-map extent requires native nonfinite ceil/integer handling".into())}
  let ceiling=value.ceil();
  if ceiling<f64::from(i32::MIN)||ceiling>f64::from(i32::MAX){return Err("visual-map extent is outside proved native signed32 conversion domain".into())}
  out[index]=ceiling as i32;
 }
 Ok(out)
}
#[cfg(test)]#[path="view_visual_map_tests.rs"]mod tests;
