//! Native50CE00 header and50C800 submission over an owned presentation queue.
//! FXC compilation and the view attachment/drain are separate required stages.
//! This module never treats authored FX inputs as compiled commands.
use std::collections::VecDeque;

pub type Result<T> = std::result::Result<T,String>;

pub trait Effects<T> {
    ///43B870: synchronous delivery to the actual attached view.
    fn deliver(&mut self,view:u64,message:Option<T>)->Result<()>;
    ///428350 then6F7430 for a nonnull command discarded while muted.
    fn destroy(&mut self,message:T)->Result<()>;
}

/// Native50CA40 array entry used by50D120: type1 reads its CString+48.
/// Other message kinds require their own real constructors and payloads.
pub struct Broadcast {text:Vec<u16>}
impl Broadcast {pub fn text(&self)->&[u16] {&self.text}}

/// Native+8/+C list is authoritative. T is an actual owned compiled command,
/// never a source-count placeholder. There is no Clone/Default capability.
pub struct State<T> {
    muted:bool,
    view:Option<u64>,
    pending:VecDeque<Option<T>>,
    enabled_1c:bool,
    broadcasts:Vec<Broadcast>,
}
impl<T> State<T> {
    ///50CE00 initializes mute0/viewNULL/listNULL/active-count0/enabled1.
    /// Its initial capacity4 is allocation policy and is not a limit.
    pub fn from_native_constructor()->Self {
        Self {muted:false,view:None,pending:VecDeque::new(),enabled_1c:true,broadcasts:Vec::new()}
    }
    pub fn muted(&self)->bool {self.muted}
    pub fn attached_view(&self)->Option<u64> {self.view}
    pub fn enabled_1c(&self)->bool {self.enabled_1c}
    pub fn pending(&self)->impl Iterator<Item=&Option<T>> {self.pending.iter()}
    pub fn broadcasts(&self)->&[Broadcast] {&self.broadcasts}

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
}
