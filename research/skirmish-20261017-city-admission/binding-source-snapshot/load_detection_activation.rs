//! Detection56EC90 on its same load component and Actor+148 zone owner.
//! Zone storage is borrowed directly through515F40/515970; missing native
//! world gates and outward callbacks are required services, never defaults.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    ActorId,
    components::{ComponentSlot, Components},
    loading::ActorShell,
};
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::Precision;
use aa_zones::{Error, Result, component::ActorZones, fpu, tile::ZoneMap, zone::Zone};
use std::collections::BTreeMap;
#[path="zone_actor_reads.rs"]
mod actor_reads;
#[path="zone_primary_reference.rs"]
mod primary_reference;

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
fn required<T>(name: &'static str) -> Result<T> {
    Err(Error::World(format!("required native service: {name}")))
}

//5595D0 consumes a WORD at Kingdom+22C. The actual saved constructor owns
//these two bytes separately; older diagnostic shells may own a packed word.
//Never extend this read into the adjacent native+22E/+22F field.
fn kingdom_supply_mask(shell: &aa_mapworld::special_state::SpecialShell) -> Result<u16> {
    if let Some(word) = shell.words.get(&0x22c) {
        return Ok(*word as u16);
    }
    let low = *shell
        .bytes
        .get(&0x22c)
        .ok_or_else(|| error("zone Kingdom+22C supply low byte uninitialized"))?;
    let high = *shell
        .bytes
        .get(&0x22d)
        .ok_or_else(|| error("zone Kingdom+22D supply high byte uninitialized"))?;
    Ok(u16::from_le_bytes([low, high]))
}

/// Portable identities of actual zone allocations. This is a separate domain
/// from generation-bearing Actor IDs; no native pointer is guessed or cast.
#[derive(Debug, Default)]
pub struct Identities {
    allocations: BTreeMap<u32, (ActorId, u32)>,
    next: u32,
}
impl Identities {
    pub fn new() -> Self {
        Self::default()
    }
    fn allocate(&mut self, owner: ActorId, slot: u32) -> Result<u32> {
        if self.allocations.values().any(|v| *v == (owner, slot)) {
            return Err(error(
                "zone allocation identity already belongs to this Actor slot",
            ));
        }
        let key = self
            .next
            .checked_add(1)
            .ok_or_else(|| error("zone identity domain exhausted"))?;
        self.allocations.insert(key, (owner, slot));
        self.next = key;
        Ok(key)
    }
    pub fn allocation(&self, key: u32) -> Option<(ActorId, u32)> {
        self.allocations.get(&key).copied()
    }
    pub fn len(&self) -> usize {
        self.allocations.len()
    }
    pub fn is_empty(&self) -> bool {
        self.allocations.is_empty()
    }
}

///4D69A0 for a reached Supply/Denizen caller. The vector and newly allocated
///inactive Zone are committed in their actual owner before any later service.
pub(crate) fn create_slot(record:&mut Record<'_>,identities:&mut Identities,index:u32)->Result<bool> {
    let actor=record.shell.actor.as_mut().ok_or_else(||error("4D69A0 actual Actor storage absent"))?;
    let zones=actor.zones_148.zones_mut();
    if zones.slots.is_empty() {
        zones.slots.try_reserve_exact(9).map_err(error)?;
        zones.slots.resize(9,None);
    }
    let entry=zones.slots.get_mut(index as usize).ok_or_else(||error("4D69A0 native zone vector overrun"))?;
    if entry.is_none() {
        let flags=1u16.wrapping_shl(index);
        if record.shell.thing.flags&1!=0 && !aa_zones::zone::kept_on_dead_actor(flags as u8) {
            return required("4D69A0 actual dead-Actor unexpected-zone diagnostic");
        }
        let key=identities.allocate(record.owner,index)?;
        *entry=Some(Zone::new(key,record.owner.raw(),flags));
    }
    Ok(true)
}

///Actual same-zone515F40/514CE0 borrow for a component caller. No zone is
///removed or cloned while its native solver/callbacks execute.
pub(crate) fn operate_slot(
    record:&mut Record<'_>,map:&mut ZoneMap,identities:&Identities,index:u32,
    movement:Option<([u32;2],u32)>,pc:Precision,effects:&mut impl Effects,
)->Result<()> {
    let current=Current {kind:record.shell.kind,thing:&record.shell.thing,
        object:record.shell.object.as_mut().ok_or_else(||error("zone actual Object storage absent"))?,
        definition:record.definition};
    let actor=record.shell.actor.as_mut().ok_or_else(||error("zone actual Actor storage absent"))?;
    let mut world=View {owner:record.owner,flags:&record.shell.thing.flags,
        components:&mut actor.components,words:&actor.words,registry:&mut *record.registry,
        current:Some(current),effects,precision:pc};
    let zone=actor.zones_148.zones_mut().slots.get_mut(index as usize)
        .and_then(Option::as_mut).ok_or(Error::NativeNull("actual component zone receiver"))?;
    if identities.allocation(zone.key)!=Some((record.owner,index)) || zone.actor!=record.owner.raw()
        || zone.flags!=1u16.wrapping_shl(index) {
        return Err(error("component zone allocation identity/owner/type differs"));
    }
    if let Some((position,radius))=movement {
        aa_zones::zone::move_to_position(map,&mut world,zone,position,radius,pc)
    }else{aa_zones::zone::release(map,&mut world,zone,pc)}
}

/// These gates belong to the actual initialized World/presentation owners.
/// A caller must not turn a missing owner into an absent-pointer success.
pub trait Effects {
    fn city_definition(&mut self,_definition:&aa_mapworld::restore::RuntimeDefinition)->Result<()> {
        required("City actual canonical source definition owner")
    }
    fn city_source_byte(&mut self,_definition:&aa_mapworld::restore::RuntimeDefinition,_offset:u32)->Result<u8> {
        required("City actual canonical Body source byte")
    }
    fn city_pending(&mut self,_owner:ActorId,_pc:Precision)->Result<bool> {
        required("City actual current queue head owner")
    }
    fn city_schedule(&mut self,_owner:ActorId,_pc:Precision)->Result<()> {
        required("City actual queue1 and shared RNG owner")
    }
    fn tile_primary_reference(&mut self,_tile:usize)->Result<u32> {
        required("zone same spatial-index blocker/registry owner")
    }
    fn actor_multi_tile(&mut self,_flags:u32,_definition:&aa_mapworld::restore::RuntimeDefinition)->Result<bool> {
        required("zone actual geometry/multi-tile source owner")
    }
    fn actor_tile(&mut self,_cached:[u32;2])->Result<i64> {
        required("zone actual Actor tile/Map identity owner")
    }
    fn zone_effects_enabled(&mut self)->Result<bool> {
        required("zone actual Session+E4 /World+48 owner")
    }
    fn tile_flags(&mut self,_tile:usize)->Result<u32> {
        required("zone same spatial-index tile flags owner")
    }
    fn actor_reveal(
        &mut self,
        _fields: &mut crate::load_detection_memory::Fields<'_>,
        _kingdom: u32,
        _explored: bool,
        _visible: bool,
    ) -> Result<()> {
        required("zone actual mutable Actor64 reveal services")
    }
    fn sai_enabled(&mut self) -> Result<bool> {
        required("zone actual SAI settings+20 owner")
    }
    fn tile_actors(&mut self, _tile: usize) -> Result<Vec<u32>> {
        required("zone same spatial-index actor traversal lease")
    }
    fn fast_detection_zones(&mut self) -> Result<bool> {
        required("native ZoneBuilderFastDetectionZones SVar owner")
    }
    fn fog_enabled(&mut self) -> Result<bool> {
        required("native FogOfWar SVar owner")
    }
    fn solver_present(&mut self, _release: bool) -> Result<bool> {
        required("515A1A actual World+248 /514CE0 World+244 solver allocation owner")
    }
    fn presentation(
        &mut self,
        _owner: u32,
        _x: u32,
        _y: u32,
        _radius: u32,
        _flags: u16,
    ) -> Result<()> {
        required("50C4C0 actual World+D8/presentation+4/UI owner")
    }
}
pub struct RequiredEffects;
impl Effects for RequiredEffects {}

struct View<'a, E> {
    owner: ActorId,
    flags: &'a u32,
    components: &'a mut Components<RestoredComponent>,
    words: &'a BTreeMap<u32, u32>,
    registry: &'a mut aa_actors::IdentityRegistry<RestoredRecord>,
    current: Option<Current<'a>>,
    effects: &'a mut E,
    precision:Precision,
}
struct Current<'a> {
    kind: aa_actors::loading::LoadKind,
    thing: &'a aa_actors::loading::ThingShell<aa_mapworld::bootstrap::DefinitionProof>,
    object: &'a mut aa_actors::loading::ObjectShell,
    definition: &'a aa_mapworld::restore::RuntimeDefinition,
}
fn city_definition_ref<'a>(
    owner:ActorId,current:&'a Option<Current<'_>>,
    registry:&'a aa_actors::IdentityRegistry<RestoredRecord>,actor:ActorId,
)->Result<&'a aa_mapworld::restore::RuntimeDefinition>{
    if actor==owner{return current.as_ref().map(|v|v.definition).ok_or_else(||error("City actual current source absent"))}
    let record=&registry.get(actor).ok_or_else(||error("City source generation stale"))?.value;
    match record{
        RestoredRecord::Object(v)|RestoredRecord::Actor(v)=>Ok(&v.definition),
        RestoredRecord::Kingdom(v)=>Ok(&v.runtime.as_ref().ok_or_else(||error("City actual Kingdom runtime absent"))?.definition),
        _=>required("City actual source-bearing Actor allocation"),
    }
}
impl<E> View<'_, E> {
    fn flags_read(&self,a:u32)->Result<u32> {
        if a==self.owner.raw(){return Ok(*self.flags)}
        Ok(self.prior_object(a)?.0.thing.flags)
    }
    fn special(&self,id:ActorId,class:u32)->Result<&aa_mapworld::special_state::SpecialShell> {
        if id==self.owner {return required("zone relation actual held derived Kingdom fields")}
        let r=&self.registry.get(id).ok_or_else(||error("zone relation current identity stale"))?.value;
        let r=match r {
            RestoredRecord::Kingdom(r) if class==4=>r.runtime.as_ref(),
            RestoredRecord::Team(r) if class==5=>r.runtime.as_ref(),
            _=>return Err(error("zone relation actual class differs")),
        }.ok_or_else(||error("zone relation actual runtime absent"))?;
        if r.definition.native_class!=class || r.definition.proof.category.runtime_class!=class
            || !crate::load_spatial::same_proof(&r.base.thing.definition,&r.definition.proof)
            || (class==4 && !crate::load_spatial::ordinary_layout(class,r.base.kind,r.base.object.is_some(),r.base.actor.is_some()))
            || (class==5 && (r.base.kind!=aa_actors::loading::LoadKind::Thing || r.base.object.is_some() || r.base.actor.is_some())) {
            return Err(error("zone relation actual source/class differs"));
        }
        r.derived.as_ref().ok_or_else(||error("zone relation actual derived fields absent"))
    }
    fn related_code(&self,a:u32,other:u32)->Result<u32> {
        let other=(other!=0).then_some(ActorId::from_raw(other));
        let class=if a==self.owner.raw() {
            self.current.as_ref().ok_or_else(||error("zone current actual definition absent"))?.definition.native_class
        }else{self.prior_object(a)?.1.native_class};
        match class {
            3=> {
                let owner=self.word(a,0xe8)?;
                crate::relations::borrowed::actor(self,(owner!=0).then_some(ActorId::from_raw(owner)),other)
            },
            4=>crate::relations::borrowed::kingdom(self,ActorId::from_raw(a),other),
            _=>required("zone relation Actor virtual144 class dispatch"),
        }
    }
    fn prior_object(&self,actor:u32)->Result<(&aa_actors::loading::LoadShell<aa_mapworld::bootstrap::DefinitionProof,RestoredComponent>,&aa_mapworld::restore::RuntimeDefinition)> {
        let record=&self.registry.get(ActorId::from_raw(actor)).ok_or_else(||error("zone Object generation is not current"))?.value;
        let (shell,definition)=match record {
            RestoredRecord::Object(r)|RestoredRecord::Actor(r)=>(&r.shell,&r.definition),
            RestoredRecord::Kingdom(r)=> {
                let r=r.runtime.as_ref().ok_or_else(||error("zone Kingdom runtime absent"))?;
                (&r.base,&r.definition)
            },
            _=>return required("zone actual Object constructor allocation"),
        };
        if !crate::load_spatial::same_proof(&shell.thing.definition,&definition.proof)
            || definition.proof.category.runtime_class!=definition.native_class {
            return Err(error("zone Object actual source/class differs"));
        }
        Ok((shell,definition))
    }
    fn prior(&self, actor: u32) -> Result<&ActorShell<RestoredComponent>> {
        if actor == self.owner.raw() {
            return required(
                "current Actor zone vector is exclusively leased to its native Zone call",
            );
        }
        let record = &self
            .registry
            .get(ActorId::from_raw(actor))
            .ok_or_else(|| error("zone Actor generation is not current"))?
            .value;
        let shell = match record {
            RestoredRecord::Actor(e) => &e.shell,
            RestoredRecord::Kingdom(e) => {
                &e.runtime
                    .as_ref()
                    .ok_or_else(|| error("zone Kingdom runtime absent"))?
                    .base
            }
            _ => return required("zone callback requires actual Actor constructor storage"),
        };
        shell
            .actor
            .as_ref()
            .ok_or_else(|| error("zone Actor allocation absent"))
    }
    fn word(&self, actor: u32, offset: u32) -> Result<u32> {
        let words = if actor == self.owner.raw() {
            self.words
        } else {
            &self.prior(actor)?.words
        };
        words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("zone actual Actor+{offset:X} is uninitialized")))
    }
    fn components(&self, actor: u32) -> Result<&Components<RestoredComponent>> {
        if actor == self.owner.raw() {
            Ok(self.components)
        } else {
            Ok(&self.prior(actor)?.components)
        }
    }
    fn kingdom_word(&self, key: u32, offset: u32) -> Result<u32> {
        let entry = self
            .registry
            .get(ActorId::from_raw(key))
            .ok_or_else(|| error("zone Kingdom generation is not current"))?;
        let RestoredRecord::Kingdom(e) = &entry.value else {
            return Err(error("zone owner is not an actual Kingdom"));
        };
        let runtime = e
            .runtime
            .as_ref()
            .ok_or_else(|| error("zone Kingdom runtime absent"))?;
        if runtime.definition.native_class != 4 || runtime.base.actor.is_none() {
            return Err(error("zone Kingdom constructor differs"));
        }
        runtime
            .derived
            .as_ref()
            .and_then(|d| d.words.get(&offset))
            .copied()
            .ok_or_else(|| error(format!("zone Kingdom+{offset:X} uninitialized")))
    }
    fn kingdom_supply_mask(&self, key: u32) -> Result<u16> {
        let entry = self
            .registry
            .get(ActorId::from_raw(key))
            .ok_or_else(|| error("zone Kingdom generation is not current"))?;
        let RestoredRecord::Kingdom(e) = &entry.value else {
            return Err(error("zone owner is not an actual Kingdom"));
        };
        let runtime = e
            .runtime
            .as_ref()
            .ok_or_else(|| error("zone Kingdom runtime absent"))?;
        if runtime.definition.native_class != 4 || runtime.base.actor.is_none() {
            return Err(error("zone Kingdom constructor differs"));
        }
        kingdom_supply_mask(
            runtime
                .derived
                .as_ref()
                .ok_or_else(|| error("zone Kingdom derived storage absent"))?,
        )
    }
}
/// A closed-operation loan identity for the actual immutable RuntimeDefinition.
/// This is an Actor registry handle, never a fabricated native source pointer.
/// No callback can replace a definition before its last source-byte read.
#[derive(Clone,Copy)]
struct CityDefinitionRead(ActorId);
impl<E:Effects> crate::zone_city_admission::World for View<'_,E> {
    type Error=Error;type Definition=CityDefinitionRead;
    fn city_owner(&mut self)->Result<u32>{Ok(self.owner.raw())}
    fn actor_flags(&mut self,a:u32)->Result<u32>{self.flags_read(a)}
    fn actor_owner_108(&mut self,a:u32)->Result<u32>{self.word(a,0xe8)}
    fn kingdom_nonmajor_190(&mut self,k:u32)->Result<u32>{self.kingdom_word(k,0x190)}
    fn element_organization(&mut self,a:u32)->Result<Option<u32>>{
        crate::load_city_candidate::organization(self.components(a)?,ActorId::from_raw(a))
            .map(|v|v.map(ActorId::raw)).map_err(error)
    }
    fn actor_value_4d01e0(&mut self,a:u32,pc:Precision)->Result<crate::zone_city_admission::ValueComparison>{
        crate::load_city_candidate::value(self.components(a)?,ActorId::from_raw(a),pc).map_err(error)
    }
    fn body_present_60(&mut self,a:u32)->Result<bool>{
        crate::load_city_candidate::body_present(self.components(a)?,ActorId::from_raw(a)).map_err(error)
    }
    fn definition(&mut self,a:u32)->Result<CityDefinitionRead>{
        if a!=self.owner.raw(){self.prior_object(a)?;}
        let definition=city_definition_ref(self.owner,&self.current,&*self.registry,ActorId::from_raw(a))?;
        self.effects.city_definition(definition)?;
        Ok(CityDefinitionRead(ActorId::from_raw(a)))
    }
    fn source_byte(&mut self,read:CityDefinitionRead,offset:u32)->Result<u8>{
        let definition=city_definition_ref(self.owner,&self.current,&*self.registry,read.0)?;
        self.effects.city_source_byte(definition,offset)
    }
    fn actor_reference_14(&mut self,a:u32)->Result<u32>{
        if a==self.owner.raw(){return Ok(self.owner.raw())}
        self.prior_object(a)?;
        //IdentityRegistry owns native GTID14/generation independently of shell
        //field leases. Resolve on every native read, preserving current lifetime.
        self.registry.get(ActorId::from_raw(a)).ok_or_else(||error("City GTID14 generation stale"))?;
        Ok(a)
    }
    fn accepted_contains_10(&mut self,r:u32)->Result<bool>{
        match self.components.get(ComponentSlot::City){
            Some(RestoredComponent::City(v))if v.header.owner==self.owner=>Ok(v.values.contains(&r)),
            _=>Err(error("City actual accepted-list component/owner differs")),
        }
    }
    fn candidates_contains_18(&mut self,r:u32)->Result<bool>{
        match self.components.get(ComponentSlot::City){
            Some(RestoredComponent::City(v))if v.header.owner==self.owner=>v.candidates.contains(r).map_err(error),
            _=>Err(error("City actual candidate-list component/owner differs")),
        }
    }
    fn candidates_prepend_18(&mut self,r:u32)->Result<()>{
        match self.components.get_mut(ComponentSlot::City){
            Some(RestoredComponent::City(v))if v.header.owner==self.owner=>v.candidates.prepend(r).map(|_|()).map_err(error),
            _=>Err(error("City actual candidate-list component/owner differs")),
        }
    }
    fn pending_c(&mut self)->Result<bool>{self.effects.city_pending(self.owner,self.precision)}
    fn schedule_50ac60(&mut self)->Result<()>{self.effects.city_schedule(self.owner,self.precision)}
}
impl<E> crate::relations::borrowed::Read for View<'_,E> {
    type Id=ActorId;
    type Error=Error;
    fn kingdom_team(&self,id:ActorId)->Result<Option<ActorId>> {
        self.special(id,4)?.identity_references.get(&0x1ec).copied()
            .ok_or_else(||error("zone relation actual resolved Kingdom1EC owner absent"))
    }
    fn kingdom_slot(&self,id:ActorId)->Result<u32> {
        self.special(id,4)?.words.get(&0x188).copied().ok_or_else(||error("zone relation actual Kingdom188 absent"))
    }
    fn team_slot(&self,id:ActorId)->Result<u32> {
        self.special(id,5)?.words.get(&0x38).copied().ok_or_else(||error("zone relation actual Team38 absent"))
    }
    fn kingdom_word(&self,id:ActorId,team:bool,slot:u32)->Result<u32> {
        self.special(id,4)?.vectors.get(&if team{0x1dc}else{0x1d0})
            .and_then(|v|v.values.get(slot as usize)).copied()
            .ok_or_else(||error("zone relation actual Kingdom vector slot absent"))
    }
    fn team_word(&self,id:ActorId,team:bool,slot:u32)->Result<u32> {
        self.special(id,5)?.vectors.get(&if team{0x58}else{0x4c})
            .and_then(|v|v.values.get(slot as usize)).copied()
            .ok_or_else(||error("zone relation actual Team vector slot absent"))
    }
    fn orphan_relation(&self)->Result<u32> {required("zone relation current mutable globalB28158 owner")}
    fn team_null_relation(&self)->Result<u32> {required("zone relation current mutable globalB2815C owner")}
}
impl<E: Effects> aa_zones::query::ZoneActors for View<'_, E> {
    fn owner(&mut self, a: u32) -> Result<u32> {
        self.word(a, 0xe8)
    }
    fn allied(&mut self, a: u32, k: u32) -> Result<bool> {
        let class=if a==self.owner.raw() {
            self.current.as_ref().ok_or_else(||error("zone current actual definition absent"))?.definition.native_class
        }else{self.prior_object(a)?.1.native_class};
        let same=match class {
            3=>self.word(a,0xe8)?==k,
            4=>a==k,
            _=>return required("zone relation Actor virtual138 class dispatch"),
        };
        Ok(same || self.related_code(a,k)?==1)
    }
    fn enemy(&mut self, a: u32, k: u32) -> Result<bool> {
        Ok(self.related_code(a,k)?==3)
    }
    fn zone_slot(&mut self, a: u32, i: u32) -> Result<u32> {
        Ok(self
            .prior(a)?
            .zones_148
            .zones()
            .slots
            .get(i as usize)
            .and_then(|z| z.as_ref())
            .map_or(0, |z| z.key))
    }
    fn word4(&mut self, _a: u32) -> Result<u32> {
        required("zone exclusion requires actual definition identity domain")
    }
    fn component_b0(&mut self, a: u32) -> Result<u32> {
        aa_zones::tile::ZoneWorld::component(self, a, 0xb0)
    }
    fn reveal_mask(&mut self, a: u32) -> Result<Option<u32>> {
        match self.components(a)?.get(ComponentSlot::Detection) {
            None => Ok(None),
            Some(RestoredComponent::Detection(d)) => Ok(Some(d.word_10)),
            _ => Err(error("zone Detection component type differs")),
        }
    }
}
impl<E: Effects> aa_zones::query::VisitWorld for View<'_, E> {
    fn kingdom(&mut self, k: u32) -> Result<aa_zones::query::Kingdom> {
        Ok(aa_zones::query::Kingdom {
            key: k,
            index: self.kingdom_word(k, 0x18c)?,
            nonmajor: self.kingdom_word(k, 0x190)?,
            shared_mask: self.kingdom_word(k, 0x228)?,
            supply_mask: self.kingdom_supply_mask(k)?,
        })
    }
    fn kingdom_slot(&mut self, _i: u32) -> Result<u32> {
        required("zone actual World+154 Kingdom-vector borrow")
    }
}
impl<E: Effects> aa_zones::tile::ZoneWorld for View<'_, E> {
    fn fog_enabled(&mut self) -> Result<bool> {
        self.effects.fog_enabled()
    }
    fn sai_enabled(&mut self) -> Result<bool> {
        self.effects.sai_enabled()
    }
    fn sai(&mut self, _h: aa_zones::tile::SaiHook, _t: usize, _k: u32) -> Result<()> {
        required("zone actual SAI region-counter owner")
    }
    fn kingdom_list(&mut self) -> Result<Vec<u32>> {
        required("zone actual World+154 Kingdom-vector borrow")
    }
    fn tile_actors(&mut self, t: usize) -> Result<Vec<u32>> {
        self.effects.tile_actors(t)
    }
    fn actor_reveal(&mut self, a: u32, k: u32, e: bool, v: bool) -> Result<()> {
        use crate::load_detection_memory::{ActorFields, Fields};
        if a == self.owner.raw() {
            let current = self
                .current
                .as_mut()
                .ok_or_else(|| error("zone current Actor64 actual Object fields unavailable"))?;
            let mut fields = Fields {
                owner: self.owner,
                definition: current.definition,
                kind: current.kind,
                thing: current.thing,
                object: &mut *current.object,
                actor: Some(ActorFields {
                    components: &*self.components,
                    words: self.words,
                }),
                registry: &*self.registry,
            };
            return self.effects.actor_reveal(&mut fields, k, e, v);
        }
        let owner = ActorId::from_raw(a);
        let value = self
            .registry
            .value_mut(owner)
            .ok_or_else(|| error("zone reveal Actor generation is not current"))?;
        let mut held = std::mem::replace(value, RestoredRecord::Loading);
        let result = (|| {
            let (shell, definition) = match &mut held {
                RestoredRecord::Object(r) | RestoredRecord::Actor(r) => {
                    (&mut r.shell, &r.definition)
                }
                RestoredRecord::Kingdom(r) => {
                    let r = r
                        .runtime
                        .as_mut()
                        .ok_or_else(|| error("zone reveal Kingdom runtime absent"))?;
                    (&mut r.base, &r.definition)
                }
                _ => return Err(error("zone reveal actual Object constructor unavailable")),
            };
            let mut record = crate::live_object_init::Record {
                owner,
                shell,
                definition,
                registry: &mut *self.registry,
            };
            let mut fields = Fields::from_record(&mut record).map_err(error)?;
            self.effects.actor_reveal(&mut fields, k, e, v)
        })();
        //Same held allocation returns even when a reached child required service fails.
        *self
            .registry
            .value_mut(owner)
            .expect("held Actor generation retained") = held;
        result
    }
    fn actor_hide(&mut self, _a: u32, _k: u32) -> Result<()> {
        required("zone actual mutable Actor68 hide services")
    }
    fn actor_unexplore(&mut self, _a: u32, _k: u32) -> Result<()> {
        required("zone actual mutable Actor6C unexplore services")
    }
    fn zone_effects_enabled(&mut self) -> Result<bool> {
        self.effects.zone_effects_enabled()
    }
    fn actor_b4(&mut self, a: u32) -> Result<bool> {
        fn kind(kind:aa_actors::loading::LoadKind)->u8 {
            match kind {aa_actors::loading::LoadKind::Thing=>0,aa_actors::loading::LoadKind::Object=>1,aa_actors::loading::LoadKind::Actor=>2}
        }
        if a==self.owner.raw() {
            let c=self.current.as_ref().ok_or_else(||error("zone current actual Object fields absent"))?;
            //operate_slot already holds this actual Actor's components/zones.
            return actor_reads::is_actor(c.definition.native_class,kind(c.kind),true,true).map_err(error);
        }
        let (shell,definition)=self.prior_object(a)?;
        actor_reads::is_actor(definition.native_class,kind(shell.kind),shell.object.is_some(),shell.actor.is_some()).map_err(error)
    }
    fn actor_multi_tile(&mut self, a: u32) -> Result<bool> {
        let flags=aa_zones::tile::ZoneWorld::actor_flags(self,a)?;
        //4EE711 branches before reading GWorld or invoking radius virtual4.
        if flags&0x20000400!=0x20000400 {return Ok(false)}
        let definition=if a==self.owner.raw() {
            self.current.as_ref().ok_or_else(||error("zone current actual definition absent"))?.definition
        }else{
            let record=&self.registry.get(ActorId::from_raw(a)).ok_or_else(||error("zone Actor generation is not current"))?.value;
            match record {
                RestoredRecord::Actor(r)=>&r.definition,
                RestoredRecord::Kingdom(r)=>&r.runtime.as_ref().ok_or_else(||error("zone Kingdom runtime absent"))?.definition,
                _=>return required("zone multi-tile actual Actor constructor"),
            }
        };
        self.effects.actor_multi_tile(flags,definition)
    }
    fn actor_tile(&mut self, a: u32) -> Result<i64> {
        let object=if a==self.owner.raw() {
            &*self.current.as_ref().ok_or_else(||error("zone current actual Object fields absent"))?.object
        }else{
            self.prior_object(a)?.0.object.as_ref().ok_or_else(||error("zone actual Object cached fields absent"))?
        };
        self.effects.actor_tile([object.word_44,object.word_48])
    }
    fn actor_flags(&mut self, a: u32) -> Result<u32> {
        self.flags_read(a)
    }
    fn actor_redirect(&mut self, _a: u32) -> Result<Option<u32>> {
        required("zone actual Denizen redirect owner")
    }
    fn component(&mut self, a: u32, offset: u32) -> Result<u32> {
        if offset == 0x100 {
            return Ok(self.components(a)?.state.low);
        }
        if offset == 0x138 {
            return self.word(a, offset);
        }
        let slot = aa_actors::components::ALL_SLOTS
            .iter()
            .copied()
            .find(|s| 0x60 + 4 * s.index() as u32 == offset)
            .ok_or_else(|| error("zone unknown component offset"))?;
        //Only native nullness is supplied; this is not a component pointer.
        Ok(u32::from(self.components(a)?.get(slot).is_some()))
    }
    fn component_effect(
        &mut self,
        e: aa_zones::tile::ComponentEffect,
        c: u32,
        a: u32,
    ) -> Result<()> {
        if matches!(e,aa_zones::tile::ComponentEffect::Type8) {
            //component() supplied nullness only; resolve the SAME component,
            //never interpret the portable1 as a native allocation pointer.
            if c!=1 {return Err(error("City Type8 component nullness differs"))}
            let city=match self.components.get(ComponentSlot::City) {
                Some(RestoredComponent::City(city)) if city.header.owner==self.owner=>city,
                _=>return Err(error("City Type8 actual held component owner differs")),
            };
            let _=city;
            return crate::zone_city_admission::admit(self,a,self.precision);
        }
        Err(error(format!("required native service: zone actual component effect {e:?}, owner{} target{a}",self.owner.raw())))
    }
}
impl<E: Effects> aa_zones::flood::FloodWorld for View<'_, E> {
    fn tile_flags(&mut self, t: usize) -> Result<u32> {
        self.effects.tile_flags(t)
    }
    fn tile_blocker(&mut self, t: usize) -> Result<u32> {
        let reference=self.effects.tile_primary_reference(t)?;
        Ok(primary_reference::resolve(reference,|raw|self.registry.get(ActorId::from_raw(raw)).is_some()))
    }
    fn assert_blocked_start(&mut self) -> Result<bool> {
        required("zone native assertion diagnostic owner")
    }
}
impl<E: Effects> aa_zones::zone::ZoneObjectWorld for View<'_, E> {
    fn solver_present(&mut self, r: bool) -> Result<bool> {
        self.effects.solver_present(r)
    }
    fn fast_detection_zones(&mut self) -> Result<bool> {
        self.effects.fast_detection_zones()
    }
    fn presentation(&mut self, k: u32, x: u32, y: u32, r: u32, f: u16) -> Result<()> {
        self.effects.presentation(k, x, y, r, f)
    }
    fn log_implicit_clear(&mut self, _a: u32, _f: u16) -> Result<()> {
        required("zone native diagnostic logging owner")
    }
    fn actor_id(&mut self, a: u32) -> Result<u32> {
        if a == self.owner.raw() {
            Ok(a)
        } else {
            self.prior(a).map(|_| a)
        }
    }
}

/// Actual first CreateZone success owns both the vector and Zone value before
/// later move/solver errors. There is no retained copy of the component/pose.
pub fn after_init(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    map: &mut ZoneMap,
    identities: &mut Identities,
    pc: Precision,
    effects: &mut impl Effects,
) -> aa_actors::actor_creation::Result<()> {
    super::load_detection_init_world::checked(catalog, record, ComponentSlot::Detection)?;
    let actor = record.shell.actor.as_ref().unwrap();
    let word138 = *actor.words.get(&0x138).ok_or_else(|| {
        aa_actors::actor_creation::Error::World("56EC90 actual Actor138 absent".into())
    })?;
    let structure = actor.components.state.low & 0x20_0000 != 0;
    if structure && word138 != 0 {
        return Ok(());
    }
    let source = crate::live_object_init::Source::bind(catalog, record.definition)?;
    let radius = if structure {
        catalog
            .actors
            .native_ground_definition(&record.definition.proof.id, source.source_precision())
            .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))?
            .detection
            .ok_or_else(|| {
                aa_actors::actor_creation::Error::World(
                    "56EC90 actual Detection source absent".into(),
                )
            })?
            .finalized_remote_range_bits
    } else {
        if word138 != 0 {
            return Err(aa_actors::actor_creation::Error::Unsupported(
                "56EC90 Actor138 path requires actual source430/Denizen/scheduling services",
            ));
        }
        let Some(RestoredComponent::Detection(d)) = actor.components.get(ComponentSlot::Detection)
        else {
            unreachable!()
        };
        d.radius.0
    };
    let position = record.shell.thing.position_bits;
    let flags = &record.shell.thing.flags;
    let actor = record.shell.actor.as_mut().unwrap();
    let current = Current {
        kind: record.shell.kind,
        thing: &record.shell.thing,
        object: record.shell.object.as_mut().unwrap(),
        definition: record.definition,
    };
    let enabled = structure
        || (*flags & 1 == 0
            && fpu::less(
                &fpu::load(0)
                    .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))?,
                &fpu::load(radius)
                    .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))?,
            ));
    zone_change(
        record.owner,
        actor,
        flags,
        position,
        radius,
        enabled,
        map,
        identities,
        pc,
        &mut *record.registry,
        effects,
        Some(current),
    )
    .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn zone_change(
    owner: ActorId,
    actor: &mut ActorShell<RestoredComponent>,
    flags: &u32,
    position: [u32; 3],
    radius: u32,
    enabled: bool,
    map: &mut ZoneMap,
    identities: &mut Identities,
    pc: Precision,
    registry: &mut aa_actors::IdentityRegistry<RestoredRecord>,
    effects: &mut impl Effects,
    current: Option<Current<'_>>,
) -> Result<()> {
    use aa_zones::{fpu, zone};
    let mut world = View {
        owner,
        flags,
        components: &mut actor.components,
        words: &actor.words,
        registry,
        current,
        effects,
        precision:pc,
    };
    let zones: &mut ActorZones = actor.zones_148.zones_mut();
    if enabled {
        if zones.slots.is_empty() {
            zones.slots.try_reserve_exact(9).map_err(error)?;
            zones.slots.resize(9, None);
        }
        if zones.slots.len() <= 6 {
            return Err(error("native CreateZone6 vector overrun"));
        }
        if zones.slots[6].is_none() {
            let key = identities.allocate(owner, 6)?;
            zones.slots[6] = Some(Zone::new(key, owner.raw(), 0x40));
        }
        let zone = zones.slots[6].as_mut().unwrap();
        if identities.allocation(zone.key) != Some((owner, 6))
            || zone.actor != owner.raw()
            || zone.flags != 0x40
        {
            return Err(error(
                "Detection Zone6 allocation identity/owner/type differs",
            ));
        }
        if zone.active() && fpu::equal(&fpu::load(zone.radius_bits)?, &fpu::load(radius)?) {
            return Ok(());
        }
        //Borrow the actual stored Zone through all callbacks; no copy is
        //temporarily removed and no stale zone is readable through View.
        zone::move_to_position(
            map,
            &mut world,
            zone,
            [position[0], position[1]],
            radius,
            pc,
        )
    } else if let Some(Some(zone)) = zones.slots.get_mut(6) {
        zone::release(map, &mut world, zone, pc)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "load_detection_activation_tests.rs"]
mod tests;
