//! Portable identity of actual Updatable instances. These tokens replace native
//! object addresses; they are unrelated to saved IDs, GTIDs or component slots.
//! Only Queues owns scheduling links. Removing/replacing an instance cancels its
//! links before its token is retired, and a replacement receives a fresh token.
use crate::actor_world::ObjectKey;
use aa_world::queues::{Owner, Queues};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// The retained GWorld+4C instance.
    Sun,
    /// Same actual instance while its saved allocation is held by Load. At
    /// staging only this address label changes; its Owner and events survive.
    Restoring(aa_mapworld::restore_components::UpdatableTarget),
    Actor(ObjectKey),
    Component {
        object: ObjectKey,
        slot: u8,
    },
    DynamicDoodad {
        object: ObjectKey,
        slot: u8,
    },
    Kingdom(ObjectKey),
    Area(ObjectKey),
    /// Identity from the command arena, not a command type or saved ID.
    Command(u64),
    /// Identity from the controller arena, not a class number.
    Controller(u64),
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Updatable instance is already registered")]
    Duplicate,
    #[error("Updatable instance is not registered")]
    Missing,
    #[error("Updatable identity space exhausted")]
    Exhausted,
    #[error(transparent)]
    Queue(#[from] aa_world::queues::Error),
    #[error("Updatable target binding: {0}")]
    Binding(String),
    #[error(transparent)]
    Saved(#[from] crate::updatables::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
pub struct Owners {
    by_target: BTreeMap<Target, Owner>,
    by_owner: BTreeMap<Owner, Target>,
    next: Owner,
}
impl Default for Owners {
    fn default() -> Self {
        Self::new()
    }
}
impl Owners {
    pub fn new() -> Self {
        Self {
            by_target: BTreeMap::new(),
            by_owner: BTreeMap::new(),
            next: 1,
        }
    }
    /// Called once by a real Updatable constructor, including saved-load ctors.
    /// Looking up the same live instance uses `get`, never another registration.
    pub fn register(&mut self, target: Target) -> Result<Owner> {
        if self.by_target.contains_key(&target) {
            return Err(Error::Duplicate);
        }
        let next = self.next.checked_add(1).ok_or(Error::Exhausted)?;
        let owner = self.next;
        self.by_target.insert(target, owner);
        self.by_owner.insert(owner, target);
        self.next = next;
        Ok(owner)
    }
    pub fn get(&self, target: Target) -> Result<Owner> {
        self.by_target.get(&target).copied().ok_or(Error::Missing)
    }
    /// Init may schedule an instance whose saved ID is-1. Rebind every
    /// existing load label, including those absent from the saved-ID table.
    pub fn rebind_restoring(&mut self,actors:&crate::actor_world::ActorStore)->Result<()> {
        let labels:Vec<_>=self.by_target.keys().filter_map(|target| match target {
            Target::Restoring(saved)=>Some(*saved),_=>None,
        }).collect();
        for saved in labels {
            let target=staged_target(&saved,actors)?;
            self.rebind(Target::Restoring(saved),target)?;
        }
        Ok(())
    }
    pub fn restoring(&mut self,target:aa_mapworld::restore_components::UpdatableTarget)->Result<Owner> {
        let target=if matches!(target,aa_mapworld::restore_components::UpdatableTarget::Sun) {
            Target::Sun
        } else {Target::Restoring(target)};
        match self.get(target) {
            Ok(owner)=>Ok(owner),
            Err(Error::Missing)=>self.register(target),
            Err(error)=>Err(error),
        }
    }
    /// Substitute only the portable address of the very same allocation.
    /// No cancellation, schedule insertion, RNG draw or new token occurs.
    pub fn rebind(&mut self,before:Target,after:Target)->Result<Owner> {
        let owner=self.get(before)?;
        if before==after {return Ok(owner);}
        if self.by_target.contains_key(&after) {return Err(Error::Duplicate);}
        if self.by_owner.get(&owner)!=Some(&before) {return Err(Error::Binding("Updatable inverse identity differs".into()));}
        self.by_target.remove(&before);
        self.by_target.insert(after,owner);
        self.by_owner.insert(owner,after);
        Ok(owner)
    }
    pub fn target(&self, owner: Owner) -> Result<Target> {
        self.by_owner.get(&owner).copied().ok_or(Error::Missing)
    }
    /// Original destructor semantics cancel scheduled callbacks while the
    /// instance is still alive. On cancellation error its identity remains.
    pub fn retire(&mut self, target: Target, queues: &mut Queues) -> Result<Owner> {
        let owner = self.get(target)?;
        queues.cancel_owner(owner)?;
        self.by_target.remove(&target);
        self.by_owner.remove(&owner);
        Ok(owner)
    }
    /// Bind saved references to the exact instances moved into ActorStore. This
    /// is pointer-identity substitution only, with no scheduling or publication.
    /// Multiple saved references to one actual instance retain one queue owner.
    pub fn bind_saved_table(
        &mut self,
        table: &crate::updatables::Table<aa_mapworld::restore_components::UpdatableTarget>,
        actors: &crate::actor_world::ActorStore,
    ) -> Result<crate::updatables::Table<Owner>> {
        self.bind_saved_table_with_sun(table,actors,None)
    }
    /// A Sun entry is legal only alongside the retained singleton allocation.
    /// It never resolves through the actor GTID table or a guessed dummy actor.
    pub fn bind_saved_table_with_sun(
        &mut self,
        table:&crate::updatables::Table<aa_mapworld::restore_components::UpdatableTarget>,
        actors:&crate::actor_world::ActorStore,
        sun:Option<&crate::sun_state::State>,
    )->Result<crate::updatables::Table<Owner>> {
        use aa_mapworld::restore_components::UpdatableTarget;
        self.rebind_restoring(actors)?;
        let mut out = crate::updatables::Table::new(
            i32::try_from(table.len()).map_err(|_| Error::Exhausted)?,
        )?;
        for (index, entry) in table.slots().iter().enumerate() {
            let Some(entry) = entry else { continue };
            if matches!(entry, UpdatableTarget::Sun) {
                sun.ok_or_else(||Error::Binding("Sun saved-ID binding requires its actual retained world owner".into()))?;
                let owner=match self.get(Target::Sun) {
                    Ok(owner)=>owner,
                    Err(Error::Missing)=>self.register(Target::Sun)?,
                    Err(error)=>return Err(error),
                };
                out.register(index as i32,owner)?;
                continue;
            }
            let target=staged_target(entry,actors)?;
            let owner = match self.get(Target::Restoring(*entry)) {
                Ok(_)=>self.rebind(Target::Restoring(*entry),target)?,
                Err(Error::Missing)=>match self.get(target) {
                Ok(owner) => owner,
                Err(Error::Missing) => self.register(target)?,
                Err(error) => return Err(error),
                },
                Err(error)=>return Err(error),
            };
            out.register(index as i32, owner)?;
        }
        Ok(out)
    }
}

// Resolve the actual moved allocation, independently of saved-ID presence.
fn staged_target(
    entry:&aa_mapworld::restore_components::UpdatableTarget,
    actors:&crate::actor_world::ActorStore,
)->Result<Target> {
    use crate::actor_world::{Source, SpecialSource};
    use aa_mapworld::{map_objects::SpecialObject, restore_components::UpdatableTarget};
    let raw = match entry {
        UpdatableTarget::Component { owner, .. }
        | UpdatableTarget::Kingdom { owner }
        | UpdatableTarget::DynamicDoodad { owner, .. }
        | UpdatableTarget::Area { owner } => *owner,
        UpdatableTarget::Sun => unreachable!("Sun requires explicit retained-owner binding"),
    };
    let id = aa_actors::ActorId::from_raw(raw);
    let key = actors
        .key(id)
        .ok_or_else(|| Error::Binding(format!("missing live GTID {raw:#x}")))?;
    let entity = actors
        .entity(key)
        .map_err(|e| Error::Binding(e.to_string()))?;
    let target = match entry {
        UpdatableTarget::Sun => unreachable!("Sun requires explicit retained-owner binding"),
        UpdatableTarget::Component { slot, .. } => {
            let slot_id = aa_actors::components::ComponentSlot::from_index(*slot as usize)
                .ok_or_else(|| Error::Binding("invalid Actor component slot".into()))?;
            if actors
                .component(id, slot_id)
                .map_err(|e| Error::Binding(e.to_string()))?
                .is_none()
            {
                return Err(Error::Binding(
                    "saved Updatable references absent Actor component".into(),
                ));
            }
            Target::Component {
                object: key,
                slot: *slot,
            }
        }
        UpdatableTarget::Kingdom { .. } => {
            if !matches!(&entity.source,Source::Special(s) if matches!(s.source,SpecialSource::Kingdom(_)) && s.derived.is_some())
            {
                return Err(Error::Binding(
                    "saved Kingdom Updatable has no constructed Kingdom".into(),
                ));
            }
            Target::Kingdom(key)
        }
        UpdatableTarget::DynamicDoodad { slot, .. } => {
            let present = matches!(&entity.source,Source::Loaded(s) if s.doodad.as_ref()
                .and_then(|d|d.components.get(*slot as usize)).is_some_and(Option::is_some));
            if !present {
                return Err(Error::Binding(
                    "saved Doodad Updatable has no constructed component".into(),
                ));
            }
            Target::DynamicDoodad {
                object: key,
                slot: *slot,
            }
        }
        UpdatableTarget::Area { .. } => {
            if !matches!(&entity.source,Source::Loaded(s) if matches!(s.special_object,Some(SpecialObject::Area(_))))
            {
                return Err(Error::Binding(
                    "saved Area Updatable has no constructed Area".into(),
                ));
            }
            Target::Area(key)
        }
    };
    Ok(target)
}
