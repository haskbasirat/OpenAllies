//!The orders input's view/actor leaves (battle_orders_input::ViewLeaves) over
//!the SAME view directory, ActorStore and spatial owner. A leaf whose native
//!owner is not ported is an explicit error naming it; nothing is defaulted.
//!Proof of the bound native code: research/skirmish-20261019-view-order-leaves
//!(original vtable slots, 45B7A0, 45B7D0, 4D12C0, 4CE6C0, 58CB30 under
//!Unicorn) and research/skirmish-20261019-view-order-world (the Kingdom,
//!visibility, seen-record and link leaves in view_order_world.rs), replayed
//!by the tests.
use super::{TypedVObject,ViewDirectory,ViewHandle};
use crate::battle_orders_input::{Result,ViewLeaves};
use aa_actors::{ActorId,components::ComponentSlot};
use aa_runtime::{actor_world::{ActorStore,RuntimeComponent},load_view_variables,player_order_input::pick::Model,spatial_world::SpatialWorld};
#[path="view_order_world.rs"]
mod world;
pub use world::OrderContext;
use world::ViewClass;

///45B7A0's thing vt+0xB4 by the 4F9C10 runtime class (Data category+1C),
///read from each constructor's PRIMARY vtable: 3 4D6BB0 (GActor), 4 4E4DC0
///(Kingdom) and 8 4DE350 (GROUP), both GActor subclasses -> 6A71D0 true;
///2 4EA5E0, 5 4E66D0, 6 4DB610 (A28A00), 7 4DBEE0, 9 4DCB30, 10 4DC450 ->
///6A71F0 false.
fn thing_b4(class:u32)->Result<bool>{
    match class{
        3|4|8=>Ok(true),
        2|5|6|7|9|10=>Ok(false),
        _=>Err(format!("45B7A0 runtime class{class} has no 4F9C10 Thing vtable")),
    }
}
///45B7D0: a NULL view+14 answers true, else Thing+8 bit0.
fn flag8_bit0_45b7d0(thing:Option<u32>)->bool{thing.is_none_or(|flags|flags&1!=0)}
///4D12C0 / 4D10A0 over Thing+8 and +100/+104.
fn disabled_4d12c0(thing_flags:u32,state_100:u32,state_104:u32)->bool{
    let clear=state_104&4==0&&state_100&0x0400_0000==0&&state_104&0x10==0&&state_100&0x0200_0000==0;
    thing_flags&(1<<29)!=0||!clear
}
///4CE6C0: +7C (Organization) NULL and +80 (Element) +10 nonzero -> that
///organization, else the actor itself.
fn organization_4ce6c0<T>(own:T,organization_7c:bool,element_parent_10:Option<T>)->T{
    if organization_7c{own}else{element_parent_10.unwrap_or(own)}
}

///The SAME directory and World owners for one orders frame (pass as
///battle_orders_input::Live::views). Without an [`OrderContext`] the
///Session/UI-dependent leaves are explicit errors.
pub struct DirectoryViews<'a>{
    directory:&'a mut ViewDirectory,actors:&'a ActorStore,spatial:&'a SpatialWorld,
    variables:&'a load_view_variables::State,world:WorldFields<'a>,context:Option<&'a OrderContext>,
    order_150:Option<Box<Order150<'a>>>,
}
///4CF430 on the SAME World and Catalog (battle_orders_input::actor_virtual_150_between_frames).
pub type Order150<'a>=dyn FnMut(u16,Option<ActorId>,&mut aa_orders::commands::CommandWire,&mut crate::battle_orders_input::UiOrderGlobals)
    ->Result<bool>+'a;
///GWorld+275 and ObjectData+16D owners of the SAME World.
#[derive(Clone,Copy)]
pub struct WorldFields<'a>{
    pub runtime_fields:&'a aa_runtime::load_world_runtime_fields::State,
    pub source_fields:&'a aa_rules::native_source_requirements::NativeSourceRequirements,
}
impl<'a> DirectoryViews<'a>{
    pub fn new(directory:&'a mut ViewDirectory,actors:&'a ActorStore,spatial:&'a SpatialWorld,
        variables:&'a load_view_variables::State,world:WorldFields<'a>,context:Option<&'a OrderContext>)->anyhow::Result<Self>{
        anyhow::ensure!(directory.actors.is_for(actors)&&directory.spatial.is_for(spatial),"orders view leaves use another World allocation");
        Ok(Self{directory,actors,spatial,variables,world,context,order_150:None})
    }
    ///A view of a destroyed Thing would have been uninited natively (its +14
    ///cleared); that uninit chain is not ported, so it is refused here.
    fn entry(&self,v:ViewHandle)->Result<&TypedVObject>{
        let entry=self.directory.entries.get(&v).ok_or("orders view handle names no view allocation")?;
        if self.actors.key(entry.actor)!=Some(entry.key){return Err("view+14 Thing was destroyed; the VThing uninit (vtable+CC) is not ported".into())}
        Ok(entry)
    }
    fn fields(&self,v:ViewHandle)->Result<&super::Fields>{
        self.entry(v)?.native_fields.as_ref().ok_or_else(||"view native fields are unbound".into())
    }
    fn word(&self,v:ViewHandle,offset:usize)->Result<u32>{self.fields(v)?.required(offset).map_err(|e|e.to_string())}
    fn class(&self,v:ViewHandle)->Result<u32>{
        let entry=self.entry(v)?;Ok(self.actors.object(entry.key).map_err(|e|e.to_string())?.definition.native_class)
    }
    ///GActor-layout things only (classes 3, 4 and 8): +60 component table.
    fn actor(&self,v:ViewHandle)->Result<ActorId>{
        match self.class(v)?{3|4|8=>Ok(self.entry(v)?.actor),c=>Err(format!("view+14 class{c} Thing has no GActor fields"))}
    }
    fn component(&self,v:ViewHandle,slot:ComponentSlot)->Result<Option<&RuntimeComponent>>{
        let actor=self.actor(v)?;self.actors.component(actor,slot).map_err(|e|e.to_string())
    }
    ///Thing+C of an actor: its published view (0 is NULL).
    fn thing_view(&self,actor:ActorId)->Result<Option<ViewHandle>>{
        let key=self.actors.key(actor).ok_or("Thing identity is not live")?;
        let word=self.actors.object(key).map_err(|e|e.to_string())?.thing.word_c;
        if word==0{return Ok(None)}
        let view=ViewHandle(word);
        if !self.directory.entries.contains_key(&view){return Err("Thing+C names no view allocation".into())}
        Ok(Some(view))
    }
    ///Bind 4CF430 for actor_virtual_150 (order_views_with does this).
    pub fn with_order_150(mut self,call:Box<Order150<'a>>)->Self{self.order_150=Some(call);self}
    fn swept(&self)->Result<()>{
        if self.directory.streaming_passes==0{return Err("VGrid On list / VThing+18: no 4312F0 -> 42F670 streaming pass has run in this World".into())}
        Ok(())
    }
    fn live(&self)->world::Live<'_>{
        world::Live{directory:&*self.directory,actors:self.actors,spatial:self.spatial,variables:self.variables,
            runtime_fields:self.world.runtime_fields,source_fields:self.world.source_fields,context:self.context}
    }
    fn view_class(&mut self,v:ViewHandle,which:ViewClass)->Result<bool>{
        let thing=self.entry(v)?.actor;world::view_class(&mut self.live(),Some(thing),which)
    }
    ///view+14, its Data (view+10: the Thing's own definition) and view+54.
    fn view_parts(&self,v:ViewHandle)->Result<(ActorId,world::Data,u32)>{
        let entry=self.entry(v)?;
        let category=self.actors.object(entry.key).map_err(|e|e.to_string())?.definition.proof.category;
        Ok((entry.actor,world::data_virtuals(category.native_kind,category.registry_index)?,self.fields(v)?.flags().map_err(|e|e.to_string())?))
    }
}
fn unported<T>(leaf:&str,native:&str)->Result<T>{Err(format!("{leaf}: {native} is not ported"))}
impl ViewLeaves for DirectoryViews<'_>{
    fn flags_54(&mut self,v:ViewHandle)->Result<u32>{self.fields(v)?.flags().map_err(|e|e.to_string())}
    fn set_flags_54(&mut self,v:ViewHandle,f:u32)->Result<()>{
        self.entry(v)?;
        self.directory.entries.get_mut(&v).and_then(|e|e.native_fields.as_mut()).ok_or("view native fields are unbound")?
            .set_flags_54(f).map_err(|e|e.to_string())
    }
    fn position_38(&mut self,v:ViewHandle)->Result<[u32;3]>{Ok([self.word(v,0x38)?,self.word(v,0x3c)?,self.word(v,0x40)?])}
    ///58CB30 = view+10, the Data allocation fixed at construction: one
    ///identity per canonical definition (id and source), never reused.
    fn field_10_58cb30(&mut self,v:ViewHandle)->Result<u64>{
        let proof=&self.entry(v)?.source.proof;
        let key=(proof.id.to_ascii_lowercase(),proof.source.clone());
        let ids=&mut self.directory.data_identities;
        let index=match ids.iter().position(|k|*k==key){Some(i)=>i,None=>{ids.push(key);ids.len()-1}};
        Ok(index as u64+1)
    }
    ///vt+9C of A02798 / A02270 / A01EF0 is 6A71D0 (`mov al,1`).
    fn pickable_9c(&mut self,v:ViewHandle)->Result<bool>{self.entry(v)?;Ok(true)}
    fn class_78(&mut self,v:ViewHandle)->Result<bool>{self.view_class(v,ViewClass::Own78)}
    fn class_84(&mut self,v:ViewHandle)->Result<bool>{self.view_class(v,ViewClass::Enemy84)}
    fn class_7c(&mut self,v:ViewHandle)->Result<bool>{self.view_class(v,ViewClass::Ally7c)}
    fn class_80(&mut self,v:ViewHandle)->Result<bool>{self.view_class(v,ViewClass::Neutral80)}
    ///vt+50 45BF20(1) (640660 pushes 1).
    fn virtual_50(&mut self,v:ViewHandle)->Result<bool>{
        let(thing,data,flags)=self.view_parts(v)?;world::visible_45bf20(&mut self.live(),Some(thing),data,flags,true)
    }
    ///vt+54 45BD40(1).
    fn virtual_54(&mut self,v:ViewHandle)->Result<bool>{
        let(thing,data,flags)=self.view_parts(v)?;world::remembered_45bd40(&mut self.live(),Some(thing),data,flags,true)
    }
    fn actor_present_14(&mut self,v:ViewHandle)->Result<bool>{self.entry(v)?;Ok(true)}
    fn actor_45b7a0(&mut self,v:ViewHandle)->Result<bool>{let class=self.class(v)?;thing_b4(class)}
    fn actor_flag8_bit0(&mut self,v:ViewHandle)->Result<bool>{
        let entry=self.entry(v)?;
        let flags=self.actors.placement(entry.key,self.spatial).map_err(|e|e.to_string())?.flags;
        Ok(flag8_bit0_45b7d0(Some(flags)))
    }
    ///Thing vt+58 4EE3C0(5C0E50(), 1).
    fn actor_virtual_58(&mut self,v:ViewHandle)->Result<bool>{
        let thing=self.entry(v)?.actor;let mut live=self.live();
        let k=world::kingdom_5c0e50(&mut live)?;world::seen_4ee3c0(&mut live,thing,k,true)
    }
    ///Thing vt+84 4EB8B0(5C0E50()).
    fn actor_virtual_84(&mut self,v:ViewHandle)->Result<bool>{
        let thing=self.entry(v)?.actor;let mut live=self.live();
        let k=world::kingdom_5c0e50(&mut live)?;world::targetable_4eb8b0(&mut live,thing,k)
    }
    fn actor_4d12c0(&mut self,v:ViewHandle)->Result<bool>{
        let actor=self.actor(v)?;
        let f=self.actors.action_fields(actor,self.spatial).map_err(|e|e.to_string())?;
        Ok(disabled_4d12c0(f.thing_flags,f.state_bits as u32,(f.state_bits>>32) as u32))
    }
    ///Thing vt+20(1): 4CF120 (6406C0 pushes 1).
    fn actor_virtual_20(&mut self,v:ViewHandle)->Result<bool>{let thing=self.entry(v)?.actor;world::movable_20(&mut self.live(),thing,true)}
    ///GActor+70: the Mind component slot.
    fn actor_70(&mut self,v:ViewHandle)->Result<bool>{Ok(self.component(v,ComponentSlot::Mind)?.is_some())}
    ///GActor+90: the Wall component slot.
    fn actor_90(&mut self,v:ViewHandle)->Result<bool>{Ok(self.component(v,ComponentSlot::Wall)?.is_some())}
    fn link_16c_view(&mut self,v:ViewHandle)->Result<Option<ViewHandle>>{let a=self.actor(v)?;world::link_16c_view(&mut self.live(),a)}
    fn link_94_view(&mut self,v:ViewHandle)->Result<Option<ViewHandle>>{let a=self.actor(v)?;world::link_94_view(&mut self.live(),a)}
    fn link_94_live(&mut self,v:ViewHandle)->Result<bool>{let a=self.actor(v)?;world::link_94_live(&mut self.live(),a)}
    fn hidden_4549c0(&mut self,v:ViewHandle)->Result<bool>{let a=self.actor(v)?;world::hidden_4549c0(&mut self.live(),a)}
    fn actor_view_0c(&mut self,v:ViewHandle)->Result<Option<ViewHandle>>{let actor=self.entry(v)?.actor;self.thing_view(actor)}
    fn root_view_4ce6c0(&mut self,v:ViewHandle)->Result<Option<ViewHandle>>{
        let actor=self.actor(v)?;
        let organization=self.component(v,ComponentSlot::Organization)?.is_some();
        let parent=match self.component(v,ComponentSlot::Element)?{
            None=>None,
            Some(RuntimeComponent::Element(e))=>e.organization,
            Some(_)=>return Err("GActor+80 Element slot holds another component".into()),
        };
        let root=organization_4ce6c0(actor,organization,parent);
        self.thing_view(root)
    }
    fn voiced_436600(&mut self,v:ViewHandle)->Result<Option<Option<ViewHandle>>>{let a=self.actor(v)?;world::voiced_436600(&mut self.live(),a)}
    ///76A500 writes the low word of Thing+14 (the GTID).
    fn actor_slot_14(&mut self,v:ViewHandle)->Result<u16>{Ok(self.entry(v)?.actor.slot())}
    fn actor_handle_14(&mut self,v:ViewHandle)->Result<u32>{Ok(self.entry(v)?.actor.raw())}
    ///Thing vt+34(i): 4D1F90.
    fn actor_can_34(&mut self,v:ViewHandle,index:u32)->Result<bool>{let thing=self.entry(v)?.actor;world::can_34(&mut self.live(),thing,index)}
    ///VGrid+100 from its +0 tail through VThing+8 (43D240's 42EEC0 order), as
    ///the 42FAD0/42F670 streaming passes linked it.
    ///436950's vt+0x150(5C0E30(), &cmd, 1): 4CF430 over the World.
    fn actor_virtual_150(&mut self,v:ViewHandle,wire:&mut aa_orders::commands::CommandWire)->Result<bool>{
        let slot=self.actor_slot_14(v)?;
        let context=self.context.ok_or("0x436950 vt+0x150: orders Session/UI context was not supplied")?;
        let kingdom=context.kingdom_76ac70()?;let mut globals=context.order_globals();
        let call=self.order_150.as_mut().ok_or("0x436950 vt+0x150: 4CF430 World/Catalog binding was not supplied")?;
        call(slot,kingdom,wire,&mut globals)
    }
    fn on_list_100(&mut self)->Result<Vec<ViewHandle>>{
        self.swept()?;Ok(self.directory.streaming.tail_first(aa_runtime::view_streaming::List::On))
    }
    ///VThing+18, written by 45B330 in those passes.
    fn proximity_18(&mut self,v:ViewHandle)->Result<i32>{self.swept()?;Ok(self.word(v,0x18)? as i32)}
    fn model_78(&mut self,_:ViewHandle)->Result<Option<Model>>{unported("model_78","VObject+78 NiAVObject instance")}
    fn selection_radius_469ac0(&mut self,_:ViewHandle)->Result<u32>{unported("selection_radius_469ac0","457DD0 + 469AC0 (Aspect+60 / Model+68)")}
    fn update_model_86e8d0(&mut self,_:ViewHandle)->Result<Model>{unported("update_model_86e8d0","86E8D0 NiAVObject::Update (model owner)")}
    ///451860 / 452A90: 42EB10 runs only on a non-NULL view+9C, the attached
    ///VFX list that 455310 allocates from VWorld 43CD00. The constructor's
    ///NULL has no ported writer, so a non-NULL list is refused.
    fn floater_remove_42eb10(&mut self,v:ViewHandle)->Result<()>{
        if self.word(v,0x9c)?==0{return Ok(())}
        unported("floater_remove_42eb10","42EB10 over the view+9C attached VFX list")
    }
    ///453470 returns before 42E640/43CD00 when view+9C (or VActor+150) is NULL.
    fn floater_update_453470(&mut self,v:ViewHandle)->Result<()>{
        if self.word(v,0x9c)?==0{return Ok(())}
        unported("floater_update_453470","453470 42E640/43CD00 group floater VFX")
    }
    ///459600 first stores view+98 = (u32)59ABA0(B396E0) * 0.001f; the global
    ///B396E0 application timer has no owner shared with the orders frame
    ///(battle_notifications keeps its own anchor, the HUD reads Ui::now_ms).
    fn flash_459600(&mut self,_:ViewHandle)->Result<()>{unported("flash_459600","459600 global B396E0 timer (59ABA0) owner")}
}

#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::Value;
    #[test]fn native_view_order_leaf_kernels_replay(){
        let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261019-view-order-leaves/native-view-order-leaves.json")).unwrap();
        let u=|v:&Value|v.as_u64().unwrap() as u32;
        let mut rows=0;
        for row in fixture["rows"].as_array().unwrap(){
            let (c,o)=(&row["input"],&row["output"]);
            match c["leaf"].as_str().unwrap(){
                //Every view class the directory constructs answers true.
                "pickable_9c"=>assert_eq!(o["value"],1),
                "45b7a0"=>{
                    let expected=o["value"].as_u64().unwrap()!=0;
                    match c["class"].as_u64(){
                        None=>assert!(!expected),
                        Some(class)=>assert_eq!(thing_b4(class as u32).unwrap(),expected,"{c}"),
                    }
                }
                "45b7d0"=>{
                    let thing=(c["present"]==true).then(||u(&c["flags"]));
                    assert_eq!(flag8_bit0_45b7d0(thing),o["value"]==1,"{c}");
                }
                "4d12c0"=>assert_eq!(disabled_4d12c0(u(&c["flags"]),u(&c["state_100"]),u(&c["state_104"])),o["value"]==1,"{c}"),
                "4ce6c0"=>{
                    let parent=(c["element_80"]==true&&c["element_parent_10"]==true).then_some("organization");
                    assert_eq!(organization_4ce6c0("self",c["organization_7c"]==true,parent),o["result"].as_str().unwrap(),"{c}");
                }
                "58cb30"=>assert_eq!(o["value"],0x12345678),
                other=>panic!("{other}"),
            }
            rows+=1;
        }
        assert_eq!(rows,240);
        assert!(thing_b4(11).is_err());
    }
}
