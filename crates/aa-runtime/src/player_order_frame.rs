//!Accepted86/87 callbacks on the actual Frame graph. Native allocation,
//!permission, selection and Mind dispatch remain the existing algorithms.
//!State effects reuse the InitExisting service implementation on the SAME
//!owners; no fresh InitExisting/Runtime, simulated actor or alternate Bank.
use crate::{actor_world::{ActorAccess,Spatial,PlacementState},catalog::Catalog,
    live_world_frame::GameplayOrderLoans,mind_world::{self,Services as _},
    player_order_decree::{self as decree,Decoded},player_order_world::{self as orders,Banks,DefinitionBankLoan}};
use aa_actors::ActorId;
use aa_mind::{Arena,MindId,StateId,command,construct::Kind};
use std::cell::RefCell;
fn mind_error(e:impl std::fmt::Display)->aa_mind::Error{aa_mind::Error::World(e.to_string())}
pub(crate) struct Fields<'a>{pub(crate) spatial:&'a mut crate::spatial_world::SpatialWorld,
    pub(crate) variables:&'a mut crate::load_view_variables::State,pub(crate) random:&'a mut aa_simulation::NativeRng,
    pub(crate) clock:&'a aa_simulation::NativeClock,pub(crate) inventory:&'a aa_mapworld::WorldInventory,
    pub(crate) presentation:&'a mut crate::fx_manager::State<crate::fx_graph::Command>,
    pub(crate) zones:&'a aa_zones::tile::ZoneMap,
    pub(crate) current_settings:crate::stored_actor_state_effects::CurrentSettings<'a>,
    pub(crate) terrain:Option<&'a crate::battle_terrain::BattleTerrain>,pub(crate) mission:Option<&'a crate::mission_fields::State>,pub(crate) phase:Option<&'a u32>}
pub(crate) struct Services<'a,'owner,'source>{pub(crate) catalog:&'a Catalog<'source>,pub(crate) bank:&'a DefinitionBankLoan<'owner>,
    pub(crate) fields:&'a RefCell<Fields<'owner>>,pub(crate) pc:aa_numeric::Precision}
impl Services<'_,'_,'_>{
    fn with<T>(&self,call:impl FnOnce(&mut crate::stored_mind_init_world::Bound<'_,'_>)->aa_mind::Result<T>)->aa_mind::Result<T>{
        let mut bank=self.bank.borrow_bank_mut().map_err(mind_error)?;
        let mut f=self.fields.try_borrow_mut().map_err(mind_error)?;
        let Fields{spatial,variables,random,clock,inventory,presentation,zones,terrain,mission,phase,current_settings}=&mut *f;
        call(&mut crate::stored_mind_init_world::Bound{catalog:self.catalog,bank:&mut bank,
            spatial,variables,random,clock,inventory,presentation,zones,
            terrain:*terrain,mission:*mission,phase:*phase,current_settings:*current_settings,pc:self.pc})
    }
}
macro_rules! delegate_mut{($name:ident($($arg:ident:$ty:ty),*)->$out:ty)=>{
    fn $name(&mut self,$($arg:$ty),*)->aa_mind::Result<$out>{self.with(|bound|bound.$name($($arg),*))}
};}
impl mind_world::Services for Services<'_,'_,'_>{
    delegate_mut!(can_activate_other(s:StateId,b:mind_world::StateBinding,c:&mut mind_world::Context<'_,'_>)->bool);
    delegate_mut!(enter_other(s:StateId,b:mind_world::StateBinding,c:&mut mind_world::Context<'_,'_>)->());
    delegate_mut!(leave_other(s:StateId,b:mind_world::StateBinding,c:&mut mind_world::Context<'_,'_>)->());
    delegate_mut!(actor_state_other(s:StateId,b:mind_world::StateBinding,c:&mut mind_world::Context<'_,'_>)->u8);
    delegate_mut!(set_actor_state(a:ActorId,i:u8,c:&mut mind_world::Context<'_,'_>)->());
    delegate_mut!(rebuild_actions(a:ActorId,c:&mut mind_world::Context<'_,'_>)->());
    delegate_mut!(default_stance(a:ActorId,c:&mut mind_world::Context<'_,'_>)->());
    delegate_mut!(reset_other(m:MindId,k:Kind,c:&mut mind_world::Context<'_,'_>)->());
    delegate_mut!(sync_state_other(m:MindId,k:Kind,c:&mut mind_world::Context<'_,'_>)->());
    delegate_mut!(set_facing(a:ActorId,f:[u32;2],c:&mut mind_world::Context<'_,'_>)->());
    delegate_mut!(finish_group_commands(a:ActorId,c:&mut mind_world::Context<'_,'_>)->());
    delegate_mut!(invoke_command(m:MindId,s:u32,v:u32,cc:&mut command::Context,c:&mut mind_world::Context<'_,'_>)->bool);
    delegate_mut!(deliver_stop(a:ActorId,c:&mut mind_world::Context<'_,'_>)->bool);
    delegate_mut!(defers_other(s:StateId,b:mind_world::StateBinding,c:&mut mind_world::Context<'_,'_>)->bool);
    fn facing(&self,a:ActorId,actors:&ActorAccess<'_>)->aa_mind::Result<[u32;2]>{self.with(|bound|bound.facing(a,actors))}
    fn actor_flags(&self,a:ActorId,actors:&ActorAccess<'_>)->aa_mind::Result<u32>{self.with(|bound|bound.actor_flags(a,actors))}
    fn has_live_group(&self,a:ActorId,actors:&ActorAccess<'_>)->aa_mind::Result<bool>{self.with(|bound|bound.has_live_group(a,actors))}
}
struct SpatialRead<'a,'owner>(&'a RefCell<Fields<'owner>>);
impl Spatial for SpatialRead<'_,'_>{
    fn placement(&self,id:ActorId)->crate::actor_world::Result<PlacementState>{
        let fields=self.0.try_borrow().map_err(|_|crate::actor_world::Error::Missing("actual order placement callback remains borrowed"))?;
        fields.spatial.placement(id)
    }
    fn publish(&mut self,_:ActorId,_:PlacementState)->crate::actor_world::Result<()>{Err(crate::actor_world::Error::Missing("readonly order placement adapter cannot publish"))}
}
struct Groups;
impl decree::Groups for Groups{
    fn prepare(&mut self,_:&Catalog<'_>,_:&mut ActorAccess<'_>,_:&mut mind_world::Runtime,_:&mut Arena,_:ActorId,_:ActorId)->orders::Result<()>{
        Err(orders::Error::Required("actual4DFF30 linked Group command preparation".into()))
    }
}
///Only the accepted Actor decree calls this. Admin normalization/logging and
///player/client guards happen earlier on the actual retained Admin streams.
pub fn actor(catalog:&Catalog<'_>,parts:GameplayOrderLoans<'_>,wire:&aa_orders::commands::ActorCommandWire,
    globals:&mut impl orders::Globals)->Result<aa_orders::commands::ActorOutcome,String>{
    let bank=DefinitionBankLoan::new(parts.definitions);
    let fields=RefCell::new(Fields{spatial:parts.spatial,variables:parts.variables,random:parts.rng,
        clock:parts.clock,inventory:parts.inventory,presentation:parts.presentation,zones:parts.zones,
        terrain:Some(parts.terrain),mission:Some(parts.mission),phase:Some(parts.phase),current_settings:parts.current_settings});
    let spatial=SpatialRead(&fields);let mut services=Services{catalog,bank:&bank,fields:&fields,pc:parts.precision};
    let(arena,actors)=parts.actors.split_minds();
    let mut world=mind_world::World{actors,runtime:parts.mind_runtime,queues:parts.queues,owners:parts.owners,services:&mut services};
    decree::issue_actor_in_bank(catalog,&bank,&mut world,arena,&spatial,globals,&mut Groups,wire).map_err(|e|e.to_string())
}
struct SelectionDiagnostics;
impl decree::Diagnostics for SelectionDiagnostics{
    fn selection(&mut self,_:&Catalog<'_>,_:&ActorAccess<'_>,_:&mind_world::Commands,_:&impl crate::player_selection::View,_:u32,_:bool)->orders::Result<()>{
        Err(orders::Error::Required("actual61F210 selected-command description/diagnostic service".into()))
    }
    fn command_sync(&mut self,_:&mind_world::Commands,_:u32)->orders::Result<()>{
        Err(orders::Error::Required("actual4D9A40 command synchronizer service".into()))
    }
}
pub fn selected(catalog:&Catalog<'_>,parts:GameplayOrderLoans<'_>,wire:&aa_orders::commands::CommandWire,
    globals:&mut impl orders::Globals,slot:&u32,view:&mut impl crate::player_selection::View)->Result<aa_orders::commands::SelectedOutcome,String>{
    let bank=DefinitionBankLoan::new(parts.definitions);
    let fields=RefCell::new(Fields{spatial:parts.spatial,variables:parts.variables,random:parts.rng,
        clock:parts.clock,inventory:parts.inventory,presentation:parts.presentation,zones:parts.zones,
        terrain:Some(parts.terrain),mission:Some(parts.mission),phase:Some(parts.phase),current_settings:parts.current_settings});
    let spatial=SpatialRead(&fields);let mut services=Services{catalog,bank:&bank,fields:&fields,pc:parts.precision};
    let(arena,actors)=parts.actors.split_minds();
    let mut world=mind_world::World{actors,runtime:parts.mind_runtime,queues:parts.queues,owners:parts.owners,services:&mut services};
    decree::issue_selected_in_bank(catalog,&bank,&mut world,arena,&spatial,globals,&mut SelectionDiagnostics,slot,view,wire).map_err(|e|e.to_string())
}
///The decoder is shared with86/87's existing native decree binding. It must
///complete before any mutable gameplay World loan is acquired.
pub fn decoded(descriptor:usize,fields:&[aa_orders::payload::Field])->Result<Decoded,String>{decree::from_fields(descriptor,fields).map_err(|e|e.to_string())}
