//! Native50CE00 header and50C800 submission over an owned presentation queue.
//! FXC compilation and concrete view command delivery remain required stages.
//! This module never treats authored FX inputs as compiled commands.
use std::collections::VecDeque;

pub type Result<T> = std::result::Result<T,String>;

pub trait Effects<T> {
    ///43B870: synchronous delivery to the actual attached view.
    fn deliver(&mut self,view:u64,message:Option<T>)->Result<()>;
    ///428350 then6F7430 for a nonnull command discarded while muted.
    fn destroy(&mut self,message:T)->Result<()>;
}

///50CAB0's reached command children. A delivery moves a positive command out
///of `message` only when the actual destination has accepted its ownership.
///On failure the same detached payload remains owned by the manager. These
///loans expose no reentrant access to this manager's pending list.
pub trait AttachmentEffects<T> {
    fn deliver(&mut self,view:u64,message:&mut Option<T>)->Result<()>;
    fn destroy(&mut self,message:&mut T)->Result<()>;
}

/// The World+130 +10 broadcast array (slots, producers, 50C750/50C600).
#[path="world_broadcasts.rs"]
mod broadcasts;
pub use broadcasts::{Broadcast,BroadcastDelivery,RequestWorld};

/// Native+8/+C list is authoritative. T is an actual owned compiled command,
/// never a source-count placeholder. There is no Clone/Default capability.
pub struct State<T> {
    muted:bool,
    view:Option<u64>,
    pending:VecDeque<Option<T>>,
    enabled_1c:bool,
    broadcasts:broadcasts::Broadcasts,
    ///The native node has already been removed at50CAD1. Keep its actual
    ///payload on a required-child failure, rather than dropping it early.
    detached:Option<Option<T>>,
}
impl<T> State<T> {
    ///50CE00 initializes mute0/viewNULL/listNULL/active-count0/enabled1.
    /// Its initial capacity4 is allocation policy and is not a limit.
    pub fn from_native_constructor()->Self {
        Self {muted:false,view:None,pending:VecDeque::new(),enabled_1c:true,broadcasts:broadcasts::Broadcasts::from_native_constructor(),detached:None}
    }
    pub fn muted(&self)->bool {self.muted}
    pub fn attached_view(&self)->Option<u64> {self.view}
    pub fn enabled_1c(&self)->bool {self.enabled_1c}
    pub fn pending(&self)->impl Iterator<Item=&Option<T>> {self.pending.iter()}
    ///The live entries of the +10 array (below the +14 count).
    pub fn broadcasts(&self)->&[Broadcast] {self.broadcasts.live()}
    pub fn detached_command(&self)->Option<&Option<T>> {self.detached.as_ref()}

    ///50CAB0 stores the actual view first, then removes each original list
    ///node BEFORE dispatch. An unmuted null view with pending commands keeps
    ///following its reappended nodes in native code; that nonterminating edge
    ///is refused explicitly after the reached view assignment.
    ///The concrete callback must bind actual43B870/428350 behavior, or fail at
    ///that reached boundary. This method cannot certify VWorld initialization.
    pub fn attach_view(&mut self,view:Option<u64>,effects:&mut impl AttachmentEffects<T>)->Result<()> {
        if self.detached.is_some(){return Err("50CAB0 previous detached command child is incomplete".into());}
        if view==Some(0){return Err("50CAB0 positive portable view handle is zero".into());}
        self.view=view;
        if !self.muted&&self.view.is_none()&&!self.pending.is_empty(){return Err("50CAB0 unmuted null-view nonempty reappend loop is unsupported".into());}
        let original_nodes=self.pending.len();
        for _ in 0..original_nodes {
            self.detached=Some(self.pending.pop_front().ok_or("50CAB0 original node disappeared")?);
            if self.muted {
                if let Some(message)=self.detached.as_mut().expect("detached node").as_mut(){effects.destroy(message)?;}
            }else if let Some(view)=self.view {
                let message=self.detached.as_mut().expect("detached node");
                effects.deliver(view,message)?;
                if message.is_some(){return Err("43B870 destination did not accept the actual detached command".into());}
            }
            self.detached=None;
        }
        Ok(())
    }

    ///50D120 appends type1 messages to the separate +10 array. First append
    /// clears+1C before allocation and inserts WaitWorld. It does not deliver
    /// to the attached UI or change the50C800 linked command queue.
    pub fn broadcast(&mut self,text:&[u16])->Result<()> {
        if self.enabled_1c {
            self.enabled_1c=false;
            self.append_broadcast(&"WaitWorld".encode_utf16().collect::<Vec<_>>())?;
        }
        self.append_broadcast(text)
    }
    ///50C750 on World+130, the first child of 43EC60 (43ECE9): every slot
    ///below the +14 count (reread each iteration) through 50C600 in index
    ///order, then +14 = 0. The +1C byte, the capacity and every slot word are
    ///untouched, so a later 50D120 does not insert WaitWorld again and a
    ///reused slot keeps its stale words (see world_broadcasts.rs). Case 0
    ///uses the attached view (+4). A refused child keeps the count (native
    ///has no failure edge); the successful prefix of deliveries stays
    ///committed. Returns the number of delivered entries.
    ///Proof: research/phase8-vworld-update messages_50c750 and
    ///native-world-broadcasts.json.
    pub fn deliver_broadcasts_50c750<D:BroadcastDelivery>(&mut self,delivery:&mut D)->Result<usize> {
        self.broadcasts.walk_50c750(self.view,delivery)
    }
    fn append_broadcast(&mut self,text:&[u16])->Result<()> {
        self.broadcasts.append_text(text)
    }
    ///50D0B0(actor, &record): the gates and the actor+14 handle are
    ///aa_combat's port of the same function (World+D8 != 6, ViewShowDamage,
    ///actor+C definition +54 bit 1); a queued number becomes a kind-0 slot.
    pub fn queue_damage_number_50d0b0(&mut self,actor:u32,record:&aa_body::damage::DamageRecord,
        world:&impl aa_combat::presentation::World)->Result<()> {
        let mut queued=Vec::new();
        aa_combat::presentation::queue_damage_number(&mut queued,actor,record,world).map_err(|e|format!("50D0B0: {e}"))?;
        match queued.pop() {
            Some(number)=>self.broadcasts.append_thing_record(number.actor_id,number.record.words()),
            None=>Ok(()),
        }
    }
    ///50D230(definition, thing): a VFX attached to the thing whose +14
    ///handle word is `thing_14` (None: a null thing, no request).
    pub fn queue_attached_vfx_50d230(&mut self,world:&mut impl RequestWorld,definition:Option<u64>,
        thing_14:Option<u32>)->Result<()> {
        self.broadcasts.append_attached_vfx_50d230(world,definition,thing_14)
    }
    ///50D280(definition, &position, &direction): a detached VFX; only the
    ///first two direction words are read, +40 = 0, +28 is not written.
    pub fn queue_detached_vfx_50d280(&mut self,world:&mut impl RequestWorld,definition:Option<u64>,
        position:[u32;3],direction:[u32;2])->Result<()> {
        self.broadcasts.append_detached_vfx_50d280(world,definition,position,direction)
    }
    ///50D370: kind 2 (440190 mission result).
    pub fn queue_mission_result_50d370(&mut self)->Result<()> {self.broadcasts.append_kind(2)}
    ///50D330: kind 3 (4408A0 team victory).
    pub fn queue_team_victory_50d330(&mut self)->Result<()> {self.broadcasts.append_kind(3)}

    ///50C800 reaches the mute gate first, then the attached view, then the
    /// enqueue flag. A true flag prepends; false appends through45F240.
    /// Delivery errors preserve all earlier effects. No renderer succeeds here
    /// unless its concrete implementation really accepts the command.
    pub fn submit(&mut self,message:Option<T>,flag:u8,effects:&mut impl Effects<T>)->Result<()> {
        if self.muted {
            if let Some(message)=message {effects.destroy(message)?;}
        } else if let Some(view)=self.view {
            effects.deliver(view,message)?;
        } else {
            self.pending.try_reserve(1).map_err(|e|e.to_string())?;
            if flag==0 {self.pending.push_back(message);} else {self.pending.push_front(message);}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Sink(Vec<serde_json::Value>);
    impl Effects<u32> for Sink {
        fn deliver(&mut self,_:u64,message:Option<u32>)->Result<()> {
            self.0.push(serde_json::json!(["deliver",message.unwrap_or(0)]));Ok(())
        }
        fn destroy(&mut self,message:u32)->Result<()> {
            self.0.push(serde_json::json!(["destroy",message]));Ok(())
        }
    }
    #[test]
    fn native_header_and_submission_orders() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!(
            "../../../research/development10/fx-manager/native-fx-manager.json")).unwrap();
        assert_eq!(fixture["rows"].as_array().unwrap().len(),72);
        assert_eq!(fixture["constructors"].as_array().unwrap().len(),9);
        for row in fixture["constructors"].as_array().unwrap() {
            let state=State::<u32>::from_native_constructor();
            assert_eq!(serde_json::json!(u8::from(state.muted())),row["output"]["muted"]);
            assert_eq!(row["output"]["view"],0);
            assert_eq!(row["output"]["head"],0);
            assert_eq!(row["output"]["tail"],0);
            assert_eq!(row["output"]["active_count"],0);
            assert!(state.pending().next().is_none());
            assert_eq!(serde_json::json!(u8::from(state.enabled_1c())),row["output"]["enabled"]);
        }
        for row in fixture["rows"].as_array().unwrap() {
            let mut state=State::from_native_constructor();
            assert!(!state.muted() && state.attached_view().is_none() && state.enabled_1c());
            // Explicit laboratory pre-state from native fixture, not an
            // application path for attaching a view without its real drain.
            state.muted=row["input"]["muted"].as_bool().unwrap();
            state.view=row["input"]["view"].as_bool().unwrap().then_some(1);
            state.pending.extend([Some(11),None,Some(12)]);
            let mut sink=Sink(Vec::new());
            for item in row["input"]["submissions"].as_array().unwrap() {
                let message=item[0].as_u64().unwrap() as u32;
                state.submit((message!=0).then_some(message),item[1].as_u64().unwrap() as u8,&mut sink).unwrap();
            }
            assert_eq!(serde_json::json!(sink.0),row["output"]["calls"]);
            let pending:Vec<_>=state.pending().map(|p|p.unwrap_or(0)).collect();
            assert_eq!(serde_json::json!(pending),row["output"]["pending"]);
        }
    }
    #[test]
    fn native_broadcast_array_and_wait_world_order() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!(
            "../../../research/development10/fx-manager/native-broadcast.json")).unwrap();
        assert_eq!(fixture["rows"].as_array().unwrap().len(),36);
        for row in fixture["rows"].as_array().unwrap() {
            let mut owner=State::<u32>::from_native_constructor();
            owner.enabled_1c=row["input"]["enabled"].as_u64().unwrap()!=0;
            for text in row["input"]["prior"].as_array().unwrap() {
                owner.append_broadcast(&text.as_str().unwrap().encode_utf16().collect::<Vec<_>>()).unwrap();
            }
            for text in row["input"]["submissions"].as_array().unwrap() {
                owner.broadcast(&text.as_str().unwrap().encode_utf16().collect::<Vec<_>>()).unwrap();
            }
            let messages:Vec<_>=owner.broadcasts().iter().map(|b|serde_json::json!({
                "kind":1,"text":String::from_utf16(b.text()).unwrap()})).collect();
            assert_eq!(serde_json::json!(messages),row["output"]["messages"]);
            assert_eq!(serde_json::json!(u8::from(owner.enabled_1c())),row["output"]["enabled"]);
            assert!(owner.pending().next().is_none());
        }
    }
    ///research/phase8-vworld-update messages_50c750: original 50C750/50C600
    ///over arrays of type-1 entries. Its rows with other kinds seed raw slot
    ///words with a pointer-keyed 432E40 seam; every kind is replayed through
    ///the Rust producers in world_broadcasts.rs (native-world-broadcasts.json).
    #[test]
    fn native_broadcast_walk_50c750() {
        let fixture=crate::vworld_update::tests_fixture::load();
        let rows=fixture["messages_50c750"].as_array().unwrap();
        assert_eq!(rows.len(),42);
        struct Sink(Vec<serde_json::Value>,Option<usize>);
        impl BroadcastDelivery for Sink {
            type Thing=();
            type Vfx=();
            fn script_eval_427950(&mut self,text:&[u16],flag:u8)->Result<()> {
                if self.1==Some(self.0.len()) {return Err("explicit 427950 refusal".into());}
                self.0.push(serde_json::json!(["427950",String::from_utf16(text).unwrap(),flag]));Ok(())
            }
            fn thing_432e40(&mut self,_:u32)->Result<Option<()>> {unreachable!("type-1 rows only")}
            fn thing_record_43b270(&mut self,_:u64,_:Option<()>,_:&[u32;7])->Result<()> {unreachable!()}
            fn mission_result_440190(&mut self)->Result<()> {unreachable!()}
            fn team_victory_4408a0(&mut self)->Result<()> {unreachable!()}
            fn attached_vfx_43cd00(&mut self,_:u64,_:())->Result<Option<()>> {unreachable!()}
            fn detached_vfx_43cc50(&mut self,_:u64,_:[u32;3],_:[u32;3])->Result<Option<()>> {unreachable!()}
            fn vfx_time_50(&mut self,_:(),_:u32)->Result<()> {unreachable!()}
        }
        let mut replayed=0;
        for row in rows {
            let messages=row["input"]["messages"].as_array().unwrap();
            if messages.iter().any(|m|m["kind"]!=1) {continue;}
            let mut owner=State::<u32>::from_native_constructor();
            for m in messages {owner.append_broadcast(&m["text"].as_str().unwrap().encode_utf16().collect::<Vec<_>>()).unwrap();}
            let enabled=owner.enabled_1c();
            let mut sink=Sink(Vec::new(),None);
            let delivered=owner.deliver_broadcasts_50c750(&mut sink).unwrap();
            assert_eq!(serde_json::json!(sink.0),row["output"]["trace"],"{row}");
            assert_eq!(owner.broadcasts().len() as u64,row["output"]["count"].as_u64().unwrap());
            assert_eq!(delivered,messages.len());
            assert_eq!(owner.enabled_1c(),enabled);
            replayed+=1;
        }
        assert_eq!(replayed,24);
        //A refused child keeps the array and count; the delivered prefix stays.
        let mut owner=State::<u32>::from_native_constructor();
        owner.broadcast(&"TriggerListUpdate".encode_utf16().collect::<Vec<_>>()).unwrap();
        let mut sink=Sink(Vec::new(),Some(1));
        assert!(owner.deliver_broadcasts_50c750(&mut sink).is_err());
        assert_eq!(sink.0.len(),1);
        assert_eq!(owner.broadcasts().len(),2);
        //After a successful walk a new 50D120 appends without a second WaitWorld.
        let mut sink=Sink(Vec::new(),None);
        assert_eq!(owner.deliver_broadcasts_50c750(&mut sink).unwrap(),2);
        owner.broadcast(&"again".encode_utf16().collect::<Vec<_>>()).unwrap();
        assert_eq!(owner.broadcasts().len(),1);
        assert_eq!(owner.broadcasts()[0].text(),"again".encode_utf16().collect::<Vec<_>>().as_slice());
    }
    #[test]
    fn native_view_attachment_drain_and_required_child_prefixes() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!(
            "../../../research/development10/fx-manager/native-view-attachment.json")).unwrap();
        assert_eq!(fixture["rows"].as_array().unwrap().len(),120);
        struct Attached {calls:Vec<serde_json::Value>,fail:Option<usize>}
        impl AttachmentEffects<u32> for Attached {
            fn deliver(&mut self,view:u64,message:&mut Option<u32>)->Result<()> {
                assert_eq!(view,1);self.calls.push(serde_json::json!(["deliver",message.unwrap_or(0)]));
                if self.fail==Some(self.calls.len()){return Err("required deliver".into());}
                message.take();Ok(())
            }
            fn destroy(&mut self,message:&mut u32)->Result<()> {
                self.calls.push(serde_json::json!(["destroy",*message]));
                if self.fail==Some(self.calls.len()){return Err("required destroy".into());}Ok(())
            }
        }
        for row in fixture["rows"].as_array().unwrap() {
            let c=&row["input"];let mut state=State::from_native_constructor();
            state.muted=c["muted"].as_bool().unwrap();state.view=Some(99);
            state.pending.extend(c["pending"].as_array().unwrap().iter().map(|v|{let n=v.as_u64().unwrap()as u32;(n!=0).then_some(n)}));
            let mut effects=Attached{calls:Vec::new(),fail:c["fail_at"].as_u64().map(|v|v as usize)};
            let result=state.attach_view(c["view"].as_bool().unwrap().then_some(1),&mut effects);
            assert_eq!(serde_json::json!(state.attached_view().is_some()),row["output"]["attached"]);
            assert_eq!(serde_json::json!(effects.calls),row["output"]["calls"]);
            assert_eq!(serde_json::json!(state.pending().map(|v|v.unwrap_or(0)).collect::<Vec<_>>()),row["output"]["pending"]);
            assert_eq!(serde_json::json!(result.err()),row["output"]["error"]);
            assert_eq!(state.detached_command().is_some(),!row["output"]["error"].is_null());
        }
        let mut state=State::from_native_constructor();state.pending.push_back(Some(11));
        let mut effects=Attached{calls:Vec::new(),fail:None};
        assert!(state.attach_view(None,&mut effects).unwrap_err().contains("nonempty reappend loop"));
        assert!(state.attached_view().is_none());assert_eq!(state.pending().collect::<Vec<_>>(),vec![&Some(11)]);
        assert!(effects.calls.is_empty());
    }
}
