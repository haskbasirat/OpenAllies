use super::*;
use serde_json::{Value,json};
struct Children{input:Value,events:Vec<String>,screen:u8}
impl Children{
    fn event(&mut self,label:&str)->Result<()>{
        if self.input["stop"].as_str()==Some(label){return Err(format!("explicit child cutoff:{label}"))}
        self.events.push(label.into());Ok(())
    }
}
impl Services for Children{
    type Grid=usize;
    fn begin_screen_7040e0(&mut self)->Result<()>{self.event("screen")?;self.screen=1;Ok(())}
    fn reset_user_746e30(&mut self)->Result<()>{self.event("user")}
    fn resource_context_5ae0a0(&mut self)->Result<()>{self.event("resource")}
    fn construct_grid_430290(&mut self)->Result<Option<usize>>{
        self.event("allocate_grid")?;
        if self.input["null_grid"].as_bool().unwrap(){Ok(None)}else{self.event("grid")?;Ok(Some(1))}
    }
    fn resetting_editor_variables_diagnostic(&mut self)->Result<()>{self.event("diagnostic")}
    fn reset_view_variables(&mut self)->Result<()>{self.event("uvars")}
}
fn word(v:&Value)->u32{v.as_u64().unwrap().try_into().unwrap()}
fn snapshot(f:&Fields<usize>)->Value{json!({"revision":f.init_word_b8,"flags":[f.initializing.load(Ordering::Acquire),f.initialized.load(Ordering::Acquire),f.uninitializing],"words":f.words,"grid":f.grid.is_some()})}
#[test]
fn original_v14_constructor_and_init_prefix_preserve_native_child_failure_stores(){
    let fixture:Value=serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/../../research/development10/native-vworld-prefix.json")).unwrap()).unwrap();
    let constructors=fixture["constructor_rows"].as_array().unwrap();assert_eq!(constructors.len(),18);
    for row in constructors{assert_eq!(snapshot(&Fields::<usize>::constructed()),row["output"]);}
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),360);
    for row in rows{
        let input=&row["input"];
        let mut fields=Fields{init_word_b8:word(&input["old_revision"]),initializing:Arc::new(AtomicU8::new(word(&input["flags"][0])as u8)),
            initialized:Arc::new(AtomicU8::new(word(&input["flags"][1])as u8)),uninitializing:word(&input["flags"][2])as u8,
            words:std::array::from_fn(|i|word(&input["words"][i])),grid:input["old_grid"].as_bool().unwrap().then_some(99)};
        let mut services=Children{input:input.clone(),events:vec![],screen:0};
        let result=prefix(&mut fields,word(&input["revision"]),&mut services);
        let expected=&row["output"];
        assert_eq!(result.is_ok(),expected["outcome"]=="prefix_complete");
        assert_eq!(snapshot(&fields),expected["fields"]);
        assert_eq!(json!(services.events),expected["events"]);
        assert_eq!(u32::from(services.screen),word(&expected["screen_byte24"]));
        assert_eq!(fields.initialized.load(Ordering::Acquire),0,"a prefix never creates nativeAD readiness");
    }
}
#[test]
fn public_prefix_retains_failed_owner_and_never_mints_initialized_state(){
    let mut application=crate::world_revision::Counter::fresh_application();let world=application.construct();
    let mut state=State::<usize>::from_constructor_fields();
    assert!(state.initialized_for(&world).is_err());
    assert!(state.message_word_8c_for(&world).is_err());
    let mut children=Children{input:json!({"stop":"uvars","null_grid":false}),events:vec![],screen:0};
    assert!(state.initialize_prefix(&world,&mut children).is_err());
    assert_eq!(state.revision(),world.value());assert!(state.initializing());assert!(!state.initialized());
    assert_eq!(state.grid(),Some(&1));assert!(state.require_prefix_complete().is_err());
    assert_eq!(state.initialized_for(&world).unwrap(),false);
    assert_eq!(state.message_word_8c_for(&world).unwrap(),0);
    let foreign=application.construct();assert!(state.initialized_for(&foreign).is_err());
    assert!(state.message_word_8c_for(&foreign).is_err());
    assert!(state.initialize_prefix(&world,&mut children).is_err());
}
#[test]
fn readonly_ad_reader_tracks_the_same_moved_field_and_retires_with_its_owner(){
    let mut application=crate::world_revision::Counter::fresh_application();let world=application.construct();
    let state=State::<usize>::from_constructor_fields();let read=state.initialized_reader();
    assert!(!read.read().unwrap());assert!(read.is_for(&state));
    let foreign=State::<usize>::from_constructor_fields();assert!(!read.is_for(&foreign));
    let mut moved=Box::new(state);assert!(read.is_for(&moved));
    let mut children=Children{input:json!({"stop":null,"null_grid":false}),events:vec![],screen:0};
    moved.initialize_prefix(&world,&mut children).unwrap();
    assert!(!read.read().unwrap(),"prefix completion alone cannot write AD");
    moved.complete_full_init_43ff36(&world).unwrap();
    assert!(read.read().unwrap());assert!(moved.initializing());
    moved.resource_callback_5b355d(&world).unwrap();assert!(read.read().unwrap());
    drop(moved);assert!(read.require_live().is_err());assert!(read.read().is_err());
}

#[test]
fn notification_b8_argument_is_independent_of_the_same_world_identity(){
    let mut app=crate::world_revision::Counter::fresh_application();let world=app.construct();
    for word in [0,7,u32::MAX]{
        let mut view=State::<usize>::from_constructor_fields();
        let mut children=Children{input:json!({"stop":"screen","null_grid":false}),events:vec![],screen:0};
        assert!(view.initialize_prefix_with_argument(&world,word,&mut children).is_err());
        assert_eq!(view.init_word_b8(),word);assert_eq!(view.initialized_for(&world).unwrap(),false);
        let foreign=app.construct();assert!(view.initialized_for(&foreign).is_err());
    }
}
