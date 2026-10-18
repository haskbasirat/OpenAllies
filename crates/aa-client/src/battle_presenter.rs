//!Rendering-only loans of the current live World. No activation, selection,
//!simulation step, aspect override or visibility default is manufactured here.
use super::{ActorVisual,LiveScene,LiveUpdateReport,battle::{BattleScene,Presentation},spawn::{self,SpawnActor,SpawnReport}};
use crate::{gpu::{InstanceUpdate,SceneRenderer},inspection::{ObjectTransformUpdate,SceneObjects},loading::LoadSettings};
use aa_actors::{ActorId,loading::LoadKind};
use aa_assets::AssetStore;
use aa_runtime::{actor_world::ActorStore,catalog::Catalog,definitions::Bank,live_world_frame::World,
    restoration::Staged,spatial_world::SpatialWorld};
use anyhow::{Context,Result,ensure,bail};

pub struct Presenter {bindings:Option<BattleScene>}
struct RenderOwners<'a> {actors:&'a ActorStore,spatial:&'a SpatialWorld,definitions:&'a Bank}
impl<'a> RenderOwners<'a> {
    fn current(world:&'a World<'_>)->Result<Self> {
        let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
        Ok(Self {actors:p.actors,spatial:p.spatial,definitions:p.definitions})
    }
    fn staged(staged:&'a Staged)->Result<Self> {
        let world=staged.load_world.as_ref().context("staged rendering requires the actual managed World")?;
        Ok(Self {actors:&staged.actors,spatial:world.live_spatial().map_err(anyhow::Error::msg)?,definitions:&staged.definitions})
    }
}
///One completed rendering stage, not VWorldAD or an activation capability.
///Borrowing the exact renderer prevents another renderer with an equal numeric
///epoch from satisfying this stage. Native view initialization must additionally
///verify its required resource/actor set and complete its other ordered work.
pub(crate) struct BoundGpuScene<'a> {
    renderer:&'a SceneRenderer,
    live:&'a LiveScene,
    map_source:&'a aa_mapworld::SourceIdentity,
}
impl BoundGpuScene<'_> {
    pub(crate) fn map_source(&self)->&aa_mapworld::SourceIdentity {self.map_source}
    pub(crate) fn bound_actor_ids(&self)->impl Iterator<Item=u32>+'_ {self.live.actor_ids()}
    pub(crate) fn scene_epoch(&self)->u64 {self.renderer.scene_epoch()}
}
impl Presenter {
    pub fn new()->Self {Self {bindings:None}}
    ///Prepare the current canonical model before retiring any old rendered
    ///binding. A replacement may retain the numerical GTID while changing its
    ///source definition; original placement keys never choose the new model.
    pub fn bind_current_actor(&mut self,world:&World<'_>,catalog:&Catalog<'_>,store:&AssetStore,
        settings:LoadSettings,presentation:&impl Presentation,id:ActorId,
        live:&mut LiveScene,renderer:&mut SceneRenderer,objects:&mut SceneObjects)->Result<SpawnReport> {
        self.bind_actor(RenderOwners::current(world)?,catalog,store,settings,presentation,id,live,renderer,objects)
    }
    ///The pre-unfreeze view stage borrows the same started graph; it does not
    ///construct a frame World or grant native view/application completion.
    pub fn bind_staged_actor(&mut self,staged:&Staged,catalog:&Catalog<'_>,store:&AssetStore,
        settings:LoadSettings,presentation:&impl Presentation,id:ActorId,
        live:&mut LiveScene,renderer:&mut SceneRenderer,objects:&mut SceneObjects)->Result<SpawnReport> {
        self.bind_actor(RenderOwners::staged(staged)?,catalog,store,settings,presentation,id,live,renderer,objects)
    }
    fn bind_actor(&mut self,p:RenderOwners<'_>,catalog:&Catalog<'_>,store:&AssetStore,
        settings:LoadSettings,presentation:&impl Presentation,id:ActorId,
        live:&mut LiveScene,renderer:&mut SceneRenderer,objects:&mut SceneObjects)->Result<SpawnReport> {
        let key=p.actors.key(id).context("current render GTID is absent or stale")?;
        let object=p.actors.object(key)?;
        ensure!(matches!(object.definition.native_class,2|3|4),"current model requires its actual derived view/aspect service");
        ensure!(object.kind==if object.definition.native_class==2 {LoadKind::Object}else{LoadKind::Actor},"current model constructor layout differs");
        let source=aa_runtime::live_object_init::Source::bind_in_bank(catalog,p.definitions,&object.definition)?;
        if let Some(actor)=&object.actor {
            ensure!(!actor.components.state.construction(),"current construction aspect4EAAA0 is required");
            ensure!(actor.words.get(&0x15c)==Some(&u32::MAX),"current stance aspect4D0700 is required");
        }
        let fields=object.object.as_ref().context("current render Object allocation absent")?;
        let creation=catalog.actors.native_creation_definition(&object.definition.proof.id,source.source_precision())?;
        let index=if creation.variations_enabled && fields.variant!=-1 {
            u32::try_from(fields.variant).context("negative native variation index")?
        } else {aa_runtime::object_aspect_source::DefaultIndex::bind(catalog,p.definitions,&object.definition)?.index()};
        let placement=p.actors.placement(key,p.spatial)?;
        let appearance=presentation.appearance(p.actors,p.spatial,id,object,placement)?;
        let visual=ActorVisual {actor_id:id.raw(),native_position:placement.position_bits.map(f32::from_bits),
            native_facing:placement.facing_bits.map(f32::from_bits),scale:f32::from_bits(fields.scale_bits),
            visible:appearance.visible,team_color:appearance.team_color};
        let prepared=spawn::prepare(store,catalog.actors,settings,SpawnActor {
            thing_id:object.definition.proof.id.clone(),object_variant:fields.variant,resolved_aspect:Some(index),visual,
        },source.source_precision())?;
        self.bindings=None;
        live.retire_render_binding(renderer,objects,id.raw())?;
        live.append(renderer,objects,prepared)
    }
    pub fn seal_current(&mut self,world:&World<'_>,live:&LiveScene,objects:&SceneObjects)->Result<()> {
        self.bindings=Some(BattleScene::from_live_world(world,live,objects)?);Ok(())
    }
    pub fn seal_staged(&mut self,staged:&Staged,live:&LiveScene,objects:&SceneObjects)->Result<()> {
        self.bindings=Some(BattleScene::from_staged(staged,live,objects)?);Ok(())
    }
    pub fn present_current(&self,world:&World<'_>,presentation:&impl Presentation,live:&mut LiveScene,
        renderer:&mut SceneRenderer,objects:&mut SceneObjects,sequence:u64)->Result<LiveUpdateReport> {
        self.bindings.as_ref().context("current actor/model bindings must be sealed after rebinding")?
            .apply_live_world(world,presentation,live,renderer,objects,sequence)
    }
    pub fn present_staged(&self,staged:&Staged,presentation:&impl Presentation,live:&mut LiveScene,
        renderer:&mut SceneRenderer,objects:&mut SceneObjects,sequence:u64)->Result<LiveUpdateReport> {
        self.bindings.as_ref().context("staged actor/model bindings must be sealed after rebinding")?
            .apply(staged,presentation,live,renderer,objects,sequence)
    }
    ///Mint only after actual source binding, accepted GPU/picking publication
    ///and a current owner recheck. It cannot call74FF50 or unfreeze the world.
    pub(crate) fn rendering_stage<'a>(&self,world:&'a World<'_>,presentation:&impl Presentation,
        live:&'a LiveScene,renderer:&'a SceneRenderer,objects:&SceneObjects)->Result<BoundGpuScene<'a>> {
        let bindings=self.checked_renderer(live,renderer,objects)?;
        Self::require_complete(RenderOwners::current(world)?,live)?;
        bindings.collect_live_world(world,presentation)?;
        Ok(BoundGpuScene {renderer,live,map_source:&world.evidence().inventory.map_source})
    }
    pub(crate) fn rendering_stage_staged<'a>(&self,staged:&'a Staged,presentation:&impl Presentation,
        live:&'a LiveScene,renderer:&'a SceneRenderer,objects:&SceneObjects)->Result<BoundGpuScene<'a>> {
        let bindings=self.checked_renderer(live,renderer,objects)?;
        Self::require_complete(RenderOwners::staged(staged)?,live)?;
        bindings.collect(staged,presentation)?;
        Ok(BoundGpuScene {renderer,live,map_source:&staged.inventory.map_source})
    }
    fn require_complete(p:RenderOwners<'_>,live:&LiveScene)->Result<()> {
        ensure!(!live.bindings.is_empty(),"GPU stage has no bound current native renderables");
        //Until the concrete native per-class view dispatch supplies a proved
        //nonrendering branch, an unbound current source is an error. In
        //particular an empty scene cannot certify all actors as invisible.
        for entry in p.actors.identities().iter() {
            let entity=p.actors.entity(entry.value)?;
            let object=entity.runtime.as_ref().with_context(||format!("GPU stage GTID{} requires its actual derived native view dispatch",entry.id.raw()))?;
            ensure!(live.bindings.contains_key(&entry.id.raw()),
                "GPU stage missing current GTID{} source{} class{} model/view dispatch",
                entry.id.raw(),object.definition.proof.id,object.definition.native_class);
        }
        Ok(())
    }
    fn checked_renderer(&self,live:&LiveScene,renderer:&SceneRenderer,objects:&SceneObjects)->Result<&BattleScene> {
        let bindings=self.bindings.as_ref().context("GPU stage has no sealed actor bindings")?;
        let epoch=live.epoch.context("GPU stage is not attached to a scene")?;
        ensure!(epoch!=0 && epoch==renderer.scene_epoch(),"GPU stage scene epoch differs");
        ensure!(live.last_sequence.is_some(),"GPU stage has no accepted current actor snapshot");
        //Collect does not mutate simulation, GPU or picking. It rejects changed
        //definitions/variants/stances and foreign actor/spatial owner witnesses.
        bindings.validate_live_bindings(live,objects)?;
        Ok(bindings)
    }
    ///Explicitly hide removed native generations. No unchanged visual receives
    ///a fabricated transform, and no simulation identity is modified.
    pub fn retire_absent(&mut self,world:&World<'_>,live:&mut LiveScene,renderer:&mut SceneRenderer,objects:&mut SceneObjects)->Result<usize> {
        let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
        let ids=live.actor_ids().filter(|id|p.actors.key(ActorId::from_raw(*id)).is_none()).collect::<Vec<_>>();
        for id in &ids {self.bindings=None;live.retire_render_binding(renderer,objects,*id)?;}
        Ok(ids.len())
    }
}
impl LiveScene {
    ///GPU and picking must both accept hiding before a source binding retires.
    ///Old GPU slots remain hidden allocations until the next scene upload.
    fn retire_render_binding(&mut self,renderer:&mut SceneRenderer,objects:&mut SceneObjects,id:u32)->Result<bool> {
        let Some(binding)=self.bindings.get(&id) else {return Ok(false)};
        let epoch=self.epoch.context("current map scene is not attached")?;
        ensure!(epoch==renderer.scene_epoch(),"replacement render binding has stale scene epoch");
        let visual=binding.original;
        let value=visual.instance()?;
        let picking=objects.prepare_transforms(&[ObjectTransformUpdate {object:binding.object,instance:value,
            native_position:visual.native_position,visible:false}])?;
        let instances=binding.slots.iter().map(|&slot|InstanceUpdate {slot,value,visible:false}).collect::<Vec<_>>();
        renderer.update_instances(epoch,&instances)?;
        objects.apply_transforms(picking);
        if self.bindings.remove(&id).is_none() {bail!("render binding disappeared during callback-free retirement")}
        Ok(true)
    }
}
