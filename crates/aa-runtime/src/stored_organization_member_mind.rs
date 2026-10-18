//!524B80 over the same Arena and actual member Mind fields.
//!Current base24/28 writes precede the required522890 command child.
use crate::{
    actor_world::{ActorAccess, RuntimeComponent},
    catalog::Catalog,
    definitions::Bank,
    restoration::Staged,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_mind::{Arena, MindId, cai_member_added::MemberAddedWorld, construct::Kind};
type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(e.to_string())
}
struct SpatialLoan<'a>(Option<&'a crate::spatial_world::SpatialWorld>);
impl crate::actor_world::Spatial for SpatialLoan<'_> {
    fn placement(
        &self,
        id: ActorId,
    ) -> crate::actor_world::Result<crate::actor_world::PlacementState> {
        crate::actor_world::Spatial::placement(
            self.0.ok_or(crate::actor_world::Error::Missing(
                "52C280 actual published home placement owner absent",
            ))?,
            id,
        )
    }
    fn publish(
        &mut self,
        _: ActorId,
        _: crate::actor_world::PlacementState,
    ) -> crate::actor_world::Result<()> {
        Err(crate::actor_world::Error::Missing(
            "52C280 readonly placement loan cannot publish",
        ))
    }
}
struct Bound<'a, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    bank: &'a mut Bank,
    actors: &'a mut ActorAccess<'store>,
    owner: ActorId,
    world: Option<&'a mut crate::load_world::World>,
    queues: &'a mut aa_world::queues::Queues,
    owners: &'a mut crate::scheduler_owners::Owners,
    random: &'a mut aa_simulation::NativeRng,
    clock: &'a aa_simulation::NativeClock,
    inventory: &'a aa_mapworld::WorldInventory,
    pc: aa_numeric::Precision,
    phase: Option<&'a dyn crate::load_world_globals::SessionPhase>,
}
impl Bound<'_, '_, '_> {
    fn spatial(&self)->Option<&crate::spatial_world::SpatialWorld>{
        self.world.as_ref().and_then(|world|match &world.spatial {
            crate::load_world::SpatialOwner::Adopted(a)
                if a.error.is_none()&&a.pending.is_empty()=>Some(&a.world),
            _=>None,
        })
    }
    fn require_no_nested_members(&self, actor: ActorId, child: &str) -> aa_mind::Result<()> {
        let Some(RuntimeComponent::Organization(org)) = self
            .actors
            .component(actor, Slot::Organization)
            .map_err(error)?
        else {
            return Err(error("522620/522690 actual nested Organization absent"));
        };
        if org.header.owner != actor || org.members.len() > i32::MAX as usize {
            return Err(error(
                "522620/522690 actual nested Organization owner/count differs",
            ));
        }
        //4364C0 with attached-only=false skips actual NULL members. No outward
        //callback can change this borrowed array during this empty-path read.
        if org.members.iter().any(|id| !id.is_null()) {
            return Err(error(child));
        }
        Ok(())
    }
    fn mind(&self, actor: ActorId, arena: &Arena) -> aa_mind::Result<(MindId, Kind)> {
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| error("524B80 actual member generation absent"))?;
        let object = self.actors.object(key).map_err(error)?;
        if object.kind != aa_actors::loading::LoadKind::Actor
            || object.definition.native_class != 3
            || object.object.is_none()
            || object.actor.is_none()
            || !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
        {
            return Err(error(
                "524B80 actual ordinary Actor constructor/proof differs",
            ));
        }
        crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, &object.definition)
            .map_err(error)?;
        let expected = crate::catalog::mind_kind(
            object
                .definition
                .component_mask
                .ok_or_else(|| error("524B80 actual member source mask absent"))?,
        )
        .ok_or_else(|| error("524B80 member source has no Mind"))?;
        let Some(RuntimeComponent::Mind(m)) =
            self.actors.component(actor, Slot::Mind).map_err(error)?
        else {
            return Err(error("524B80 member actual Mind pointer is null"));
        };
        if expected != m.kind || arena.mind(m.mind_id)?.actor != actor.raw() {
            return Err(error(
                "524B80 actual member Mind/Arena/source identity differs",
            ));
        }
        Ok((m.mind_id, m.kind))
    }
}
impl MemberAddedWorld for Bound<'_, '_, '_> {
    fn resolve(&mut self, gtid: u32) -> aa_mind::Result<Option<u32>> {
        let id = ActorId::from_raw(gtid);
        Ok(self.actors.key(id).map(|_| id.raw()))
    }
    fn mind_word(&mut self, id: MindId, offset: u32, arena: &Arena) -> aa_mind::Result<u32> {
        let mind = arena.mind(id)?;
        if mind.actor != self.owner.raw() {
            return Err(error("524B80 owner Mind allocation differs"));
        }
        match offset {
            0x24 => Ok(mind.words_24_30[0]),
            0x28 => Ok(mind.words_24_30[1]),
            _ => Err(error("524B80 unexpected base word")),
        }
    }
    fn member_mind_3c(
        &mut self,
        member: u32,
        value: u32,
        arena: &mut Arena,
    ) -> aa_mind::Result<()> {
        let (id, kind) = self.mind(ActorId::from_raw(member), arena)?;
        //51C220 is inherited by Base/Element/Aircraft;522620 writes first,
        //then dispatches the current Regiment's member iteration.
        arena.mind_mut(id)?.words_24_30[0] = value;
        if kind == Kind::Regiment {
            self.require_no_nested_members(
                ActorId::from_raw(member),
                "522620 actual nested-Regiment member propagation after24 store",
            )?;
        }
        Ok(())
    }
    fn member_mind_40(
        &mut self,
        member: u32,
        target: Option<u32>,
        arena: &mut Arena,
    ) -> aa_mind::Result<()> {
        let (id, kind) = self.mind(ActorId::from_raw(member), arena)?;
        if let Some(target) = target {
            let target = ActorId::from_raw(target);
            let key = self
                .actors
                .key(target)
                .ok_or_else(|| error("52C280 current home target generation absent"))?;
            let flags = self
                .actors
                .placement(key, &SpatialLoan(self.spatial()))
                .map_err(error)?
                .flags;
            if flags & 1 == 0 {
                return Err(error(
                    "52C29A actual non-destroyed home target virtual174 relation owner",
                ));
            }
        }
        //The real null/destroyed target path clears the current base28 cell.
        arena.mind_mut(id)?.words_24_30[1] = 0;
        if kind == Kind::Regiment {
            self.require_no_nested_members(
                ActorId::from_raw(member),
                "522690 actual nested-Regiment member propagation after28 store",
            )?;
        }
        Ok(())
    }
    fn helper_522890(
        &mut self,
        mind: MindId,
        member: u32,
        flag: u32,
        arena: &mut Arena,
    ) -> aa_mind::Result<()> {
        let world=self.world.as_deref_mut().ok_or_else(||error("522890 actual World owner absent"))?;
        crate::stored_member_command::run(self.catalog,self.actors,self.bank,world,mind,
            ActorId::from_raw(member),flag as u8,arena,self.queues,self.owners,self.random,
            self.clock,self.inventory,self.pc)
    }
    fn game_mode(&mut self) -> aa_mind::Result<u32> {
        self.phase
            .ok_or_else(|| error("524B80 actual Session+E4 owner absent"))?
            .application_state()
            .map_err(error)
    }
    fn actor_gtid(&mut self, actor: u32) -> aa_mind::Result<u32> {
        let id = ActorId::from_raw(actor);
        self.actors
            .key(id)
            .ok_or_else(|| error("524B80 current member GTID absent"))?;
        Ok(actor)
    }
    fn arrival_push_front(&mut self, mind: MindId, gtid: u32, arena: &mut Arena) -> aa_mind::Result<()> {
        crate::stored_cai_arrivals::push_front(self.actors,arena,self.owner,mind,gtid)
    }
}
pub(crate) fn notify(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    owner: ActorId,
    member: ActorId,
    pc: aa_numeric::Precision,
) -> Result<()> {
    let key = staged
        .actors
        .key(owner)
        .ok_or("524B80 actual owner generation absent")?;
    crate::fresh_post_create_world::canonical(staged, catalog, key)?;
    let Some(RuntimeComponent::Mind(m)) = staged
        .actors
        .component(owner, Slot::Mind)
        .map_err(|e| e.to_string())?
    else {
        return Err("524B80 actual owner Mind absent".into());
    };
    if m.kind != Kind::Regiment {
        return Err("524B80 actual owner is not Regiment Mind".into());
    }
    let id = m.mind_id;
    let (arena, mut actors) = staged.actors.split_minds();
    let mut bound = Bound {
        catalog,
        bank: &mut staged.definitions,
        actors: &mut actors,
        owner,
        world: staged.load_world.as_mut(),
        queues: &mut staged.queues,
        owners: &mut staged.owners,
        random: &mut staged.rng,
        clock: &staged.clock,
        inventory: &staged.inventory,
        pc,
        phase: catalog.load_session,
    };
    bound.mind(owner, arena).map_err(|e| e.to_string())?;
    aa_mind::cai_member_added::member_added(&mut bound, id, member.raw(), arena)
        .map_err(|e| e.to_string())
}
