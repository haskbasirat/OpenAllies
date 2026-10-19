//!The orders input's view leaves that ask the current Kingdom: view
//!vt+78/7C/80/84 (457B70/457BB0/457BF0/457C30 over 5C0E50 and 4EAD70),
//!vt+50/54 (45BF20/45BD40), Thing vt+58 4EE3C0, vt+84 4EB8B0, vt+20
//!4CF120, vt+34 4D1F90 and the 437650/640B40/5E05D0/436600 links, as
//!kernels over [`Reads`]. Production reads bind the SAME ActorStore,
//!spatial owner, SVar manager, detection memory (Thing+58/+5C) and the
//!ported NativeVisibility (vt+70/+74) and relation (Kingdom vt+144) leaves;
//!the Session/UI words come from [`OrderContext`].
//!Proof: research/skirmish-20261019-view-order-world (original code under
//!Unicorn; only vt+70/+74, Kingdom vt+144, 40EAA0 and GetCurrentThreadId
//!are seams), replayed by the tests below with the seam call order.
use super::super::{ViewDirectory,ViewHandle};
use crate::live_scene::battle::native::{CurrentView,NativeVisibility};
use aa_actors::{ActorId,components::ComponentSlot};
use aa_mapworld::special_state::SpecialShell;
use aa_runtime::{actor_world::{ActorStore,RuntimeComponent,RuntimeObject,Source,SpecialSource},
    load_view_variables,spatial_world::SpatialWorld};
pub type Result<T>=std::result::Result<T,String>;

///Data vt+38 (ActorData 4F0500) / vt+40 (DynamicDoodadData 4F0500) and the
///category word +0C that 4BB3C0 compares (the registry index).
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct Data{pub actor_38:bool,pub doodad_40:bool,pub category_0c:u32}
///Data vt+38/vt+40 by native data kind: ThingData A20770 and ObjectData
///A13318 both 4C9FA0 (0); ActorData A05540 vt+38 4F0500; DynamicDoodadData
///A0F6E0 vt+40 4F0500. Other kinds' vtables are not proven here.
pub(crate) fn data_virtuals(native_kind:u32,category_0c:u32)->Result<Data>{
    let(actor_38,doodad_40)=match native_kind{0|1=>(false,false),2=>(true,false),21=>(false,true),
        k=>return Err(format!("Data vt+38/+40 of native data kind {k} is not proven"))};
    Ok(Data{actor_38,doodad_40,category_0c})
}
///One Thing+5C seen record (4EB9F0 layout): +14 remembered owner, +34 flags.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct SeenRecord<T>{pub owner_14:Option<T>,pub flags_34:u32}

///Reads of the original owners at the native positions. Each method is one
///field or ported leaf; the kernels below keep the native order of reads.
pub(crate) trait Reads{
    type T:Copy+Eq;
    type V:Copy;
    ///Session+E4; None is a NULL B36550.
    fn session_e4(&mut self)->Result<Option<u32>>;
    ///76AC60: [B36558]+8 local player is non-NULL.
    fn local_player_76ac60(&mut self)->Result<bool>;
    ///765E00 on the non-NULL local player: player+28 (GWorld is present).
    fn player_kingdom_765e00(&mut self)->Result<Option<Self::T>>;
    ///UI(B3653C)+1E0.
    fn ui_kingdom_1e0(&mut self)->Result<Option<Self::T>>;
    ///76AC90: Session+F8.
    fn session_phase_f8(&mut self)->Result<u32>;
    fn can_process_40eaa0(&mut self)->Result<bool>;
    ///4EEF8A: GetCurrentThreadId() == [B364D4]+48 (MainThread).
    fn current_thread_is_main_b364d4(&mut self)->Result<bool>;
    ///4F0930: [[B39AB0]+1BC] CheatFog byte, then GWorld+275.
    fn cheat_fog_1bc(&mut self)->Result<bool>;
    fn world_275(&mut self)->Result<bool>;
    ///45BC1B: the static 4B2E60 UVar PerformanceDisableDoodads byte.
    fn performance_disable_doodads(&mut self)->Result<bool>;
    fn class(&mut self,t:Self::T)->Result<u32>;
    fn flags_8(&mut self,t:Self::T)->Result<u32>;
    fn data(&mut self,t:Self::T)->Result<Data>;
    fn data_mask_174(&mut self,t:Self::T)->Result<u32>;
    ///ObjectData+16D (49C460).
    fn data_16d(&mut self,t:Self::T)->Result<u8>;
    ///Thing vt+70 (4EA1D0 / 4D3B80) and vt+74 (4EA0D0 / 4D3C50).
    fn visible_70(&mut self,t:Self::T,k:Option<Self::T>,deep:bool)->Result<bool>;
    fn explored_74(&mut self,t:Self::T,k:Option<Self::T>,deep:bool)->Result<bool>;
    ///Thing+58 seen-record count and +5C[index].
    fn memory_count_58(&mut self,t:Self::T)->Result<i32>;
    fn memory_slot_5c(&mut self,t:Self::T,index:u32)->Result<Option<SeenRecord<Self::T>>>;
    ///GActor+E8 (vt+108 4D1E80).
    fn owner_e8(&mut self,t:Self::T)->Result<Option<Self::T>>;
    fn kingdom_190(&mut self,k:Self::T)->Result<u32>;
    fn kingdom_18c(&mut self,k:Self::T)->Result<u32>;
    ///Kingdom vt+144 4E52D0.
    fn relation_144(&mut self,k:Self::T,other:Option<Self::T>)->Result<u32>;
    ///GActor+offset component slot is non-NULL.
    fn component(&mut self,t:Self::T,offset:u32)->Result<bool>;
    ///+80 Element -> +10 (4549C0); None when either is NULL.
    fn element_parent_10(&mut self,t:Self::T)->Result<Option<Self::T>>;
    fn action_bits_f0(&mut self,t:Self::T)->Result<u64>;
    ///+16C handle through B364C0: the live object, None for a stale
    ///generation or a NULL entry.
    fn link_16c(&mut self,t:Self::T)->Result<Option<Self::T>>;
    ///+94 Structure (None when NULL) and its +34 handle through B364C0.
    fn structure_34(&mut self,t:Self::T)->Result<Option<Option<Self::T>>>;
    ///+7C Organization (None when NULL) and its +40 leader.
    fn leader_40(&mut self,t:Self::T)->Result<Option<Option<Self::T>>>;
    ///Thing+C, the published view.
    fn view_c(&mut self,t:Self::T)->Result<Option<Self::V>>;
}

///The 4F9C10 runtime classes whose vtables the oracle copies (class5
///Team has a short vtable without these slots).
fn class<R:Reads>(r:&mut R,t:R::T)->Result<u32>{
    match r.class(t)?{c@(2|3|4|6|7|8|9|10)=>Ok(c),c=>Err(format!("Thing runtime class{c} has no proven 4F9C10 vtable for these leaves"))}
}
///vt+20/+34/+108 are the GActor 4CF120/4D1F90/4D1E80 for classes 3, 4, 8.
pub(crate) fn gactor(class:u32)->bool{matches!(class,3|4|8)}

///5C0E50 (ecx = UI): 40CEC0 (Session+E4 == 3) -> NULL; a NULL Session or a
///NULL 76AC70 -> UI+1E0; else 76AC70.
pub(crate) fn kingdom_5c0e50<R:Reads>(r:&mut R)->Result<Option<R::T>>{
    match r.session_e4()?{
        Some(3)=>Ok(None),
        None=>r.ui_kingdom_1e0(),
        Some(_)=>match kingdom_76ac70(r)?{Some(k)=>Ok(Some(k)),None=>r.ui_kingdom_1e0()},
    }
}
///76AC70 on the non-NULL Session.
fn kingdom_76ac70<R:Reads>(r:&mut R)->Result<Option<R::T>>{
    if r.session_e4()?==Some(3)||!r.local_player_76ac60()?{return Ok(None)}
    r.player_kingdom_765e00()
}
///5C0E10: NULL Session -> 0, else 76AC60.
fn local_player_5c0e10<R:Reads>(r:&mut R)->Result<bool>{
    if r.session_e4()?.is_none(){return Ok(false)}
    r.local_player_76ac60()
}
///4F0930 on the activated GWorld (40CFA0's B36534 test is satisfied by the
///activated session's own World).
fn fog_cheat_4f0930<R:Reads>(r:&mut R)->Result<bool>{Ok(r.cheat_fog_1bc()?||r.world_275()?)}
///4EEF70 (vt+5C of every supported class).
fn visible_4eef70<R:Reads>(r:&mut R,t:R::T,k:Option<R::T>,deep:bool)->Result<bool>{
    if r.flags_8(t)?&(1<<7)!=0{return Ok(false)}
    if r.current_thread_is_main_b364d4()?{
        let e4=r.session_e4()?.ok_or("4EEF9C reads Session+E4 of a NULL B36550 (native NULL dereference)")?;
        if e4==3||fog_cheat_4f0930(r)?{return Ok(true)}
    }
    r.visible_70(t,k,deep)
}
///4EA430: the k+18C seen record when k is non-NULL, k+190 == 0 and vt+8C
///4EA3D0 (Thing+58 > 0).
fn seen_record_4ea430<R:Reads>(r:&mut R,t:R::T,k:Option<R::T>)->Result<Option<SeenRecord<R::T>>>{
    let Some(k)=k else{return Ok(None)};
    if r.kingdom_190(k)?!=0||r.memory_count_58(t)?<=0{return Ok(None)}
    let index=r.kingdom_18c(k)?;
    r.memory_slot_5c(t,index)
}
///4EA350.
fn potentially_visible_4ea350<R:Reads>(r:&mut R,t:R::T,class:u32)->Result<bool>{
    let d=r.data(t)?;
    if d.doodad_40{return Ok(false)}
    if d.actor_38&&r.data_mask_174(t)?&(1<<26)!=0{return Ok(false)}
    let f=r.flags_8(t)?;
    if f&(1<<29)==0||f&(1<<11)==0||f&(1<<23)!=0||f&(1<<22)!=0{return Ok(false)}
    if r.data_16d(t)?!=0{return Ok(false)}
    //vt+A8: 6A71D0 (true) for class4 only.
    Ok(class!=4)
}
///4EAB20 (vt+60; 4CE890 jumps here for the GActor classes).
fn remembered_4eab20<R:Reads>(r:&mut R,t:R::T,k:Option<R::T>,a:bool)->Result<bool>{
    let class=class(r,t)?;
    if visible_4eef70(r,t,k,a)?{return Ok(false)}
    if !r.explored_74(t,k,a)?{return Ok(false)}
    if potentially_visible_4ea350(r,t,class)?{return Ok(true)}
    Ok(seen_record_4ea430(r,t,k)?.is_some())
}
///4EE3C0 (vt+58).
pub(crate) fn seen_4ee3c0<R:Reads>(r:&mut R,t:R::T,k:Option<R::T>,a:bool)->Result<bool>{
    class(r,t)?;
    Ok(visible_4eef70(r,t,k,a)?||remembered_4eab20(r,t,k,a)?)
}
enum Apparent<T>{Current,Remembered(SeenRecord<T>),Default}
///The common head of 4EAD70 and 4EB340.
fn apparent<R:Reads>(r:&mut R,t:R::T,k:Option<R::T>)->Result<Apparent<R::T>>{
    let class=class(r,t)?;
    let mut visible=visible_4eef70(r,t,k,true)?;
    let mut seen=remembered_4eab20(r,t,k,true)?;
    if visible&&r.flags_8(t)?&(1<<7)!=0{visible=false;seen=false;}
    let d=r.data(t)?;
    let flare=d.actor_38&&r.data_mask_174(t)?&(1<<26)!=0;
    if visible||(seen&&potentially_visible_4ea350(r,t,class)?)||flare{return Ok(Apparent::Current)}
    if seen{
        return seen_record_4ea430(r,t,k)?.map(Apparent::Remembered)
            .ok_or_else(||"4EA430 returned a NULL seen record that 4EADF9/4EB3C9 dereferences (native NULL dereference)".into());
    }
    Ok(Apparent::Default)
}
///4EAD70: vt+108 (GActor+E8, else 4C9FA0 NULL), the record's +14, else
///[B3CC00] (zero-filled, never stored).
pub(crate) fn apparent_owner_4ead70<R:Reads>(r:&mut R,t:R::T,k:Option<R::T>)->Result<Option<R::T>>{
    Ok(match apparent(r,t,k)?{
        Apparent::Current=>if gactor(class(r,t)?){r.owner_e8(t)?}else{None},
        Apparent::Remembered(s)=>s.owner_14,
        Apparent::Default=>None,
    })
}
///4EB340: Thing+8, the record's +34, else [B3CC24] (zero-filled, never stored).
fn apparent_flags_4eb340<R:Reads>(r:&mut R,t:R::T,k:Option<R::T>)->Result<u32>{
    Ok(match apparent(r,t,k)?{Apparent::Current=>r.flags_8(t)?,Apparent::Remembered(s)=>s.flags_34,Apparent::Default=>0})
}
///The four view class slots.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum ViewClass{
    ///vt+78 457B70 -> Kingdom vt+134 4E5240 (k == owner).
    Own78,
    ///vt+7C 457BB0 -> vt+138 4E5250 (owner == k or relation 1).
    Ally7c,
    ///vt+80 457BF0 -> vt+13C 4E5280 (relation 2); NULL k or Thing -> true.
    Neutral80,
    ///vt+84 457C30 -> vt+140 4E52A0 (relation 3).
    Enemy84,
}
pub(crate) fn view_class<R:Reads>(r:&mut R,thing:Option<R::T>,which:ViewClass)->Result<bool>{
    let k=kingdom_5c0e50(r)?;
    let(Some(k),Some(t))=(k,thing) else{return Ok(which==ViewClass::Neutral80)};
    let owner=apparent_owner_4ead70(r,t,Some(k))?;
    Ok(match which{
        ViewClass::Own78=>owner==Some(k),
        ViewClass::Ally7c=>owner==Some(k)||r.relation_144(k,owner)?==1,
        ViewClass::Neutral80=>r.relation_144(k,owner)?==2,
        ViewClass::Enemy84=>r.relation_144(k,owner)?==3,
    })
}
///45BBD0 (esi = view+14, edi = view+10).
fn doodads_45bbd0<R:Reads>(r:&mut R,thing:Option<R::T>,data:Data)->Result<bool>{
    if !r.performance_disable_doodads()?{return Ok(false)}
    if let Some(t)=thing{if r.flags_8(t)?&(1<<10)!=0{return Ok(false)}}
    Ok(data.category_0c==1||data.category_0c==0x26||data.doodad_40)
}
///View vt+50 45BF20(a).
pub(crate) fn visible_45bf20<R:Reads>(r:&mut R,thing:Option<R::T>,data:Data,flags_54:u32,a:bool)->Result<bool>{
    if doodads_45bbd0(r,thing,data)?{return Ok(false)}
    if fog_cheat_4f0930(r)?{return Ok(true)}
    if !local_player_5c0e10(r)?{return Ok(false)}
    if flags_54&(1<<26)!=0||flags_54&(1<<20)!=0{return Ok(true)}
    let Some(k)=kingdom_5c0e50(r)? else{return Ok(true)};
    match thing{
        Some(t)=>visible_4eef70(r,t,Some(k),a),
        None=>Err("45BFA5 Thing-less view visibility (4282A0/42A530 position path) is not ported".into()),
    }
}
///View vt+54 45BD40(a).
pub(crate) fn remembered_45bd40<R:Reads>(r:&mut R,thing:Option<R::T>,data:Data,flags_54:u32,a:bool)->Result<bool>{
    if !local_player_5c0e10(r)?||doodads_45bbd0(r,thing,data)?{return Ok(false)}
    let k=kingdom_5c0e50(r)?;
    let(Some(t),Some(k))=(thing,k) else{return Ok(false)};
    if flags_54&(1<<20)!=0{return Ok(false)}
    remembered_4eab20(r,t,Some(k),a)
}
///Thing vt+84 4EB8B0.
pub(crate) fn targetable_4eb8b0<R:Reads>(r:&mut R,t:R::T,k:Option<R::T>)->Result<bool>{
    class(r,t)?;
    if let Some(k)=k{
        if r.kingdom_190(k)?==0&&r.memory_count_58(t)?>0{
            let index=r.kingdom_18c(k)?;
            if r.memory_slot_5c(t,index)?.is_some(){return Ok(apparent_flags_4eb340(r,t,Some(k))?&(1<<14)!=0)}
        }
    }
    Ok(r.flags_8(t)?&(1<<14)!=0)
}
///Thing vt+20: 4CF120 for GActor classes, 969B10 (false) otherwise. The
///Element parent is a tail call of its own vt+20.
pub(crate) fn movable_20<R:Reads>(r:&mut R,mut t:R::T,a:bool)->Result<bool>{
    for _ in 0..0x10000{
        if !gactor(class(r,t)?)||r.flags_8(t)?&4!=0{return Ok(false)}
        if let Some(parent)=r.element_parent_10(t)?{t=parent;continue}
        if a&&r.can_process_40eaa0()?&&r.session_phase_f8()?==7{return Ok(true)}
        if !r.component(t,0x6c)?{return Ok(true)}
        return Ok(r.component(t,0xac)?||r.component(t,0xa0)?||r.component(t,0x88)?);
    }
    Err("4CF143 Element parent vt+20 chain does not terminate".into())
}
///Thing vt+34: 4D1F90 (96D710 shifts 1 by the low byte; >= 64 is 0).
pub(crate) fn can_34<R:Reads>(r:&mut R,t:R::T,index:u32)->Result<bool>{
    if !gactor(class(r,t)?){return Ok(false)}
    let shift=index&0xff;
    Ok(shift<64&&r.action_bits_f0(t)?&(1u64<<shift)!=0)
}
///437650 after 45B7A0: the +16C object's non-NULL +C view.
pub(crate) fn link_16c_view<R:Reads>(r:&mut R,t:R::T)->Result<Option<R::V>>{
    match r.link_16c(t)?{Some(o)=>r.view_c(o),None=>Ok(None)}
}
///640B40: the Structure+34 object's non-NULL +C view.
pub(crate) fn link_94_view<R:Reads>(r:&mut R,t:R::T)->Result<Option<R::V>>{
    match r.structure_34(t)?{Some(Some(o))=>r.view_c(o),_=>Ok(None)}
}
///5E05D0's test: Structure+34 resolves to a non-NULL B364C0 entry.
pub(crate) fn link_94_live<R:Reads>(r:&mut R,t:R::T)->Result<bool>{Ok(matches!(r.structure_34(t)?,Some(Some(_))))}
///640B40: s = 4549C0 non-NULL and s vt+58(5C0E50, 1) false.
pub(crate) fn hidden_4549c0<R:Reads>(r:&mut R,t:R::T)->Result<bool>{
    let Some(s)=r.element_parent_10(t)? else{return Ok(false)};
    let k=kingdom_5c0e50(r)?;
    Ok(!seen_4ee3c0(r,s,k,true)?)
}
///436600 for one selected 45B7A0 actor: Organization+40 leader with an
///Experience (+A4) answers its +C view.
pub(crate) fn voiced_436600<R:Reads>(r:&mut R,t:R::T)->Result<Option<Option<R::V>>>{
    let Some(leader)=r.leader_40(t)? else{return Ok(None)};
    let leader=leader.ok_or("43662B reads +A4 of a NULL Organization+40 leader (native NULL dereference)")?;
    if !r.component(leader,0xa4)?{return Ok(None)}
    Ok(Some(r.view_c(leader)?))
}

///Session/UI words of one orders frame. The AdminNode loan holds the
///activated Session mutably while the frame runs, so these are read from
///their actual owners immediately before it; each failure stays an error
///that is returned only where the native code reads that word.
pub struct OrderContext{
    session_e4:u32,
    local_player:bool,
    kingdom_76ac70:Result<Option<ActorId>>,
    phase_f8:u32,
    can_process:Result<bool>,
    ui_kingdom_1e0:Result<Option<ActorId>>,
    performance_disable_doodads:Result<Option<u8>>,
    swap_right_click_4be150:Result<bool>,
    hotkey_alt_right_click_4b2e60:Result<Vec<u16>>,
}
impl OrderContext{
    ///`kingdom_76ac70` is ActualActivatedSession::local_player_kingdom_76ac70,
    ///`ui_kingdom_1e0` the 5C3000-written UI field, `performance_disable_doodads`
    ///the UVar manager (B39938) raw byte (None: no record).
    #[allow(clippy::too_many_arguments)]
    pub fn new(session_e4:u32,local_player:bool,kingdom_76ac70:Result<Option<ActorId>>,phase_f8:u32,
        can_process:Result<bool>,ui_kingdom_1e0:Result<Option<ActorId>>,performance_disable_doodads:Result<Option<u8>>,
        swap_right_click_4be150:Result<bool>,hotkey_alt_right_click_4b2e60:Result<Vec<u16>>)->Self{
        Self{session_e4,local_player,kingdom_76ac70,phase_f8,can_process,ui_kingdom_1e0,performance_disable_doodads,
            swap_right_click_4be150,hotkey_alt_right_click_4b2e60}
    }
    ///76AC70 as 436950 reads it through 5C0E30 before each vt+150.
    pub(crate) fn kingdom_76ac70(&self)->Result<Option<ActorId>>{self.kingdom_76ac70.clone()}
    ///4CF430's globals: 40EAA0, 76AC90 and the two right-click UVars.
    pub(crate) fn order_globals(&self)->crate::battle_orders_input::UiOrderGlobals{
        crate::battle_orders_input::UiOrderGlobals{can_process_40eaa0:self.can_process.clone(),
            session_phase_76ac90:Ok(self.phase_f8),
            swap_right_click_with_alt_right_click_4be150:self.swap_right_click_4be150.clone(),
            hotkey_alt_right_click_command_4b2e60:self.hotkey_alt_right_click_4b2e60.clone()}
    }
}

///Production reads over the SAME owners the directory was bound to.
pub(crate) struct Live<'a>{
    pub directory:&'a ViewDirectory,
    pub actors:&'a ActorStore,
    pub spatial:&'a SpatialWorld,
    pub variables:&'a load_view_variables::State,
    ///GWorld+275 (4F2BE4 resets it; the World's runtime field owner).
    pub runtime_fields:&'a aa_runtime::load_world_runtime_fields::State,
    ///ObjectData+16D (49C460) of the World's source requirements.
    pub source_fields:&'a aa_rules::native_source_requirements::NativeSourceRequirements,
    pub context:Option<&'a OrderContext>,
}
const NO_CONTEXT:&str="orders Session/UI context was not supplied (bind DirectoryViews with an OrderContext)";
fn text(e:impl std::fmt::Display)->String{e.to_string()}
impl Live<'_>{
    fn context(&self)->Result<&OrderContext>{self.context.ok_or_else(||NO_CONTEXT.to_string())}
    fn object(&self,t:ActorId)->Result<&RuntimeObject>{
        let key=self.actors.key(t).ok_or_else(||format!("Thing GTID {:#x} is not live",t.raw()))?;
        self.actors.object(key).map_err(text)
    }
    fn actor_words(&self,t:ActorId)->Result<&std::collections::BTreeMap<u32,u32>>{
        Ok(&self.object(t)?.actor.as_ref().ok_or("GActor fields are absent")?.words)
    }
    fn memory(&self,t:ActorId)->Result<&aa_actors::detection_memory::State>{
        Ok(&self.object(t)?.object.as_ref().ok_or("GObject fields (+58/+5C detection memory) are absent")?.memory)
    }
    fn get(&self,t:ActorId,slot:ComponentSlot)->Result<Option<&RuntimeComponent>>{
        if !gactor(self.object(t)?.definition.native_class){return Err("Thing has no GActor component table".into())}
        self.actors.component(t,slot).map_err(text)
    }
    ///A B364C0 handle: a matching generation with a live entry.
    fn resolve(&self,raw:u32)->Option<ActorId>{
        let id=ActorId::from_raw(raw);
        (!id.is_null()&&self.actors.key(id).is_some()).then_some(id)
    }
    fn special(&self,id:ActorId,class:u32)->Result<&SpecialShell>{
        let key=self.actors.key(id).ok_or("Kingdom/Team GTID is not live")?;
        if self.actors.object(key).map_err(text)?.definition.native_class!=class{return Err(format!("relation owner is not class{class}"))}
        let Source::Special(saved)=&self.actors.entity(key).map_err(text)?.source else{return Err("Kingdom/Team derived owner is absent".into())};
        if !matches!((&saved.source,class),(SpecialSource::Kingdom(_),4)|(SpecialSource::Team(_),5)){return Err("Kingdom/Team derived class differs".into())}
        saved.derived.as_ref().ok_or_else(||"Kingdom/Team derived fields are absent".into())
    }
    fn word(shell:&SpecialShell,offset:u32)->Result<u32>{shell.words.get(&offset).copied().ok_or_else(||format!("Kingdom/Team +{offset:X} is unwritten"))}
}
///vt+70/+74 read only the FogOfWar SVar through CurrentView.
struct Fog<'a>(&'a load_view_variables::State);
impl CurrentView for Fog<'_>{
    fn kingdom(&self)->anyhow::Result<Option<ActorId>>{anyhow::bail!("vt+70/+74 take the caller's Kingdom")}
    fn fog_enabled(&self)->anyhow::Result<bool>{Ok(self.0.fog_enabled())}
    fn current_thread_is_simulation(&self)->anyhow::Result<bool>{anyhow::bail!("4EEF70's gate is read by the caller")}
    fn application_state(&self)->anyhow::Result<u32>{anyhow::bail!("4EEF70's gate is read by the caller")}
    fn editor_option_enabled(&self)->anyhow::Result<bool>{anyhow::bail!("4EEF70's gate is read by the caller")}
    fn world_editor_override(&self)->anyhow::Result<bool>{anyhow::bail!("4EEF70's gate is read by the caller")}
}
///The ported 4CF040/4E52D0/4E10F0 relation reads over the Kingdom/Team
///derived shells. B28158 = 2 and B2815C = 3 are their .data initial values;
///every reference is a load (the oracle's static section).
struct Relations<'a,'b>(&'a Live<'b>);
impl aa_runtime::relations::borrowed::Read for Relations<'_,'_>{
    type Id=ActorId;
    type Error=String;
    fn kingdom_team(&self,id:ActorId)->Result<Option<ActorId>>{
        let shell=self.0.special(id,4)?;
        match shell.identity_references.get(&0x1ec){
            Some(team)=>Ok(*team),
            None if shell.words.get(&0x1ec)==Some(&0)=>Ok(None),
            None=>Err("4E52EB Kingdom+1EC Team reference is unbound".into()),
        }
    }
    fn kingdom_slot(&self,id:ActorId)->Result<u32>{Live::word(self.0.special(id,4)?,0x188)}
    fn team_slot(&self,id:ActorId)->Result<u32>{Live::word(self.0.special(id,5)?,0x38)}
    fn kingdom_word(&self,id:ActorId,team:bool,slot:u32)->Result<u32>{vector(self.0.special(id,4)?,if team{0x1dc}else{0x1d0},slot)}
    fn team_word(&self,id:ActorId,team:bool,slot:u32)->Result<u32>{vector(self.0.special(id,5)?,if team{0x58}else{0x4c},slot)}
    fn orphan_relation(&self)->Result<u32>{Ok(2)}
    fn team_null_relation(&self)->Result<u32>{Ok(3)}
}
fn vector(shell:&SpecialShell,offset:u32,slot:u32)->Result<u32>{
    shell.vectors.get(&offset).and_then(|v|v.values.get(slot as usize)).copied()
        .ok_or_else(||format!("relation +{offset:X} vector has no slot {slot}"))
}
impl Reads for Live<'_>{
    type T=ActorId;
    type V=ViewHandle;
    fn session_e4(&mut self)->Result<Option<u32>>{Ok(Some(self.context()?.session_e4))}
    fn local_player_76ac60(&mut self)->Result<bool>{Ok(self.context()?.local_player)}
    fn player_kingdom_765e00(&mut self)->Result<Option<ActorId>>{self.context()?.kingdom_76ac70.clone()}
    fn ui_kingdom_1e0(&mut self)->Result<Option<ActorId>>{self.context()?.ui_kingdom_1e0.clone()}
    fn session_phase_f8(&mut self)->Result<u32>{Ok(self.context()?.phase_f8)}
    fn can_process_40eaa0(&mut self)->Result<bool>{self.context()?.can_process.clone()}
    ///The orders input runs in the UI frame 5C92D0, which App 410E70 runs
    ///from MainThread vt+3C (A84530 -> 6FD290), the object at B364D4 whose
    ///+48 85FAA9 captured on that same thread: the comparison is equal.
    fn current_thread_is_main_b364d4(&mut self)->Result<bool>{self.context()?;Ok(true)}
    fn cheat_fog_1bc(&mut self)->Result<bool>{Ok(self.variables.cheat_fog_enabled())}
    fn world_275(&mut self)->Result<bool>{Ok(*self.runtime_fields.fog_cheat_word()!=0)}
    fn performance_disable_doodads(&mut self)->Result<bool>{
        self.context()?.performance_disable_doodads.clone()?.map(|b|b!=0)
            .ok_or_else(||"4B2E60 found no UVar PerformanceDisableDoodads record; 45BC20 dereferences its NULL result".into())
    }
    fn class(&mut self,t:ActorId)->Result<u32>{Ok(self.object(t)?.definition.native_class)}
    fn flags_8(&mut self,t:ActorId)->Result<u32>{
        let key=self.actors.key(t).ok_or("Thing GTID is not live")?;
        Ok(self.actors.placement(key,self.spatial).map_err(text)?.flags)
    }
    fn data(&mut self,t:ActorId)->Result<Data>{
        let category=self.object(t)?.definition.proof.category;
        data_virtuals(category.native_kind,category.registry_index)
    }
    fn data_mask_174(&mut self,t:ActorId)->Result<u32>{
        self.object(t)?.definition.component_mask.ok_or_else(||"ActorData+174 component mask is absent".into())
    }
    fn data_16d(&mut self,t:ActorId)->Result<u8>{
        let id=&self.object(t)?.definition.proof.id;
        self.source_fields.byte(id,0x16d).map_err(text)
    }
    fn visible_70(&mut self,t:ActorId,k:Option<ActorId>,deep:bool)->Result<bool>{
        NativeVisibility::new(&Fog(self.variables)).visible_to(self.actors,t,k,deep).map_err(|e|format!("{e:#}"))
    }
    fn explored_74(&mut self,t:ActorId,k:Option<ActorId>,deep:bool)->Result<bool>{
        NativeVisibility::new(&Fog(self.variables)).explored_by(self.actors,t,k,deep).map_err(|e|format!("{e:#}"))
    }
    fn memory_count_58(&mut self,t:ActorId)->Result<i32>{Ok(self.memory(t)?.count())}
    fn memory_slot_5c(&mut self,t:ActorId,index:u32)->Result<Option<SeenRecord<ActorId>>>{
        let memory=self.memory(t)?;
        let Some(id)=memory.slot(index).map_err(text)? else{return Ok(None)};
        let record=memory.record(id).map_err(text)?;
        Ok(Some(SeenRecord{owner_14:(!record.owner.is_null()).then_some(record.owner),flags_34:record.flags}))
    }
    fn owner_e8(&mut self,t:ActorId)->Result<Option<ActorId>>{self.actors.ordinary_owner(t).map_err(text)}
    fn kingdom_190(&mut self,k:ActorId)->Result<u32>{Live::word(self.special(k,4)?,0x190)}
    fn kingdom_18c(&mut self,k:ActorId)->Result<u32>{Live::word(self.special(k,4)?,0x18c)}
    fn relation_144(&mut self,k:ActorId,other:Option<ActorId>)->Result<u32>{
        aa_runtime::relations::borrowed::kingdom(&Relations(self),k,other)
    }
    fn component(&mut self,t:ActorId,offset:u32)->Result<bool>{
        let slot=offset.checked_sub(0x60).filter(|o|o%4==0).and_then(|o|ComponentSlot::from_index(o as usize/4))
            .ok_or_else(||format!("GActor+{offset:X} is not a component slot"))?;
        Ok(self.get(t,slot)?.is_some())
    }
    fn element_parent_10(&mut self,t:ActorId)->Result<Option<ActorId>>{
        match self.get(t,ComponentSlot::Element)?{
            None=>Ok(None),
            Some(RuntimeComponent::Element(e))=>Ok(e.organization),
            Some(_)=>Err("GActor+80 Element slot holds another component".into()),
        }
    }
    fn action_bits_f0(&mut self,t:ActorId)->Result<u64>{Ok(self.actors.action_fields(t,self.spatial).map_err(text)?.action_bits)}
    fn link_16c(&mut self,t:ActorId)->Result<Option<ActorId>>{
        let raw=*self.actor_words(t)?.get(&0x16c).ok_or("GActor+16C is unwritten")?;
        Ok(self.resolve(raw))
    }
    fn structure_34(&mut self,t:ActorId)->Result<Option<Option<ActorId>>>{
        match self.get(t,ComponentSlot::Structure)?{
            None=>Ok(None),
            Some(RuntimeComponent::Structure(s))=>Ok(Some(self.resolve(s.word_34))),
            Some(_)=>Err("GActor+94 Structure slot holds another component".into()),
        }
    }
    fn leader_40(&mut self,t:ActorId)->Result<Option<Option<ActorId>>>{
        match self.get(t,ComponentSlot::Organization)?{
            None=>Ok(None),
            Some(RuntimeComponent::Organization(o))=>{
                let raw=*o.words.get(&0x40).ok_or("Organization+40 is unwritten")?;
                if raw==0{return Ok(Some(None))}
                let leader=self.resolve(raw).ok_or("Organization+40 leader is not a live Thing (436622 dereferences it)")?;
                Ok(Some(Some(leader)))
            }
            Some(_)=>Err("GActor+7C Organization slot holds another component".into()),
        }
    }
    fn view_c(&mut self,t:ActorId)->Result<Option<ViewHandle>>{
        let word=self.object(t)?.thing.word_c;
        if word==0{return Ok(None)}
        let view=ViewHandle(word);
        if !self.directory.entries.contains_key(&view){return Err("Thing+C names no view allocation".into())}
        Ok(Some(view))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::{Value,json};
    use std::collections::BTreeMap;
    ///The fixture's case as the same reads; seams append the oracle's trace.
    struct Case<'c>{c:&'c Value,names:Vec<String>,trace:Vec<Value>}
    impl<'c> Case<'c>{
        fn new(c:&'c Value)->Self{
            let mut names:Vec<String>=Vec::new();
            for group in ["kingdoms","things"]{
                if let Some(o)=c[group].as_object(){names.extend(o.keys().cloned())}
            }
            Self{c,names,trace:Vec::new()}
        }
        fn id(&self,name:&Value)->Option<u32>{name.as_str().map(|n|self.names.iter().position(|x|x==n).expect("named Thing") as u32)}
        fn name(&self,t:Option<u32>)->Value{t.map_or(Value::Null,|t|json!(self.names[t as usize]))}
        fn key(&self,t:Option<u32>)->String{t.map_or("None".into(),|t|self.names[t as usize].clone())}
        fn thing(&self,t:u32)->&'c Value{&self.c["things"][&self.names[t as usize]]}
        fn kingdom(&self,k:u32)->&'c Value{&self.c["kingdoms"][&self.names[k as usize]]}
        fn handle(&self,spec:&Value)->Option<u32>{
            if spec.is_null()||spec[3]==true{return None}
            self.id(&spec[2])
        }
        fn kind(&self,t:u32)->&'c str{
            let class=self.thing(t)["class"].as_u64().unwrap();
            self.thing(t)["data"].as_str().unwrap_or(if gactor(class as u32){"actor"}else{"object"})
        }
    }
    fn n(v:&Value,default:u64)->u64{v.as_u64().unwrap_or(default)}
    impl Reads for Case<'_>{
        type T=u32;
        ///The published view, named by its Thing.
        type V=u32;
        fn session_e4(&mut self)->Result<Option<u32>>{Ok(Some(n(&self.c["session"]["state"],0) as u32))}
        fn local_player_76ac60(&mut self)->Result<bool>{Ok(self.c["session"]["local_player"]==true)}
        fn player_kingdom_765e00(&mut self)->Result<Option<u32>>{Ok(self.id(&self.c["session"]["kingdom"]))}
        fn ui_kingdom_1e0(&mut self)->Result<Option<u32>>{Ok(self.id(&self.c["ui_kingdom"]))}
        fn session_phase_f8(&mut self)->Result<u32>{Ok(n(&self.c["session"]["phase"],0) as u32)}
        fn can_process_40eaa0(&mut self)->Result<bool>{self.trace.push(json!({"call":"can_process_40eaa0"}));Ok(self.c["can_process"]==true)}
        fn current_thread_is_main_b364d4(&mut self)->Result<bool>{
            self.trace.push(json!({"call":"current_thread"}));Ok(self.c["thread"].as_str().unwrap_or("main")=="main")
        }
        fn cheat_fog_1bc(&mut self)->Result<bool>{Ok(n(&self.c["cheat_fog"],0)!=0)}
        fn world_275(&mut self)->Result<bool>{Ok(n(&self.c["world_275"],0)!=0)}
        fn performance_disable_doodads(&mut self)->Result<bool>{Ok(n(&self.c["perf_doodads"],0)!=0)}
        fn class(&mut self,t:u32)->Result<u32>{Ok(n(&self.thing(t)["class"],0) as u32)}
        fn flags_8(&mut self,t:u32)->Result<u32>{Ok(n(&self.thing(t)["flags"],0) as u32)}
        fn data(&mut self,t:u32)->Result<Data>{
            let kind=match self.kind(t){"thing"=>0,"object"=>1,"actor"=>2,"doodad"=>21,k=>panic!("{k}")};
            let category=n(&self.thing(t)["category"],if kind==2{2}else{7}) as u32;
            data_virtuals(kind,category)
        }
        fn data_mask_174(&mut self,t:u32)->Result<u32>{Ok(n(&self.thing(t)["mask"],0) as u32)}
        fn data_16d(&mut self,t:u32)->Result<u8>{Ok(n(&self.thing(t)["byte_16d"],0) as u8)}
        fn visible_70(&mut self,t:u32,k:Option<u32>,deep:bool)->Result<bool>{
            self.trace.push(json!({"call":"visible_70","thing":self.name(Some(t)),"kingdom":self.name(k),"deep":deep as u8}));
            Ok(self.thing(t)["visible_70"][self.key(k)]==true)
        }
        fn explored_74(&mut self,t:u32,k:Option<u32>,deep:bool)->Result<bool>{
            self.trace.push(json!({"call":"explored_74","thing":self.name(Some(t)),"kingdom":self.name(k),"deep":deep as u8}));
            Ok(self.thing(t)["explored_74"][self.key(k)]==true)
        }
        fn memory_count_58(&mut self,t:u32)->Result<i32>{
            let seen=self.thing(t)["seen"].as_object().is_some_and(|s|!s.is_empty());
            Ok(n(&self.thing(t)["seen_count"],if seen{4}else{0}) as i32)
        }
        fn memory_slot_5c(&mut self,t:u32,index:u32)->Result<Option<SeenRecord<u32>>>{
            let Some(seen)=self.thing(t)["seen"].as_object() else{return Ok(None)};
            for(kingdom,record) in seen{
                if n(&self.c["kingdoms"][kingdom]["w18c"],0) as u32==index{
                    return Ok(Some(SeenRecord{owner_14:self.id(&record["owner"]),flags_34:n(&record["flags"],0) as u32}));
                }
            }
            Ok(None)
        }
        fn owner_e8(&mut self,t:u32)->Result<Option<u32>>{Ok(self.id(&self.thing(t)["owner"]))}
        fn kingdom_190(&mut self,k:u32)->Result<u32>{Ok(n(&self.kingdom(k)["w190"],0) as u32)}
        fn kingdom_18c(&mut self,k:u32)->Result<u32>{Ok(n(&self.kingdom(k)["w18c"],0) as u32)}
        fn relation_144(&mut self,k:u32,other:Option<u32>)->Result<u32>{
            self.trace.push(json!({"call":"relation_144","kingdom":self.name(Some(k)),"other":self.name(other)}));
            Ok(n(&self.kingdom(k)["relations"][self.key(other)],0) as u32)
        }
        fn component(&mut self,t:u32,offset:u32)->Result<bool>{
            let listed=self.thing(t)["components"].as_array().is_some_and(|a|a.iter().any(|v|v==offset));
            Ok(listed||(offset==0x7c&&!self.thing(t)["leader"].is_null())||(offset==0x94&&!self.thing(t)["structure_34"].is_null()))
        }
        fn element_parent_10(&mut self,t:u32)->Result<Option<u32>>{Ok(self.id(&self.thing(t)["element"]))}
        fn action_bits_f0(&mut self,t:u32)->Result<u64>{Ok(n(&self.thing(t)["action_bits"],0))}
        fn link_16c(&mut self,t:u32)->Result<Option<u32>>{Ok(self.handle(&self.thing(t)["link_16c"]))}
        fn structure_34(&mut self,t:u32)->Result<Option<Option<u32>>>{
            let thing=self.thing(t);
            Ok(thing.as_object().unwrap().contains_key("structure_34").then(||self.handle(&thing["structure_34"])))
        }
        fn leader_40(&mut self,t:u32)->Result<Option<Option<u32>>>{
            let leader=&self.thing(t)["leader"];
            Ok((!leader.is_null()).then(||self.id(leader)))
        }
        fn view_c(&mut self,t:u32)->Result<Option<u32>>{
            let name=&self.names[t as usize];
            Ok(self.c["views"].as_array().is_some_and(|v|v.iter().any(|x|x==name)).then_some(t))
        }
    }
    fn view(r:&mut Case<'_>)->(Option<u32>,Data,u32){
        let thing=r.id(&r.c["thing"]);
        let data=thing.map(|t|r.data(t).unwrap()).unwrap_or(Data{actor_38:false,doodad_40:false,category_0c:0});
        (thing,data,n(&r.c["view_flags"],0) as u32)
    }
    ///45B7A0 over the oracle's vtables (vt+B4).
    fn b4(r:&mut Case<'_>,t:u32)->bool{gactor(r.class(t).unwrap())}
    ///ui_world.rs select_owner_640b40 composed from these leaves.
    fn select_640b40(r:&mut Case<'_>,t:u32)->Result<Option<u32>>{
        let o=if b4(r,t){link_16c_view(r,t)?.unwrap_or(t)}else{return Ok(Some(t))};
        if !b4(r,o){return Ok(Some(o))}
        if let Some(container)=link_94_view(r,o)?{return Ok(Some(container))}
        if hidden_4549c0(r,o)?{return r.view_c(o)}
        let organization=r.component(o,0x7c)?;
        let parent=r.element_parent_10(o)?;
        let root=if organization{o}else{parent.unwrap_or(o)};
        r.view_c(root)
    }
    #[test]fn native_view_order_world_replay(){
        let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261019-view-order-world/native-view-order-world.json")).unwrap();
        let mut counts=BTreeMap::new();
        for row in fixture["rows"].as_array().unwrap(){
            let(c,o)=(&row["input"],&row["output"]);
            let leaf=c["leaf"].as_str().unwrap();
            let mut r=Case::new(c);
            let value=match leaf{
                "class_78"|"class_7c"|"class_80"|"class_84"=>{
                    let which=match leaf{"class_78"=>ViewClass::Own78,"class_7c"=>ViewClass::Ally7c,"class_80"=>ViewClass::Neutral80,_=>ViewClass::Enemy84};
                    let thing=r.id(&c["thing"]);
                    json!(view_class(&mut r,thing,which).unwrap() as u8)
                }
                "virtual_50"=>{let(t,d,f)=view(&mut r);json!(visible_45bf20(&mut r,t,d,f,true).unwrap() as u8)}
                "virtual_54"=>{let(t,d,f)=view(&mut r);json!(remembered_45bd40(&mut r,t,d,f,true).unwrap() as u8)}
                "vt58"|"vt84"|"vt20"|"vt34"=>{
                    let t=r.id(&c["thing"]).unwrap();
                    let k=r.id(&c["kingdom"]);
                    let class=r.class(t).unwrap();
                    let entry=o["entry"].as_str().unwrap();
                    match leaf{
                        "vt58"=>{assert_eq!(entry,"0x4ee3c0");json!(seen_4ee3c0(&mut r,t,k,true).unwrap() as u8)}
                        "vt84"=>{assert_eq!(entry,"0x4eb8b0");json!(targetable_4eb8b0(&mut r,t,k).unwrap() as u8)}
                        "vt20"=>{
                            assert_eq!(entry,if gactor(class){"0x4cf120"}else{"0x969b10"});
                            json!(movable_20(&mut r,t,n(&c["arg"],1)!=0).unwrap() as u8)
                        }
                        _=>{
                            assert_eq!(entry,if gactor(class){"0x4d1f90"}else{"0x969b10"});
                            json!(can_34(&mut r,t,n(&c["index"],0) as u32).unwrap() as u8)
                        }
                    }
                }
                "437650"=>{
                    let t=r.id(&c["thing"]).unwrap();
                    let result=if b4(&mut r,t){link_16c_view(&mut r,t).unwrap().unwrap_or(t)}else{t};
                    assert_eq!(r.name(Some(result)),o["result"],"{c}");rows(&mut counts,leaf);continue;
                }
                "640b40"=>{
                    let t=r.id(&c["thing"]).unwrap();
                    let result=select_640b40(&mut r,t).unwrap();
                    assert_eq!(r.name(result),o["result"],"{c}");
                    assert_eq!(json!(r.trace),o["trace"],"{c}");rows(&mut counts,leaf);continue;
                }
                "5e05d0"=>{
                    let t=r.id(&c["thing"]).unwrap();
                    assert!(b4(&mut r,t));
                    assert_eq!(json!(link_94_live(&mut r,t).unwrap()),o["live"],"{c}");rows(&mut counts,leaf);continue;
                }
                "436600"=>{
                    let selection:Vec<u32>=c["selection"].as_array().unwrap().iter().map(|v|r.id(v).unwrap()).collect();
                    let mut result=selection.first().copied();
                    for t in &selection{
                        if b4(&mut r,*t){if let Some(voiced)=voiced_436600(&mut r,*t).unwrap(){result=voiced;break}}
                    }
                    assert_eq!(r.name(result),o["result"],"{c}");rows(&mut counts,leaf);continue;
                }
                //view+9C NULL: 453470 returns with no call (the leaves' no-op).
                "453470"=>{assert_eq!(o["returned"],true);assert_eq!(o["trace"],json!([]));rows(&mut counts,leaf);continue;}
                other=>panic!("{other}"),
            };
            assert_eq!(value,o["value"],"{c}");
            assert_eq!(json!(r.trace),o["trace"],"seam order {c}");
            rows(&mut counts,leaf);
        }
        fn rows(counts:&mut BTreeMap<String,usize>,leaf:&str){*counts.entry(leaf.to_string()).or_default()+=1}
        assert_eq!(counts.values().sum::<usize>(),775,"{counts:?}");
        //Every reference to the relation/default globals is a load.
        for(global,initial) in [("0xb28158","0x2"),("0xb2815c","0x3"),("0xb3cc00","uninitialized (zero-filled)"),("0xb3cc24","uninitialized (zero-filled)")]{
            assert_eq!(fixture["globals"][global]["initial"],initial);
            for r in fixture["globals"][global]["references"].as_array().unwrap(){
                let text=r["text"].as_str().unwrap();
                assert!(!text.split_once(' ').unwrap().1.starts_with(&format!("dword ptr [{global}]"))||text.starts_with("cmp "),"{text}");
            }
        }
        assert!(data_virtuals(20,0).is_err());
    }
}
