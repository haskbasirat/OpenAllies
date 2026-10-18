//! Render-only static map transport. No ActorStore/World crosses threads and
//! no static upload certifies native VObject Init or74FF50 completion.
use super::{LoadSettings,load_impl};
use crate::gpu::{CpuScene,RendererWitness,SceneRenderer};
use aa_assets::AssetStore;
use aa_mapworld::{SourceIdentity,WorldInventory};
use aa_runtime::{actor_world::{ActorStore,ActorStoreWitness},live_world_frame::World,
    world_revision::{ConstructedWorldRevision,Witness}};
use anyhow::{Context,Result,ensure};
use std::{collections::BTreeSet,sync::Arc};

struct Ticket {
    id:String,source:SourceIdentity,version:u32,original_ids:BTreeSet<u32>,excluded:BTreeSet<u32>,
    world:Witness,actors:ActorStoreWitness,
}
///Retained on the simulation thread while its matching request is serviced.
pub struct Pending(Arc<Ticket>);
///Only immutable source identities and render exclusions enter the UI packet.
pub struct Request(Arc<Ticket>);
pub struct Prepared{ticket:Arc<Ticket>,scene:CpuScene}
pub struct Receipt{ticket:Arc<Ticket>,renderer:RendererWitness}
pub struct Bound{ticket:Arc<Ticket>,renderer:RendererWitness}

impl Pending {
    pub fn request(world:&World<'_>,selected_map_id:&str)->Result<(Self,Request)>{
        let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
        let original_ids=original_ids(p.inventory)?;let mut excluded=original_ids.clone();
        excluded.extend(p.actors.identities().iter().map(|entry|entry.id.raw()));
        let ticket=Arc::new(Ticket{id:selected_map_id.into(),source:p.inventory.map_source.clone(),
            version:p.inventory.world.version,original_ids,excluded,world:p.revision.witness(),actors:p.actors.identity_witness()});
        Ok((Self(Arc::clone(&ticket)),Request(ticket)))
    }
    pub fn accept(self,world:&World<'_>,receipt:Receipt)->Result<Bound>{
        ensure!(Arc::ptr_eq(&self.0,&receipt.ticket),"static map receipt belongs to another request");
        validate(&self.0,world)?;ensure!(receipt.renderer.is_current(),"static map renderer was dropped/reloaded");
        Ok(Bound{ticket:receipt.ticket,renderer:receipt.renderer})
    }
}
fn original_ids(inventory:&WorldInventory)->Result<BTreeSet<u32>>{
    ensure!(inventory.dynamic_things.map_source==inventory.map_source,"battle DynamicThings belongs to another map");
    ensure!(inventory.dynamic_things.unsupported.is_empty()&&inventory.generation_mismatches.is_empty()
        &&inventory.team_generation_mismatches.is_empty()&&inventory.kingdom_generation_mismatches.is_empty(),
        "battle exclusions require complete same-generation native restore records");
    let teams=inventory.teams.as_ref().context("battle Team records are unbound")?;
    let kingdoms=inventory.kingdoms.as_ref().context("battle Kingdom records are unbound")?;
    let mut ids=inventory.dynamic_things.placements.iter().map(|v|v.serialized_id).collect::<BTreeSet<_>>();
    ids.extend(teams.slots.iter().filter_map(|v|v.record.as_ref()).map(|v|v.serialized_id));
    ids.extend(kingdoms.slots.iter().filter_map(|v|v.record.as_ref()).map(|v|v.serialized_id));Ok(ids)
}
fn validate(ticket:&Ticket,world:&World<'_>)->Result<()>{
    let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
    require_owners(ticket,p.actors,p.revision,&p.inventory.map_source)?;
    ensure!(p.inventory.world.version==ticket.version&&original_ids(p.inventory)?==ticket.original_ids,
        "static map immutable restore record identities changed");
    ensure!(p.actors.identities().iter().all(|entry|ticket.excluded.contains(&entry.id.raw())),
        "static map exclusions are stale after native actor creation");Ok(())
}
fn require_owners(ticket:&Ticket,actors:&ActorStore,revision:&ConstructedWorldRevision,source:&SourceIdentity)->Result<()>{
    ensure!(ticket.world.is_for(revision)&&ticket.actors.is_for(actors)&&&ticket.source==source,
        "static map receipt uses another/retired World allocation or source");Ok(())
}
impl Request {
    pub fn load(self,store:&AssetStore,settings:LoadSettings)->Result<Prepared>{
        let info=store.maps().iter().find(|m|m.id==self.0.id).context("battle selected map is absent")?;
        let raw=aa_mapworld::MapArchive::from_bytes(std::fs::read(store.resolve_url(&format!("/assets/original/Data/{}",info.source))?)?)?;
        ensure!(raw.source()==&self.0.source,"static map request names another raw archive");
        let dynamic=aa_mapworld::inventory::Inventory::decode(&raw,u64::MAX)?;
        let teams=aa_mapworld::kingdoms::Teams::decode(&raw.decode_chunk("TEAMS",u64::MAX)?)?;
        let kingdoms=aa_mapworld::kingdoms::Kingdoms::decode(&raw.decode_chunk("KINGDOMS",u64::MAX)?,self.0.version)?;
        let mut ids=dynamic.placements.iter().map(|v|v.serialized_id).collect::<BTreeSet<_>>();
        ids.extend(teams.slots.iter().filter_map(|v|v.record.as_ref()).map(|v|v.serialized_id));
        ids.extend(kingdoms.slots.iter().filter_map(|v|v.record.as_ref()).map(|v|v.serialized_id));
        ensure!(ids==self.0.original_ids,"static map request original IDs differ from the actual archive");
        let scene=load_impl(store,&self.0.id,settings,Some(&self.0.excluded),Some(&self.0.source))?;
        ensure!(scene.live.is_some()&&scene.objects.is_some(),"static map upload lacks retained dynamic/picking geometry");
        Ok(Prepared{ticket:self.0,scene})
    }
}
impl Prepared {
    ///The battle path uses one upload: replace the inspection terrain with
    ///the SAME native visual-heightfield geometry before submitting the full
    ///static scene. Non-native static objects and picking owners stay retained.
    pub fn upload_with_terrain(mut self,terrain:crate::view_terrain_backend::Prepared,renderer:&mut SceneRenderer)
        ->Result<(CpuScene,Receipt,crate::view_terrain_backend::Installed)>{
        self.scene.live.as_ref().context("static map live scene absent")?.require_map_source(&self.ticket.source)?;
        let installed=terrain.install_into_scene(&mut self.scene,&self.ticket.source,renderer)?;
        self.scene.live.as_mut().unwrap().attach(renderer.scene_epoch())?;
        let receipt=Receipt{ticket:self.ticket,renderer:renderer.identity_witness()};Ok((self.scene,receipt,installed))
    }
    pub fn upload(mut self,renderer:&mut SceneRenderer)->Result<(CpuScene,Receipt)>{
        self.scene.live.as_ref().context("static map live scene absent")?.require_map_source(&self.ticket.source)?;
        renderer.upload(&self.scene)?;
        self.scene.live.as_mut().unwrap().attach(renderer.scene_epoch())?;
        let receipt=Receipt{ticket:self.ticket,renderer:renderer.identity_witness()};Ok((self.scene,receipt))
    }
}
impl Bound {
    pub fn validate(&self,world:&World<'_>)->Result<()>{validate(&self.ticket,world)?;
        ensure!(self.renderer.is_current(),"static map renderer was dropped/reloaded");Ok(())}
    pub fn map_source(&self)->&SourceIdentity{&self.ticket.source}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn render_only_map_transport_is_send(){fn send<T:Send>(){}send::<Request>();send::<Prepared>();send::<Receipt>();}
}
