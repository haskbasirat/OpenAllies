#[path="../../aa-client/src/windows_film_time.rs"]
mod windows_film_time;

#[test]
#[ignore="requires actual AA_GAME_ROOT sources and Windows clock"]
fn actual_source_application_metadata_and_local_filetime() {
    use aa_rules::{actors::ActorCatalog,sources::{SourceLimits,V14Sources}};
    use aa_runtime::{catalog::Catalog,film_process_metadata::State,load_view_variables::NativeOverrides};
    let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {max_member_bytes:u64::MAX}).unwrap();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();
    let catalog=Catalog::new(&actors);
    assert!(State::from_fresh_sources(&catalog,&mut files,NativeOverrides::Unbound).is_err());
    let current=State::from_fresh_sources(&catalog,&mut files,NativeOverrides::ControlledNone).unwrap();
    assert_eq!(String::from_utf16(current.version_text()).unwrap(),"1.0.5.0");
    assert_eq!(String::from_utf16(current.application()).unwrap(),"A&A");
    assert_eq!(String::from_utf16(current.release()).unwrap(),"English AA");
    assert_eq!(current.numeric_version(),1_000_500);
    assert_eq!(current.build_bits(),0x4581_0d9a);
    assert_eq!(current.constructed_headers(),0);
    assert!(!current.source_files().is_empty());
    //This proves the actual platform leaf succeeds; it is neither a fixed
    //timestamp fixture nor an invented conversion to simulation milliseconds.
    assert_ne!(windows_film_time::local_file_time().unwrap(),[0,0]);
}
