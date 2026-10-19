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

/// Native50CA40 array entry used by50D120: type1 reads its CString+48.
/// Other message kinds require their own real constructors and payloads.
pub struct Broadcast {text:Vec<u16>}
impl Broadcast {pub fn text(&self)->&[u16] {&self.text}}

/// The reached child of a type-1 message in 50C600 (case 1 at 50C63E):
/// 427950 on the script processor `[B36500]` with the message CString+48 and
/// flag 0, which evaluates the text immediately (426C20 TGC_ScriptProcessor::
/// Eval, statement by statement through 425E80). The implementation is the
/// actual script/UI command owner; it cannot reach this manager.
pub trait BroadcastDelivery {
    fn script_eval_427950(&mut self,text:&[u16],flag:u8)->Result<()>;
}

/// Native+8/+C list is authoritative. T is an actual owned compiled command,
/// never a source-count placeholder. There is no Clone/Default capability.
pub struct State<T> {
    muted:bool,
    view:Option<u64>,
    pending:VecDeque<Option<T>>,
    enabled_1c:bool,
    broadcasts:Vec<Broadcast>,
    ///The native node has already been removed at50CAD1. Keep its actual
    ///payload on a required-child failure, rather than dropping it early.
    detached:Option<Option<T>>,
}
impl<T> State<T> {
    ///50CE00 initializes mute0/viewNULL/listNULL/active-count0/enabled1.
    /// Its initial capacity4 is allocation policy and is not a limit.
    pub fn from_native_constructor()->Self {
        Self {muted:false,view:None,pending:VecDeque::new(),enabled_1c:true,broadcasts:Vec::new(),detached:None}
    }
    pub fn muted(&self)->bool {self.muted}
    pub fn attached_view(&self)->Option<u64> {self.view}
    pub fn enabled_1c(&self)->bool {self.enabled_1c}
    pub fn pending(&self)->impl Iterator<Item=&Option<T>> {self.pending.iter()}
    pub fn broadcasts(&self)->&[Broadcast] {&self.broadcasts}
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
    ///50C750 on World+130, the first child of 43EC60 (43ECE9): every entry of
    ///the +10 array is passed to 50C600 in index order while the index differs
    ///from the count +14 (reread each iteration), then +14 = 0. The +1C
    ///enabled byte and the +18 capacity are untouched, so a later 50D120 does
    ///not insert WaitWorld again. Only type-1 entries have a Rust producer
    ///(50D120); 50C600's other cases (432E40/43B270, 440190, 4408A0,
    ///43CD00/43CC50) have none, so they cannot be reached here. A refused child
    ///keeps the array and its count (native has no failure edge); the
    ///successful prefix of deliveries stays committed. Returns the number
    ///of delivered entries.
    ///Proof: research/phase8-vworld-update section messages_50c750.
    pub fn deliver_broadcasts_50c750(&mut self,delivery:&mut impl BroadcastDelivery)->Result<usize> {
        let mut index=0;
        while index!=self.broadcasts.len() {
            delivery.script_eval_427950(self.broadcasts[index].text(),0)?;
            index+=1;
        }
        self.broadcasts.clear();
        Ok(index)
    }
    fn append_broadcast(&mut self,text:&[u16])->Result<()> {
        self.broadcasts.try_reserve(1).map_err(|e|e.to_string())?;
        //Native increments count and writes type1 before CString assignment.
        self.broadcasts.push(Broadcast {text:Vec::new()});
        let value=&mut self.broadcasts.last_mut().expect("new message").text;
        value.try_reserve_exact(text.len()).map_err(|e|e.to_string())?;
        value.extend_from_slice(text);
        Ok(())
    }

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
    ///over arrays of type-1 entries (the only kind with a Rust producer). Rows
    ///with other kinds are native reference for their future producers.
    #[test]
    fn native_broadcast_walk_50c750() {
        let fixture=crate::vworld_update::tests_fixture::load();
        let rows=fixture["messages_50c750"].as_array().unwrap();
        assert_eq!(rows.len(),42);
        struct Sink(Vec<serde_json::Value>,Option<usize>);
        impl BroadcastDelivery for Sink {
            fn script_eval_427950(&mut self,text:&[u16],flag:u8)->Result<()> {
                if self.1==Some(self.0.len()) {return Err("explicit 427950 refusal".into());}
                self.0.push(serde_json::json!(["427950",String::from_utf16(text).unwrap(),flag]));Ok(())
            }
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
