//! Constructor-owned AIPlayer allocations behind the sole manager+60 slots.
//! Session associations retain the real player allocation's opaque lifetime.
//! No late/default arena or controller is fabricated at world activation.
use crate::{catalog::Catalog,definitions::Bank};
use aa_actors::ActorId;
use aa_mind::sai::{command::SaiCommandData,native_hash::NativeHash,profile::SaiProfile};
use aa_mind::sai::{data::{DataNode,NodeId},profile::{DataManagers,Manager},rules_data::RulesData};
use aa_rules::resources::{Provenance,SourceIdentity};
use std::collections::{BTreeMap,VecDeque};
type Result<T>=std::result::Result<T,String>;

/// Implemented by a proxy retaining the exact opaque SessionPlayer handle.
/// The association is not a numeric player id or an Actor GTID.
//Owned allocation-lifetime association travels with the sole SAI owner. Send
//permits an owned handoff; it does not authorize concurrent player access.
pub trait SessionPlayer:std::fmt::Debug + Send {
    fn validate(&self)->Result<()>;
}
///The same source-created SAI profile manager is available before a World or
///AIPlayer slot exists (776580). This service never constructs controllers.
pub trait ProfileService {
    fn profile_count(&mut self)->Result<usize>;
    fn find_profile(&mut self,name:&[u16])->Result<Option<&Profile>>;
}
pub struct ProfileContext<'a,'source> {
    catalog:&'a Catalog<'source>,bank:&'a Bank,arena:&'a mut Arena,
}
impl<'a,'source> ProfileContext<'a,'source> {
    pub fn new(catalog:&'a Catalog<'source>,bank:&'a Bank,arena:&'a mut Arena)->Self {
        Self {catalog,bank,arena}
    }
}
impl ProfileService for ProfileContext<'_,'_> {
    fn profile_count(&mut self)->Result<usize> {
        self.arena.bind_profiles(self.catalog,self.bank)?;
        Ok(self.arena.profiles.len())
    }
    fn find_profile(&mut self,name:&[u16])->Result<Option<&Profile>> {
        if name.contains(&0)||name.iter().any(|&c|c>127) {return Err("SAI profile Find requires an ASCII, non-NUL native CString identity".into())}
        let name=String::from_utf16(name).map_err(|e|e.to_string())?;
        self.arena.bind_profiles(self.catalog,self.bank)?;
        Ok(self.arena.profiles.iter().find(|p|p.fields.ids.eq_ignore_ascii_case(&name)))
    }
}
/// Borrowed boundary for actual766960 callers without an aa-netcode->aa-mind
/// dependency. Production callers bind AddContext to the constructor owners.
pub trait AddService {
    fn find_profile_name(&mut self,name:&[u16])->Result<Option<String>>;
    fn add(&mut self,kingdom:ActorId,profile:Option<&str>,association:Box<dyn SessionPlayer>)->Result<Option<u32>>;
    fn profile_name(&self,slot:u32)->Result<&str>;
    fn profile_ids(&self,slot:u32)->Result<&str>;
}
pub struct AddContext<'a,'source> {
    catalog:&'a Catalog<'source>,bank:&'a Bank,actors:&'a crate::actor_world::ActorStore,
    manager:&'a mut aa_mind::sai::manager::Sai,arena:&'a mut Arena,
}
impl<'a,'source> AddContext<'a,'source> {
    pub fn new(catalog:&'a Catalog<'source>,bank:&'a Bank,actors:&'a crate::actor_world::ActorStore,
        manager:&'a mut aa_mind::sai::manager::Sai,arena:&'a mut Arena)->Self {
        Self {catalog,bank,actors,manager,arena}
    }
}
impl AddService for AddContext<'_,'_> {
    fn find_profile_name(&mut self,name:&[u16])->Result<Option<String>> {
        if name.contains(&0)||name.iter().any(|&c|c>127) {return Err("SAI profile Find requires an ASCII, non-NUL native CString identity".into())}
        let name=String::from_utf16(name).map_err(|e|e.to_string())?;
        self.arena.bind_profiles(self.catalog,self.bank)?;
        Ok(self.arena.profiles.iter().find(|p|p.fields.ids.eq_ignore_ascii_case(&name)).map(|p|p.fields.ids.clone()))
    }
    fn add(&mut self,kingdom:ActorId,profile:Option<&str>,association:Box<dyn SessionPlayer>)->Result<Option<u32>> {
        self.arena.add_player(self.catalog,self.bank,self.actors,self.manager,kingdom,profile,association)
    }
    fn profile_name(&self,slot:u32)->Result<&str> {
        let handle=self.manager.players.get(slot as usize).copied().flatten().ok_or("SAI selected slot is absent")?;
        self.arena.player(handle)?.validate_association()?;
        Ok(self.arena.profile_at_slot(self.manager,slot)?.name())
    }
    fn profile_ids(&self,slot:u32)->Result<&str> {
        let handle=self.manager.players.get(slot as usize).copied().flatten().ok_or("SAI selected slot is absent")?;
        self.arena.player(handle)?.validate_association()?;
        Ok(self.arena.profile_at_slot(self.manager,slot)?.ids())
    }
}
#[derive(Debug)]
pub struct Profile {
    pub(super) fields:SaiProfile,
    pub(super) source:Provenance,
    pub(super) initial_ego:usize,
}
impl Profile {
    pub fn ids(&self)->&str {&self.fields.ids}
    pub fn name(&self)->&str {&self.fields.name}
    pub fn first_nation(&self)->&str {&self.fields.first_nation}
}

#[derive(Debug)]
struct CommandCell {data:SaiCommandData,word_c:u32}
impl CommandCell {
    fn new()->Self {Self {data:SaiCommandData {actors:Vec::new(),player:None,command:None,ability:None},word_c:0}}
}
/// SAME boxed allocation survives native7B1080 construction and7B4830 startup.
#[derive(Debug)]
pub struct Player {
    slot:u32,
    kingdom:ActorId,
    profile:usize,
    initial_ego:usize,
    association:Box<dyn SessionPlayer>,
    startup:aa_navigation::ai_startup::Initializing,
    arrays:BTreeMap<u32,Vec<u32>>,
    buckets:[Vec<VecDeque<u32>>;2],
    hashes:BTreeMap<u32,NativeHash<u32>>,
    commands:[Vec<CommandCell>;2],
    command_counts:[usize;2],
}
impl Player {
    fn new(slot:u32,kingdom:ActorId,profile:usize,initial_ego:usize,association:Box<dyn SessionPlayer>)->Result<Self> {
        association.validate()?;
        let mut arrays=BTreeMap::new();
        // Only count entries are values; native capacity slack is uninitialized.
        for (off,capacity) in [(0x2c,0),(0x34,0),(0x3c,0),(0x44,0),(0x4c,0),
            (0x9c,0),(0x104,0),(0x12c,0)] {
            let mut v=Vec::new();v.try_reserve_exact(capacity).map_err(|e|e.to_string())?;arrays.insert(off,v);
        }
        let mut hashes=BTreeMap::new();
        for (offset,capacity) in [(0x6c,47),(0x7c,47),(0x8c,47),(0xa4,47),(0xb4,47),(0xc4,47),
            (0xd4,47),(0xe4,47),(0xf4,47),(0x10c,97),(0x11c,97),(0x134,5),(0x150,97),(0x160,97),(0x170,97)] {
            hashes.insert(offset,NativeHash::new(capacity));
        }
        Ok(Self {slot,kingdom,profile,initial_ego,association,
            startup:aa_navigation::ai_startup::Initializing::from_native_constructor().map_err(|e|e.to_string())?,arrays,
            buckets:std::array::from_fn(|_|(0..17).map(|_|VecDeque::new()).collect()),hashes,
            commands:std::array::from_fn(|_|(0..97).map(|_|CommandCell::new()).collect()),command_counts:[0,0]})
    }
    pub fn slot(&self)->u32 {self.slot}
    pub fn kingdom(&self)->ActorId {self.kingdom}
    pub fn ready(&self)->bool {self.startup.ready}
    pub(crate) fn controller_present(&self)->bool {self.startup.controller.is_some()}
    pub fn validate_association(&self)->Result<()> {self.association.validate()}
}

/// Constructed exactly once beside the application's actual native SAI.
/// Manager.players is the only slot vector; this arena owns allocations only.
#[derive(Debug)]
pub struct Arena {
    next:u32,
    allocations:BTreeMap<u32,Box<Player>>,
    profiles:Vec<Profile>,
    ego_ids:Vec<String>,
    source_files:Option<Vec<SourceIdentity>>,
    source_precision:Option<aa_numeric::Precision>,
    manager_lists:BTreeMap<u32,VecDeque<u32>>,
}
impl Arena {
    pub(crate) fn constructed()->Self {Self {next:1,allocations:BTreeMap::new(),profiles:Vec::new(),
        ego_ids:Vec::new(),source_files:None,source_precision:None,manager_lists:BTreeMap::new()}}
    pub fn is_empty(&self)->bool {self.allocations.is_empty()&&self.manager_lists.is_empty()}
    pub(crate) fn has_player_allocations(&self)->bool {!self.allocations.is_empty()}
    ///7A0390's manager+90 vector remains the sole pointer/handle mapping.
    pub(crate) fn prepend_manager_actor(&mut self,vector:&mut Vec<u32>,index:usize,actor:u32)->Result<()> {
        while vector.len()<=index {
            vector.try_reserve(1).map_err(|e|e.to_string())?;
            let handle=self.next;let next=handle.checked_add(1).ok_or("SAI list allocation identity exhausted")?;
            if self.manager_lists.contains_key(&handle)||self.allocations.contains_key(&handle) {return Err("SAI list allocation identity is occupied".into())}
            self.manager_lists.insert(handle,VecDeque::new());self.next=next;vector.push(handle);
        }
        let list=self.manager_lists.get_mut(&vector[index]).ok_or("SAI manager90 list has another allocation owner")?;
        list.try_reserve(1).map_err(|e|e.to_string())?;list.push_front(actor);Ok(())
    }
    pub fn player(&self,handle:u32)->Result<&Player> {
        self.allocations.get(&handle).map(Box::as_ref).ok_or_else(||"SAI player allocation handle is absent".into())
    }
    pub fn profile_for_player(&self,handle:u32)->Result<&Profile> {
        let player=self.player(handle)?;self.profiles.get(player.profile).ok_or_else(||"SAI player canonical profile owner is absent".into())
    }
    ///79EA90 returns the first matching actual+60 slot, in ascending order.
    pub fn profile_for_kingdom(&self,manager:&aa_mind::sai::manager::Sai,kingdom:ActorId)->Result<Option<&Profile>> {
        for (slot,handle) in manager.players.iter().enumerate() {
            let Some(handle)=handle else {continue};let player=self.player(*handle)?;
            if player.slot as usize!=slot {return Err("SAI current slot/allocation owner differs".into())}
            if player.kingdom==kingdom {player.validate_association()?;return self.profile_for_player(*handle).map(Some)}
        }
        Ok(None)
    }
    fn validate_source(&self,catalog:&Catalog<'_>,bank:&Bank)->Result<aa_numeric::Precision> {
        catalog.require_source_owner()?;
        let source=bank.combat_source()?;source.completed_means()?;
        if source.source_identities()!=catalog.actors.sources || self.source_files.as_ref().is_some_and(|s|s!=&catalog.actors.sources)
            || self.source_precision.is_some_and(|p|p!=source.source_precision()) {
            return Err("SAI player arena and actual source Bank/catalog differ".into());
        }
        Ok(source.source_precision())
    }

    fn bind_profiles(&mut self,catalog:&Catalog<'_>,bank:&Bank)->Result<()> {
        let pc=self.validate_source(catalog,bank)?;
        if self.source_files.is_some() {return Ok(())}
        // SAI/Ego are special managers, not generic native_data registries.
        // Ambiguous/reloaded or translated special roots are refused here.
        let mut translation_data=RulesData::new(catalog.actors,pc);
        for root in catalog.actors.resolved_definitions("Translate").map_err(|e|e.to_string())? {
            let node=translation_data.root(root);
            let manager=translation_data.string(node,"node").map_err(|e|e.to_string())?;
            if manager.eq_ignore_ascii_case("SAI")||manager.eq_ignore_ascii_case("Ego") {
                return Err("SAI/Ego special-manager translation source requires actual registration owner".into());
            }
        }
        let ego_roots=catalog.actors.resolved_definitions("Ego").map_err(|e|e.to_string())?;
        let mut ego_ids=Vec::new();let mut seen=std::collections::BTreeSet::new();
        for root in ego_roots {
            let (ids,_)=ids(&root,pc)?;
            if !seen.insert(ids.to_ascii_lowercase()) {return Err("duplicate/reloaded Ego source root requires native overlay".into())}
            ego_ids.push(ids);
        }
        let roots=catalog.actors.resolved_definitions("SAI").map_err(|e|e.to_string())?;
        let mut profiles=Vec::new();seen.clear();
        let mut data=ProfileData {inner:RulesData::new(catalog.actors,pc),catalog,ego_ids:&ego_ids};
        for root in roots {
            let (id,source)=ids(&root,pc)?;
            if !seen.insert(id.to_ascii_lowercase()) {return Err("duplicate/reloaded SAI source root requires native overlay".into())}
            let node=data.inner.root(root);let mut fields=SaiProfile::new(&id);
            aa_mind::sai::profile::load(&mut data,node,&mut fields,pc).map_err(|e|e.to_string())?;
            // DataRefs are prepended while loading; the final pass traverses
            // reverse creation order and each4AE340 prepends the ego index.
            for reference in fields.ego_refs.clone().into_iter().rev() {
                aa_mind::sai::profile::link_ego(&mut data,&mut fields,&reference).map_err(|e|e.to_string())?;
            }
            let initial_ego=usize::try_from(*fields.egos.first().ok_or("SAI profile actual head ego is absent")?).map_err(|e|e.to_string())?;
            if initial_ego>=ego_ids.len() {return Err("SAI profile head ego exceeds actual source registry".into())}
            profiles.push(Profile {fields,source,initial_ego});
        }
        self.ego_ids=ego_ids;self.profiles=profiles;
        self.source_files=Some(catalog.actors.sources.clone());self.source_precision=Some(pc);Ok(())
    }

    ///79FC40 over this actual arena and the SAME constructor-owned manager.
    /// None is native-1 (nonmajor or duplicate); an error after publication
    /// preserves the new slot/worklist prefix, including ready=true refusal.
    pub fn add_player(&mut self,catalog:&Catalog<'_>,bank:&Bank,
        actors:&crate::actor_world::ActorStore,manager:&mut aa_mind::sai::manager::Sai,
        kingdom:ActorId,profile_id:Option<&str>,association:Box<dyn SessionPlayer>)->Result<Option<u32>> {
        let pc=self.validate_source(catalog,bank)?;
        let derived=canonical_kingdom(catalog,bank,actors,kingdom,pc)?;
        if derived.words.get(&0x190).copied().ok_or("SAI actual Kingdom category is unbound")?!=0 {return Ok(None)}
        for (slot,handle) in manager.players.iter().enumerate() {
            if let Some(handle)=handle {
                let player=self.player(*handle)?;
                if player.slot as usize!=slot {return Err("SAI slot vector and actual player allocation disagree".into())}
                if player.kingdom==kingdom {return Ok(None)}
            }
        }
        self.bind_profiles(catalog,bank)?;
        let profile=match profile_id {
            Some(id)=>self.profiles.iter().position(|p|p.fields.ids.eq_ignore_ascii_case(id))
                .ok_or("SAI explicit canonical profile is absent")?,
            None=>{
                let nation=match derived.references.get(&0x234) {
                    Some(Some(d))=>Some(canonical_named_index(catalog,"Nation",&d.id,d.index)?),
                    Some(None)=>None,
                    None if derived.words.get(&0x234)==Some(&0)=>None,
                    None=>return Err("SAI actual Kingdom Nation reference is unbound".into()),
                };
                let faction=match &derived.commander {
                    Some(commander)=>{
                        let entry=bank.entry(commander.definition)?;
                        if entry.key.domain!="faction" {return Err("SAI Commander Faction token has another domain".into())}
                        Some(canonical_named_index(catalog,"Faction",&entry.key.name,None)?)
                    }
                    None if derived.words.get(&0x230)==Some(&0)=>None,
                    None=>return Err("SAI actual Commander pointer/owner is unbound".into()),
                };
                choose_profile(&self.profiles,nation,faction,&mut manager.rng)?
            }
        };
        let slot=manager.players.iter().position(Option::is_none).unwrap_or(manager.players.len());
        let slot_word=u32::try_from(slot).map_err(|e|e.to_string())?;
        let player=Box::new(Player::new(slot_word,kingdom,profile,self.profiles[profile].initial_ego,association)?);
        self.publish(manager,player)?;
        Ok(Some(slot_word))
    }
    fn publish(&mut self,manager:&mut aa_mind::sai::manager::Sai,player:Box<Player>)->Result<()> {
        let slot_word=player.slot;let slot=slot_word as usize;
        if slot>manager.players.len() || manager.players.get(slot).is_some_and(Option::is_some) {
            return Err("SAI publication target is not the selected actual vacant slot".into());
        }
        let handle=self.next;
        let next=handle.checked_add(1).ok_or("SAI allocation identity space exhausted")?;
        self.allocations.insert(handle,player);self.next=next;
        if slot==manager.players.len() {manager.players.push(Some(handle));}else {manager.players[slot]=Some(handle)}
        manager.players_4c.push_front(slot_word);
        manager.players_54.push_front(slot_word);
        if manager.ready {
            return Err("79FC40 published actual player; ready=true addition requires actual7B4830 and current-actor notification services".into());
        }
        Ok(())
    }
    pub fn profile_at_slot(&self,manager:&aa_mind::sai::manager::Sai,slot:u32)->Result<&Profile> {
        let handle=manager.players.get(slot as usize).copied().flatten().ok_or("SAI selected actual slot is absent")?;
        if self.player(handle)?.slot!=slot {return Err("SAI selected slot has another allocation owner".into())}
        self.profile_for_player(handle)
    }
    ///7B4830 mutates this SAME constructor allocation. Later required source
    /// children stop in place, retaining script/controller/goal publication.
    pub(crate) fn start_player(&mut self,catalog:&Catalog<'_>,bank:&Bank,
        actors:&crate::actor_world::ActorStore,kingdoms:&crate::load_kingdom_init_world::KingdomRegistryState,
        map:&crate::load_sai_world::Map,handle:u32)->Result<()> {
        self.bind_profiles(catalog,bank)?;let pc=self.validate_source(catalog,bank)?;
        let p=self.player(handle)?;p.validate_association()?;
        let ego_id=self.ego_ids.get(p.initial_ego).ok_or("SAI initial Ego source owner absent")?.clone();
        if self.profiles.get(p.profile).is_none_or(|profile|profile.initial_ego!=p.initial_ego) {
            return Err("SAI actual Player profile/head Ego differs".into());
        }
        let mut data=StartupData {catalog,bank,actors,kingdoms,kingdom:p.kingdom,slot:p.slot,handle,ego_id,pc,failure:None};
        let player=self.allocations.get_mut(&handle).ok_or("SAI allocation disappeared")?;
        let result=aa_navigation::ai_startup::initialize_from_source(&mut data,
            map.graph().ok_or("SAI AIPlayer requires actual prepared map graph")?,map.regions(),&mut player.startup);
        result.map_err(|e|data.failure.unwrap_or_else(||e.to_string()))
    }
}

struct StartupData<'a,'source> {
    catalog:&'a Catalog<'source>,bank:&'a Bank,actors:&'a crate::actor_world::ActorStore,
    kingdoms:&'a crate::load_kingdom_init_world::KingdomRegistryState,
    kingdom:ActorId,slot:u32,handle:u32,ego_id:String,pc:aa_numeric::Precision,failure:Option<String>,
}
impl StartupData<'_,'_> {
    fn result<T>(&mut self,value:Result<T>,site:&'static str)->aa_navigation::sai_state::Result<T> {
        value.map_err(|e|{self.failure=Some(e);aa_navigation::sai_state::Error::Invalid(site)})
    }
}
fn ego_root<'a>(catalog:&'a Catalog<'_>,pc:aa_numeric::Precision,id:&str)->Result<aa_rules::actors::ResolvedNode<'a>> {
    let mut found=None;
    for node in catalog.actors.resolved_definitions("Ego").map_err(|e|e.to_string())? {
        if ids(&node,pc)?.0.eq_ignore_ascii_case(id) {
            if found.is_some() {return Err("SAI head Ego source registry is ambiguous".into())}found=Some(node);
        }
    }
    found.ok_or_else(||"SAI head Ego source was removed".into())
}
fn ego_ability_counts(catalog:&Catalog<'_>,pc:aa_numeric::Precision,id:&str)->Result<[i32;3]> {
    let root=ego_root(catalog,pc,id)?;let mut data=RulesData::new(catalog.actors,pc);let root=data.root(root);
    let ge=data.section(root,"GoalEngine",true).map_err(|e|e.to_string())?.ok_or("SAI Ego GoalEngine absent")?;
    let ex=data.section(ge,"Exploration",true).map_err(|e|e.to_string())?.ok_or("SAI Ego Exploration absent")?;
    let mut counts=[0;3];
    for (i,name) in ["CloseGoals","DeepGoals","AirReconGoals"].iter().enumerate() {
        let node=data.section(ex,name,true).map_err(|e|e.to_string())?.ok_or("SAI Ego exploration block absent")?;
        counts[i]=data.int(node,"num_goals").map_err(|e|e.to_string())?;
    }
    Ok(counts)
}
fn ego_neighbor_depth(catalog:&Catalog<'_>,pc:aa_numeric::Precision,id:&str)->Result<i32> {
    let root=ego_root(catalog,pc,id)?;let mut data=RulesData::new(catalog.actors,pc);let root=data.root(root);
    let ge=data.section(root,"GoalEngine",true).map_err(|e|e.to_string())?.ok_or("SAI Ego GoalEngine absent")?;
    let combat=data.section(ge,"Combat",true).map_err(|e|e.to_string())?.ok_or("SAI Ego Combat absent")?;
    data.int(combat,"linked_region_distance").map_err(|e|e.to_string())
}
impl aa_navigation::ai_startup::StartupData for StartupData<'_,'_> {
    fn identities(&mut self)->aa_navigation::sai_state::Result<aa_navigation::ai_startup::Identities> {
        let projected=(|| {
            let mut kingdoms=Vec::new();let mut own=None;
            for (i,id) in self.kingdoms.majors().iter().enumerate() {
                kingdoms.push(match id {
                    None=>None,
                    Some(id)=>{
                        let d=canonical_kingdom(self.catalog,self.bank,self.actors,*id,self.pc)?;
                        let index=d.words.get(&0x18c).copied().ok_or("SAI Kingdom major index is unbound")?;
                        let category=d.words.get(&0x190).copied().ok_or("SAI Kingdom category is unbound")?;
                        if index as usize!=i||category!=0 {return Err("SAI current major vector/Kingdom fields differ".into())}
                        if *id==self.kingdom {own=Some(i)}
                        Some(aa_navigation::sai_state::Kingdom {object:aa_navigation::startup::Object(u64::from(id.raw())),native_id:id.raw(),major_index:index as i32,category})
                    },
                });
            }
            Ok(aa_navigation::ai_startup::Identities {
                controller:aa_navigation::startup::Object((1u64<<32)|u64::from(self.handle)),
                player:aa_navigation::startup::Object((2u64<<32)|u64::from(self.handle)),
                player_index:i32::try_from(self.slot).map_err(|e|e.to_string())?,kingdoms,
                own_slot:own.ok_or("SAI actual Player Kingdom is not in its current major slot")?,
                registered_ids:self.actors.identities().iter().map(|e|e.id.raw()).collect(),
            })
        })();self.result(projected,"SAI actual startup identities are unbound")
    }
    fn script_count(&mut self)->aa_navigation::sai_state::Result<usize> {
        let projected=(|| {
            let mut data=RulesData::new(self.catalog.actors,self.pc);
            for root in self.catalog.actors.resolved_definitions("Translate").map_err(|e|e.to_string())? {
                let node=data.root(root);
                if data.string(node,"node").map_err(|e|e.to_string())?.eq_ignore_ascii_case("CompanyTemplate") {
                    return Err("SAI CompanyTemplate translation requires actual special-manager registration owner".into());
                }
            }
            let roots=self.catalog.actors.resolved_definitions("CompanyTemplate").map_err(|e|e.to_string())?;
            let mut seen=std::collections::BTreeSet::new();
            for root in &roots {
                if !seen.insert(ids(root,self.pc)?.0.to_ascii_lowercase()) {
                    return Err("SAI CompanyTemplate reload/duplicate requires actual manager overlay".into());
                }
            }
            Ok(roots.len())
        })();self.result(projected,"SAI CompanyTemplate count is unbound")
    }
    fn ability_counts(&mut self)->aa_navigation::sai_state::Result<[i32;3]> {
        let projected=ego_ability_counts(self.catalog,self.pc,&self.ego_id);
        self.result(projected,"SAI actual Ego exploration counts are unbound")
    }
    fn recruitment(&mut self)->aa_navigation::sai_state::Result<Option<aa_navigation::ai_startup::Recruitment>> {
        let projected=(|| {
            let d=canonical_kingdom(self.catalog,self.bank,self.actors,self.kingdom,self.pc)?;
            if d.commander.is_some() {return Err("7B4830 actual Commander ability allocation references and Ego884 owner are required".into())}
            if d.words.get(&0x230)!=Some(&0) {return Err("SAI actual Commander null pointer owner is unbound".into())}
            Ok(None)
        })();self.result(projected,"SAI actual Commander recruitment is unbound")
    }
    fn owned_units(&mut self)->aa_navigation::sai_state::Result<Vec<Option<aa_navigation::startup::Object>>> {
        let projected=(|| {
            let d=canonical_kingdom(self.catalog,self.bank,self.actors,self.kingdom,self.pc)?;
            if !d.recruit_items.singles.is_empty() {
                return Err("7B4830 actual RecruitSingle allocation references are required for Kingdom214 goals".into());
            }
            Ok(Vec::new())
        })();self.result(projected,"SAI actual Kingdom RecruitSingle list is unbound")
    }
    fn neighbor_depth(&mut self)->aa_navigation::sai_state::Result<i32> {
        let projected=ego_neighbor_depth(self.catalog,self.pc,&self.ego_id);
        self.result(projected,"SAI actual Ego linked-region distance is unbound")
    }
}

fn choose_profile(profiles:&[Profile],nation:Option<u32>,faction:Option<u32>,rng:&mut aa_mind::sai::manager::SaiRng)->Result<usize> {
    let eligible=profiles.iter().enumerate().filter(|(_,p)|p.fields.difficulty>0 && p.fields.public
        && nation.is_none_or(|n|p.fields.nations.is_empty()||p.fields.nations.contains(&n))
        && faction.is_none_or(|f|p.fields.factions.is_empty()||p.fields.factions.contains(&f)))
        .map(|(i,_)|i).collect::<Vec<_>>();
    let count=if eligible.is_empty() {profiles.len()}else{eligible.len()};
    let draw=rng.next_u32(); //native advances before a possible DIV0 fault
    let index=draw.checked_rem(u32::try_from(count).map_err(|e|e.to_string())?).ok_or("79FAF0 profile registry has zero entries after RNG advance")?as usize;
    Ok(if eligible.is_empty(){index}else{eligible[index]})
}
fn canonical_named_index(catalog:&Catalog<'_>,tag:&str,id:&str,index:Option<u32>)->Result<u32> {
    let registry=catalog.actors.native_data_registry(tag).ok_or_else(||format!("SAI actual {tag} source registry absent"))?;
    let mut matches=registry.entries.iter().enumerate().filter(|(_,e)|e.ids.eq_ignore_ascii_case(id));
    let (i,entry)=matches.next().ok_or_else(||format!("SAI canonical {tag}/{id} source absent"))?;
    if matches.next().is_some() || registry.find(id).is_none_or(|e|!std::ptr::eq(e,entry)) || index.is_some_and(|v|v as usize!=i) {
        return Err("SAI canonical source identity/ordinal is shadowed or differs".into());
    }
    u32::try_from(i).map_err(|e|e.to_string())
}
pub(crate) fn canonical_kingdom<'a>(catalog:&Catalog<'_>,bank:&Bank,actors:&'a crate::actor_world::ActorStore,
    id:ActorId,pc:aa_numeric::Precision)->Result<&'a aa_mapworld::special_state::SpecialShell> {
    let key=actors.key(id).ok_or("SAI Kingdom current generation is absent")?;
    let object=actors.object(key).map_err(|e|e.to_string())?;
    let crate::actor_world::Source::Special(special)=&actors.entity(key).map_err(|e|e.to_string())?.source else {return Err("SAI Kingdom derived owner absent".into())};
    let crate::actor_world::SpecialSource::Kingdom(saved)=&special.source else {return Err("SAI actual object is not constructed Kingdom".into())};
    let proof=&object.definition.proof;catalog.require_untranslated_actor_identity(&proof.id)?;
    let canonical=catalog.actors.find(&proof.id).ok_or("SAI Kingdom canonical source absent")?;
    let source=bank.combat_source()?;let token=bank.token("ActorDefinition",&proof.id).ok_or("SAI Kingdom source token absent")?;
    source.completed_values(token)?;
    if saved.serialized_id!=id.raw() || object.definition.native_class!=4 || object.kind!=aa_actors::loading::LoadKind::Actor
        || object.actor.is_none()||object.object.is_none() || canonical.source!=proof.source||canonical.template_chain!=proof.template_chain
        || catalog.actors.native_category(&proof.id).map_err(|e|e.to_string())?!=proof.category
        || !crate::load_spatial::same_proof(&object.thing.definition,proof)
        || !source.definition_id(token)?.eq_ignore_ascii_case(&proof.id)||source.definition_source(token)?!=&proof.source
        || object.definition.component_mask!=Some(catalog.canonical_component_mask(&proof.id,pc)?) {
        return Err("SAI Kingdom actual constructor/source certificate differs".into());
    }
    special.derived.as_ref().ok_or_else(||"SAI Kingdom derived allocation absent".into())
}

fn ids(root:&aa_rules::actors::ResolvedNode<'_>,pc:aa_numeric::Precision)->Result<(String,Provenance)> {
    match root.field("IDS",pc).map_err(|e|e.to_string())? {
        Some(aa_rules::native::TypedValue::String {value,source,..}) if !value.is_empty()&&value.is_ascii()=>Ok((value,source)),
        _=>Err("SAI/Ego root has no actual typed IDS".into()),
    }
}
struct ProfileData<'a,'source> {inner:RulesData<'a>,catalog:&'a Catalog<'source>,ego_ids:&'a [String]}
impl DataNode for ProfileData<'_,'_> {
    fn fixed(&mut self,n:NodeId,k:&str)->aa_mind::Result<aa_numeric::Binary> {self.inner.fixed(n,k)}
    fn int(&mut self,n:NodeId,k:&str)->aa_mind::Result<i32> {self.inner.int(n,k)}
    fn boolean(&mut self,n:NodeId,k:&str)->aa_mind::Result<bool> {self.inner.boolean(n,k)}
    fn string(&mut self,n:NodeId,k:&str)->aa_mind::Result<String> {self.inner.string(n,k)}
    fn localized(&mut self,n:NodeId,k:&str)->aa_mind::Result<String> {
        if k!="name" {return Err(aa_mind::Error::World("unsupported SAI localized getter".into()))}
        let table=self.catalog.definition_text.as_ref().ok_or_else(||aa_mind::Error::World("SAI definition-time localization owner is absent".into()))?;
        let value=aa_rules::native::display_name::native_stored_name(&self.inner.nodes[n as usize],&mut |key|table.lookup(key)
            .map_err(|e|aa_rules::native::Error::Unsupported(e.to_string())))
            .map_err(|e|aa_mind::Error::World(e.to_string()))?;
        String::from_utf16(&value).map_err(|e|aa_mind::Error::World(e.to_string()))
    }
    fn count(&mut self,n:NodeId,k:&str)->aa_mind::Result<i32> {self.inner.count(n,k)}
    fn item(&mut self,n:NodeId,k:&str,i:i32,c:i32)->aa_mind::Result<NodeId> {self.inner.item(n,k,i,c)}
    fn section(&mut self,n:NodeId,k:&str,r:bool)->aa_mind::Result<Option<NodeId>> {self.inner.section(n,k,r)}
    fn data_error(&mut self,f:&str,a:&[String])->aa_mind::Error {self.inner.data_error(f,a)}
}
impl DataManagers for ProfileData<'_,'_> {
    fn find(&mut self,m:Manager,id:&str)->aa_mind::Result<Option<(u32,i32)>> {
        if m==Manager::Egos {
            return Ok(self.ego_ids.iter().position(|s|s.eq_ignore_ascii_case(id)).map(|i|(i as u32,i as i32)));
        }
        let tag=match m {Manager::Nations=>"Nation",Manager::Factions=>"Faction",_=>return Err(aa_mind::Error::World("unsupported SAI profile manager".into()))};
        let registry=self.catalog.actors.native_data_registry(tag).ok_or_else(||aa_mind::Error::World(format!("actual {tag} source registry absent")))?;
        Ok(registry.find(id).map(|e| {let i=registry.entries.iter().position(|p|std::ptr::eq(p,e)).unwrap();(i as u32,i as i32)}))
    }
}

#[cfg(test)]
#[path="load_sai_players_tests.rs"]
mod tests;
