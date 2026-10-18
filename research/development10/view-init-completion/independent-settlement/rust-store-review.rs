#[path=r"E:\Games\Axis and Alies\rust-rebuild\research\development10\view-init-completion\independent-settlement\world_revision-production.rs"]mod world_revision;
#[path=r"E:\Games\Axis and Alies\rust-rebuild\crates\aa-runtime\src\view_init_prefix.rs"]mod view_init_prefix;

#[test]
fn private_store_guard_preserves_ac_and_resource_callback_is_separate(){
    use view_init_prefix::{State,Services};
    struct Children{stop:bool}
    impl Services for Children{
        type Grid=usize;
        fn begin_screen_7040e0(&mut self)->Result<(),String>{Ok(())}
        fn reset_user_746e30(&mut self)->Result<(),String>{if self.stop{Err("required user".into())}else{Ok(())}}
        fn resource_context_5ae0a0(&mut self)->Result<(),String>{Ok(())}
        fn construct_grid_430290(&mut self)->Result<Option<usize>,String>{Ok(Some(7))}
        fn resetting_editor_variables_diagnostic(&mut self)->Result<(),String>{Ok(())}
        fn reset_view_variables(&mut self)->Result<(),String>{Ok(())}
    }
    let mut app=world_revision::Counter::fresh_application();let world=app.construct();
    let mut foreign_app=world_revision::Counter::fresh_application();let foreign=foreign_app.construct();
    assert_eq!(world.value(),foreign.value());
    let mut fresh=State::<usize>::from_constructor_fields();
    assert!(fresh.complete_full_init_43ff36(&world).is_err());assert!(!fresh.initialized());assert!(!fresh.initializing());
    let mut failed=State::<usize>::from_constructor_fields();
    assert!(failed.initialize_prefix(&world,&mut Children{stop:true}).is_err());
    assert!(failed.complete_full_init_43ff36(&world).is_err());assert!(!failed.initialized());assert!(failed.initializing());
    let mut valid=State::<usize>::from_constructor_fields();valid.initialize_prefix(&world,&mut Children{stop:false}).unwrap();
    assert!(valid.complete_full_init_43ff36(&foreign).is_err());assert!(!valid.initialized());assert!(valid.initializing());
    valid.complete_full_init_43ff36(&world).unwrap();assert!(valid.initialized_for(&world).unwrap());assert!(valid.initializing());
    assert!(!valid.uninitializing());assert!(valid.resource_callback_5b355d(&foreign).is_err());assert!(valid.initializing());
    valid.resource_callback_5b355d(&world).unwrap();assert!(valid.initialized());assert!(!valid.initializing());assert!(!valid.uninitializing());
}
