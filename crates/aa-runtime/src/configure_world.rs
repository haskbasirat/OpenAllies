//! Exact after-KINGDOMS/pre-static ConfigureGWorld invocation ownership.
//! Diagnostic omission is separate from a native skipped branch. A reached
//!50E8B0 is never replaced with saved parameters or successful empty work.
use crate::{catalog::Catalog,map_resource_load,resource_creator::CreatorWitness};
use aa_actors::{ActorId,IdentityRegistry};
use aa_mapworld::{header::MapHeader,restore::{RestoredRecord,NativeMessage}};
use aa_numeric::Precision;
use aa_simulation::NativeRng;
type Result<T>=std::result::Result<T,String>;

///The actual pre-static class4 removal service. Its context borrows disjoint
///persistent World fields; registry/Kingdom/registration owners are passed at
///the reached native4F53D0 boundary, never reconstructed as an ActorStore.
pub(crate) trait KingdomRemoval {
    fn remove(&mut self,catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,
        kingdoms:&mut crate::load_kingdom_init_world::KingdomRegistryState,
        registration:&mut crate::live_registration::State,id:ActorId,pc:Precision)->Result<()>;
}
struct RequiredKingdomRemoval;
impl KingdomRemoval for RequiredKingdomRemoval {
    fn remove(&mut self,_:&Catalog<'_>,_:&mut IdentityRegistry<RestoredRecord>,
        _:&mut crate::load_kingdom_init_world::KingdomRegistryState,
        _:&mut crate::live_registration::State,id:ActorId,_:Precision)->Result<()> {
        Err(format!("50E8B0 requires actual4F53D0 destruction of queued Kingdom {} before final masks",id.raw()))
    }
}

///50ED15..50ED38 drains the private append-ordered pointer vector before
///constructing the final major-Kingdom iterator. Error stops keep prior effects.
fn drain_pending(ids:&[ActorId],mut remove:impl FnMut(ActorId)->Result<()>)->Result<()> {
    for &id in ids {remove(id)?;}
    Ok(())
}

#[path="configure_world_values.rs"]mod values;

///4E4130 removes the old Team membership before clearing Kingdom+1EC.
///Reuse the same first-remove/counter semantics as ConfigureGWorld assignment.
pub(crate) fn clear_team_for_kingdom_removal(
    registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId,
)->Result<()> {
    values::set_team(registry,id,None)
}
enum Phase {DiagnosticOmitted,Pending,NativeSkipped,Required,Completed}
pub(crate) struct State {phase:Phase,creator:Option<CreatorWitness>,messages:Vec<NativeMessage>}
fn required(load_mode:u32,header_mode:impl FnOnce()->Result<u32>)->Result<bool> {
    if load_mode!=1 {return Ok(false)}
    Ok(!crate::map_resource_mode::world_rule_mode(header_mode()?)?.saved_state)
}
impl State {
    ///`selected` describes the actual prepared factory argument, not a native
    ///ready bit or a default inference from map metadata.
    pub(crate) fn from_world_constructor(selected:bool)->Self {
        Self {phase:if selected {Phase::Pending}else{Phase::DiagnosticOmitted},creator:None,messages:Vec::new()}
    }
    pub(crate) fn after_kingdoms(&mut self,catalog:&Catalog<'_>,resource:&map_resource_load::Owner,
        mode:&Option<u32>,header:&MapHeader,registry:&mut IdentityRegistry<RestoredRecord>,
        kingdoms:&mut crate::load_kingdom_init_world::KingdomRegistryState,
        teams:&mut crate::live_registration::State,rng:&mut NativeRng,pc:Precision)->Option<Result<()>> {
        self.after_kingdoms_with_removal(catalog,resource,mode,header,registry,kingdoms,teams,rng,pc,&mut RequiredKingdomRemoval)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn after_kingdoms_with_removal(&mut self,catalog:&Catalog<'_>,resource:&map_resource_load::Owner,
        mode:&Option<u32>,header:&MapHeader,registry:&mut IdentityRegistry<RestoredRecord>,
        kingdoms:&mut crate::load_kingdom_init_world::KingdomRegistryState,
        teams:&mut crate::live_registration::State,rng:&mut NativeRng,pc:Precision,
        removal:&mut impl KingdomRemoval)->Option<Result<()>> {
        if matches!(self.phase,Phase::DiagnosticOmitted) {return None}
        Some(self.invoke(catalog,resource,mode,header,registry,kingdoms,teams,rng,pc,removal))
    }
    #[allow(clippy::too_many_arguments)]
    fn invoke(&mut self,catalog:&Catalog<'_>,resource:&map_resource_load::Owner,mode:&Option<u32>,_header:&MapHeader,
        registry:&mut IdentityRegistry<RestoredRecord>,kingdoms:&mut crate::load_kingdom_init_world::KingdomRegistryState,
        teams:&mut crate::live_registration::State,rng:&mut NativeRng,pc:Precision,
        removal:&mut impl KingdomRemoval)->Result<()> {
        if !matches!(self.phase,Phase::Pending) {return Err("ConfigureGWorld boundary repeated or previously failed".into())}
        //All binding validation is preflight, not native invalid-pointer/SEH parity.
        resource.validate_loading(mode,&catalog.actors.sources)?;
        let actual_mode=mode.ok_or("ConfigureGWorld actual+A8 word unbound")?;
        let bank=catalog.definition_bank.try_borrow().map_err(|_|"ConfigureGWorld source Bank borrowed")?;
        resource.validate_creator(catalog,&bank)?;
        drop(bank);
        if !required(actual_mode,||Ok(resource.target_header()?.loaded_mode()))? {
            self.phase=Phase::NativeSkipped;
            return Ok(());
        }
        self.creator=Some(resource.creator()?.witness());
        self.phase=Phase::Required;
        //The log loan ends before any source/game callback can reborrow it.
        {
            let mut log=catalog.load_configuration_sync.try_borrow_mut().map_err(|e|e.to_string())?;
            marker(log.as_mut().ok_or("50E8B0 initial77DC50 requires actual process SyncA scope")?)?;
        }
        values::run(catalog,resource.creator()?,registry,kingdoms,teams,rng,&mut self.messages,pc,removal)?;
        self.phase=Phase::Completed;
        Ok(())
    }
    pub(crate) fn require_boundary(&self)->Result<()> {
        match self.phase {
            Phase::NativeSkipped|Phase::Completed=>Ok(()),
            Phase::DiagnosticOmitted=>Err("actual ConfigureGWorld boundary was explicitly omitted by diagnostic record loading".into()),
            Phase::Pending=>Err("actual ConfigureGWorld boundary has not run after KINGDOMS".into()),
            Phase::Required=>Err("actual required ConfigureGWorld children have not completed".into()),
        }
    }
    pub(crate) fn messages(&self)->&[NativeMessage] {&self.messages}
}
fn marker(binding:&mut crate::live_clocked::SyncBinding<'_>)->Result<()> {
 fn log(owner:&mut aa_orders::net::sync::Synchronizer,gate:&mut dyn FnMut()->Result<bool>)->Result<()> {
  let open=if owner.enabled&&owner.suspend<1 {gate()?}else{false};
  owner.log_marker(open,"ConfigureGWorld");Ok(())
 }
 match binding {
  crate::live_clocked::SyncBinding::Direct(v)=>log(v.owner,v.gate),
  crate::live_clocked::SyncBinding::Shared{owner,gate}=>{
   let mut actual=owner.try_borrow_mut().map_err(|e|e.to_string())?;
   log(&mut actual,*gate)
  }
 }
}
///4F02B0 obtains the actual World RNG after logging its current seed toA thenS.
///It does not itself draw.474B40's zero/one eligible-Faction paths still call
///this getter but do not execute85D840. Each actual log loan retires separately.
pub(crate) fn log_random_seed(catalog:&Catalog<'_>,rng:&NativeRng)->Result<()> {
 fn record(binding:&mut crate::live_clocked::SyncBinding<'_>,seed:u32)->Result<()> {
  fn log(owner:&mut aa_orders::net::sync::Synchronizer,gate:&mut dyn FnMut()->Result<bool>,seed:u32)->Result<()> {
   let open=if owner.enabled&&owner.suspend<1{gate()?}else{false};owner.log_int(open,"(int)m_rng.GetSeed()",seed);Ok(())
  }
  match binding {
   crate::live_clocked::SyncBinding::Direct(v)=>log(v.owner,v.gate,seed),
   crate::live_clocked::SyncBinding::Shared{owner,gate}=>{let mut actual=owner.try_borrow_mut().map_err(|e|e.to_string())?;log(&mut actual,*gate,seed)}
  }
 }
 {let mut a=catalog.load_configuration_sync.try_borrow_mut().map_err(|e|e.to_string())?;
  record(a.as_mut().ok_or("4F02B0 actual SyncA required")?,rng.state)?;}
 {let mut s=catalog.load_configuration_sync_s.try_borrow_mut().map_err(|e|e.to_string())?;
  record(s.as_mut().ok_or("4F02B0 actual SyncS required after actual SyncA log")?,rng.state)?;}
 Ok(())
}
///The paired4F02B0 +85D840 path used by random Color/Nation selection.
pub(crate) fn random_word(catalog:&Catalog<'_>,rng:&mut NativeRng)->Result<u32> {
 log_random_seed(catalog,rng)?;Ok(rng.next_u32())
}
#[cfg(test)]
#[path="configure_world_tests.rs"]
mod tests;
