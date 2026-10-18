//! Native4F8AE0→4F6280, including the unconditional authoritative RNG draw.
//! SyncThings follows actual GTID slot order, independently of Actor flags.
//! The same native log wrappers are invoked even when logging is disabled.
use crate::{actor_world::ActorStore,catalog::Catalog,definitions::Bank,
    launch_settings_dump::{Sink,SyncSink},live_clocked::SyncBinding,load_world::World};
use aa_actors::ActorId;
use aa_numeric::Precision;
use aa_simulation::{NativeClock,NativeRng};
type Result<T>=std::result::Result<T,String>;

///Current World140/184/194/198 storage. Source serialization is immutable
///evidence; these are the current fields retained with the actual World.
pub(crate) struct Counters{player:u32,camera:u32,area:u32,structure:u32}
impl Counters{
    ///4F7D00 explicitly stores zero to all four fields, independently of heap
    ///fill.4FC600 always replaces140, replaces184/194 atv4+, and198 atv27+.
    ///None means that exact native load store is absent and retains its ctor
    ///field. Fresh queue-item/camera/area producers must mutate THIS owner.
    pub(crate) fn from_loaded_world(v:&aa_mapworld::WorldState)->Self{
        Self{player:v.word_140,camera:v.word_184.unwrap_or(0),area:v.word_194.unwrap_or(0),structure:v.word_198.unwrap_or(0)}
    }
    fn read(&self,offset:u32)->Result<u32>{
        match offset{
            0x140=>Ok(self.player),
            0x184=>Ok(self.camera),
            0x194=>Ok(self.area),
            0x198=>Ok(self.structure),
            _=>Err("initial sync counter offset is outside native owner".into()),
        }
    }
}
fn definition_index(catalog:&Catalog<'_>,manager:&str,ids:&[u16])->Result<u32>{
    crate::launch_settings_dump::definition_index(catalog,manager,ids)
}
fn thing(world:&impl Read,id:Option<ActorId>,sink:&mut impl Sink)->Result<()>{
    let Some(id)=id else{return sink.boolean("false",false)};
    sink.float("_thing->m_created_time",world.thing(id,0x1c)?)?;
    sink.int("(int)_thing->m_id",world.thing(id,0x14)?)?;
    for (offset,label) in [(0x20,"_thing->m_position.x"),(0x24,"_thing->m_position.y"),(0x28,"_thing->m_position.z")]{sink.float(label,world.thing(id,offset)?)?;}
    Ok(())
}
fn count(n:usize)->Result<u32>{u32::try_from(n).map_err(|_|"native initial sync vector count exceeds wire representation".into())}
/// Private field projections keep the one native traversal shared by the
/// actual-owner binding and explicit native-oracle storage. No public raw
/// World/activation constructor is introduced by this testing seam.
trait Read{
    fn word(&self,offset:u32)->Result<u32>;
    fn rule_header(&self,field:RuleHeader)->Result<u32>;
    fn rule_count(&self)->usize;
    fn rule(&self,index:usize,field:RuleField)->Result<u32>;
    fn team(&self,index:usize)->Result<Option<ActorId>>;
    fn kingdom(&self,index:usize)->Result<Option<ActorId>>;
    fn thing(&self,id:ActorId,offset:u32)->Result<u32>;
    fn things(&self)->impl Iterator<Item=ActorId> + '_;
}
#[derive(Clone,Copy)]
enum RuleField{Definition,Locked,Kind,Value}
#[derive(Clone,Copy)]
enum RuleHeader{Mode,GameType}
fn rules(world:&impl Read,sink:&mut impl Sink)->Result<()>{
    sink.marker("m_world_rules")?;
    sink.int("_wr.m_map_type",world.rule_header(RuleHeader::Mode)?)?;
    sink.int("_wr.m_game_type",world.rule_header(RuleHeader::GameType)?)?;
    sink.int("(int)size",count(world.rule_count())?)?;
    for index in 0..world.rule_count(){
        sink.marker("(*iter)")?;
        sink.int("_wr.data",world.rule(index,RuleField::Definition)?)?;
        sink.boolean("_wr.locked",world.rule(index,RuleField::Locked)? as u8!=0)?;
        match world.rule(index,RuleField::Kind)?{
            0=>sink.boolean("_wr.value_bool",world.rule(index,RuleField::Value)? as u8!=0)?,
            1=>sink.int("_wr.value_int",world.rule(index,RuleField::Value)?)?,
            2=>sink.float("_wr.value_fixed",world.rule(index,RuleField::Value)?)?,
            3=>sink.int("_wr.value_enum",world.rule(index,RuleField::Value)?)?,
            _=>return Err("initial sync native WorldRule kind is unsupported".into()),
        }
    }Ok(())
}
pub(crate) struct Fields<'a>{
    pub svars:&'a crate::load_view_variables::State,
    pub counters:&'a Counters,
    pub rules:&'a crate::load_world_rules::Owner,
    pub mission:&'a crate::mission_fields::State,
    pub runtime:&'a crate::load_world_runtime_fields::State,
    pub misc:&'a crate::saved_misc::State,
    pub registration:&'a crate::live_registration::State,
    pub objectives:&'a crate::world_objectives::State,
    pub kingdoms:&'a crate::load_kingdom_init_world::KingdomRegistryState,
}
impl<'a> Fields<'a>{
    pub(crate) fn from_world(world:&'a World)->Self{Self{
        svars:&world.variables,
        counters:&world.sync_counters,rules:&world.world_rules,mission:&world.mission_fields,
        runtime:&world.runtime_fields,misc:&world.misc,registration:&world.registration,
        objectives:&world.objectives,kingdoms:&world.kingdoms,
    }}
}
///The actual live owners are loaned directly. Clock/RNG are the same retained
///clocked-state fields; this never reconstructs a Staged graph or log owner.
pub(crate) fn synchronize(
    world:&World,actors:&ActorStore,bank:&Bank,catalog:&Catalog<'_>,
    clock:&NativeClock,rng:&mut NativeRng,precision:Precision,
    label:&str,binding:&mut SyncBinding<'_>,
)->Result<()>{
    synchronize_fields(Fields::from_world(world),world.live_spatial()?,actors,bank,catalog,clock,rng,precision,label,binding)
}
pub(crate) fn synchronize_fields(
    fields:Fields<'_>,spatial:&crate::spatial_world::SpatialWorld,
    actors:&ActorStore,bank:&Bank,catalog:&Catalog<'_>,clock:&NativeClock,
    rng:&mut NativeRng,precision:Precision,label:&str,binding:&mut SyncBinding<'_>,
)->Result<()>{
    fields.rules.validate_in_bank(catalog,bank)?;
    fields.mission.require_loaded()?;
    fields.runtime.require_initialized()?;
    let mut sink=SyncSink{binding};
    traverse(&Bound{fields,spatial,actors,bank,catalog,clock},rng,precision,label,&mut sink)
}
struct Bound<'a,'source>{fields:Fields<'a>,spatial:&'a crate::spatial_world::SpatialWorld,
    actors:&'a ActorStore,bank:&'a Bank,catalog:&'a Catalog<'source>,clock:&'a NativeClock}
impl Read for Bound<'_,'_>{
    fn word(&self,offset:u32)->Result<u32>{match offset{
        0xdc=>Ok(self.clock.tick),0xe0=>Ok(self.clock.begin.0),0xe4=>Ok(self.clock.end.0),0xe8=>Ok(self.clock.step.0),
        0x140|0x184|0x194|0x198=>self.fields.counters.read(offset),
        0x44=>Ok(self.fields.mission.mode44()),0x48=>Ok(self.fields.mission.started48()),
        0x275=>Ok(u32::from(*self.fields.runtime.fog_cheat_word())),0x278=>Ok(self.fields.misc.pre_cutscene_speed),
        0x170=>count(self.fields.registration.teams().len()),0x25c=>self.fields.objectives.word_25c(),
        0x158=>count(self.fields.kingdoms.majors().len()),0x14c=>count(self.fields.kingdoms.all().len()),
        0x290=>match &self.fields.misc.weather{
            crate::saved_misc::Reference::Null=>Ok(0),
            crate::saved_misc::Reference::Uninitialized=>Err("initial sync actual Weather pointer is uninitialized".into()),
            crate::saved_misc::Reference::Definition(token)=>{
                let entry=self.bank.entry(*token)?;
                if entry.key.domain!="weather"{return Err("initial sync Weather belongs to another definition manager".into())}
                definition_index(self.catalog,"Weather",&entry.key.name.encode_utf16().collect::<Vec<_>>())
            }
        },
        _=>Err(format!("initial sync World+{offset:X} projection is unbound")),
    }}
    fn rule_header(&self,field:RuleHeader)->Result<u32>{match field{
        RuleHeader::Mode=>Ok(self.fields.rules.mode()),
        RuleHeader::GameType=>definition_index(self.catalog,"GameType",self.fields.rules.game_type()),
    }}
    fn rule_count(&self)->usize{self.fields.rules.records().count()}
    fn rule(&self,index:usize,field:RuleField)->Result<u32>{
        let record=self.fields.rules.records().nth(index).ok_or("initial sync rule index exceeds same owner")?;
        match field{RuleField::Definition=>definition_index(self.catalog,"WorldRule",&record.ids().encode_utf16().collect::<Vec<_>>()),
            RuleField::Locked=>Ok(u32::from(record.locked())),RuleField::Kind=>Ok(record.kind()),RuleField::Value=>Ok(record.value())}
    }
    fn team(&self,index:usize)->Result<Option<ActorId>>{self.fields.registration.teams().get(index).copied().ok_or("initial sync Team index exceeds same vector".into())}
    fn kingdom(&self,index:usize)->Result<Option<ActorId>>{self.fields.kingdoms.all().get(index).copied().ok_or("initial sync Kingdom index exceeds same vector".into())}
    fn thing(&self,id:ActorId,offset:u32)->Result<u32>{
        let key=self.actors.key(id).ok_or("initial sync actual referenced Thing is not live")?;
        match offset{
            0x14=>Ok(id.raw()),
            0x1c=>self.actors.object(key).map_err(|e|e.to_string())?.thing.word_1c.ok_or("initial sync actual Thing created-time producer is unbound".into()),
            0x20|0x24|0x28=>Ok(self.actors.placement(key,self.spatial).map_err(|e|e.to_string())?.position_bits[((offset-0x20)/4)as usize]),
            _=>Err("initial sync Thing projection is unbound".into()),
        }
    }
    fn things(&self)->impl Iterator<Item=ActorId> + '_{self.actors.identities().iter().map(|e|e.id)}
}
fn traverse(world:&impl Read,rng:&mut NativeRng,precision:Precision,label:&str,sink:&mut impl Sink)->Result<()>{
    sink.marker(label)?;
    //4F62BB..DF: one actual85D840 draw precedes every scalar logger.
    let random=rng.next_f32(precision).map_err(|e|e.to_string())?;
    sink.float("m_rng.GetFRand()",random.0)?;
    sink.int("m_game_ticks",world.word(0xdc)?)?;
    for(offset,label)in[(0xe0,"m_game_time_begin_tick"),(0xe4,"m_game_time_end_tick"),(0xe8,"m_game_time_per_tick")]{sink.float(label,world.word(offset)?)?;}
    for(offset,label)in[(0x140,"m_next_gplayer_id"),(0x184,"m_next_camera_id"),(0x194,"m_next_area_id"),(0x198,"m_next_structure_queue_item_id"),(0x44,"m_map_status"),(0x48,"m_mission_state")]{sink.int(label,world.word(offset)?)?;}
    rules(world,sink)?;
    sink.boolean("m_cutscene_mode",world.word(0x275)? as u8!=0)?;
    sink.float("m_pre_cutscene_game_speed",world.word(0x278)?)?;
    sink.int("m_all_teams.Size()",world.word(0x170)?)?;
    for index in 0..world.word(0x170)?{thing(world,world.team(index as usize)?,sink)?;}
    sink.int("m_next_objective_id",world.word(0x25c)?)?;
    sink.int("m_major_kingdoms.Size()",world.word(0x158)?)?;
    sink.int("m_all_kingdoms.Size()",world.word(0x14c)?)?;
    for index in 0..world.word(0x14c)?{thing(world,world.kingdom(index as usize)?,sink)?;}
    sink.marker("SyncThings")?;
    for id in world.things(){
        sink.int("(int)thing->GetID()",world.thing(id,0x14)?)?;
        sink.float("thing->GetPosition().x",world.thing(id,0x20)?)?;
        sink.float("thing->GetPosition().y",world.thing(id,0x24)?)?;
    }
    sink.int("m_active_weather",world.word(0x290)?)
}
#[cfg(test)]
#[path="initial_world_sync_tests.rs"]
mod tests;
