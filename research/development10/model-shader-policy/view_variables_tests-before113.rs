use super::*;

#[test]
fn original_complete_registration_and_lazy_reset_prefixes_match() {
    let registration:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-view-init/native-uvar-registration.json")).unwrap();
    for case in registration["cases"].as_array().unwrap() {
        let rows=case["registrations"].as_array().unwrap();assert_eq!(rows.len(),registrations().len());
        for (row,metadata)in rows.iter().zip(registrations()) {
            let name=String::from_utf16(&row["name"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u16).collect::<Vec<_>>()).unwrap();
            assert_eq!(name,metadata.name);assert_eq!(row["index"].as_u64().unwrap(),metadata.index as u64);
            assert_eq!(row["kind"].as_u64().unwrap(),metadata.native_kind as u64);
            assert_eq!(row["flags"].as_u64().unwrap(),metadata.flags as u64);
            if OWNED_BOOL_INDICES.contains(&metadata.index) {
                let initial=if metadata.index==108{1}else{0};
                assert_eq!(row["bool_current"],initial);assert_eq!(row["bool_default"],initial);
            }
        }
    }
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-view-init/native-view-uvar-reset.json")).unwrap();
    let mut count=0;
    for row in fixture["cases"].as_array().unwrap() {
        let mut state=State::from_native_constructors();state.source_bound=true;
        let c=&row["input"];for cell in &mut state.cells {cell.current=c["before"].as_u64().unwrap()as u8;cell.reset_default=c["default"].as_u64().unwrap()as u8;}
        let phases=c["phases"].as_array().unwrap();let mut reads=0;
        let result=state.reset(||{
            reads+=1;
            if c["fail_read"].as_u64()==Some(reads) {return Err("phase_required".into())}
            Ok(phases[(reads as usize-1).min(phases.len()-1)].as_u64().unwrap()as u32)
        });
        let output=&row["output"];
        let trace=if output["stop"].is_null(){result.unwrap()}else {
            let failure=result.unwrap_err();assert_eq!(failure.reason,"phase_required");failure.trace
        };
        let mut ordinal=0;
        let events=trace.events.iter().map(|v|match v {
            ResetEvent::Lookup(i)=>serde_json::json!(["lookup",i]),
            ResetEvent::Reset(i)=>serde_json::json!(["reset",i]),
            ResetEvent::SessionRead=>{ordinal+=1;serde_json::json!(["phase",ordinal])},
        }).collect::<Vec<_>>();
        assert_eq!(serde_json::json!(events),output["events"],"{row}");
        assert_eq!(serde_json::json!(state.cells.iter().take(8).map(|v|v.current).collect::<Vec<_>>()),output["current"]);
        assert_eq!(serde_json::json!(state.cells.iter().take(8).map(|v|v.reset_default).collect::<Vec<_>>()),output["default"]);
        assert_eq!(state.raw("RendererPreRequestAllVThings"),Some(c["before"].as_u64().unwrap()as u8));
        assert_eq!(reads,output["phase_reads"].as_u64().unwrap());count+=1;
    }
    assert_eq!(count,84);
}

#[test]
fn unbound_source_and_real_editor_gate_preserve_cells() {
    let mut state=State::from_native_constructors();let mut reads=0;
    assert!(state.reset(||{reads+=1;Ok(0)}).is_err());assert_eq!(reads,0);
    assert!(state.assign("ViewShowBlocking",255,None).is_err());assert_eq!(state.raw("ViewShowBlocking"),Some(0));
    state.assign("ViewShowBlocking",128,Some(false)).unwrap();assert_eq!(state.raw("ViewShowBlocking"),Some(0));
    state.assign("ViewShowBlocking",128,Some(true)).unwrap();assert_eq!(state.raw("ViewShowBlocking"),Some(128));
    state.assign("CameraInformation",255,None).unwrap();assert_eq!(state.raw("CameraInformation"),Some(255));
}

#[test]
fn original_bool_raw_byte_and_editor_gate_outcomes_match() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-view-init/native-view-bool-assignment.json")).unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let c=&row["input"];let o=&row["output"];let mut state=State::from_native_constructors();
        let name=if c["flags"]==5 {"ViewShowBlocking"}else{"CameraInformation"};
        let slot=State::slot(name).unwrap();state.cells[slot].current=c["before"].as_u64().unwrap()as u8;
        state.cells[slot].reset_default=77;
        state.assign(name,c["requested"].as_u64().unwrap()as u8,Some(o["allowed"]!=0)).unwrap();
        assert_eq!(state.cells[slot].current as u64,o["current"].as_u64().unwrap(),"{row}");
        assert_eq!(state.cells[slot].reset_default as u64,o["default"].as_u64().unwrap());
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(),288);
}

#[test]
fn native_renderer_pre_request_flags_zero_preserves_raw_byte_and_reset_default(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/native-view-uvar108.json")).unwrap();
    let rows=fixture["cases"].as_array().unwrap();assert_eq!(rows.len(),144);
    for row in rows{
        let c=&row["input"];let expected=&row["output"];let mut state=State::from_native_constructors();
        let slot=State::slot("RendererPreRequestAllVThings").unwrap();
        state.cells[slot].current=c["before"].as_u64().unwrap()as u8;
        state.cells[slot].reset_default=77;
        state.assign("RendererPreRequestAllVThings",c["requested"].as_u64().unwrap()as u8,None).unwrap();
        assert_eq!(expected["allowed"],1);
        assert_eq!(state.cells[slot].current as u64,expected["current"].as_u64().unwrap());
        assert_eq!(state.cells[slot].reset_default as u64,expected["default"].as_u64().unwrap());
    }
}

#[test]
#[ignore="requires actual original/v14 extracted data; no native engine or GPU execution"]
fn actual_source_same_ui_camera_and_debug_cells_reset_without_a_shadow() {
    use crate::{tgi::TgiStore,uvars::UVars};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let root=DataRoot::with_overlay(game.join("reconstruction/assets/original/Data"),game.join("reconstruction/native-game/runtime-expanded-v14/Data"));
    let mut store=TgiStore::new(root);let mut values=UVars::load_with_view_scope(&mut store,SourceOverrides::ControlledNone);
    assert_eq!(values.view_source_failure(),None);assert_eq!(values.view_source_files().len(),3);
    assert_eq!(values.renderer_pre_request_all_vthings().unwrap(),1);
    assert_eq!(values.terrain_quad_size_current().unwrap(),1);
    assert!(values.get_bool("RendererPreRequestAllVThings"));
    for &index in &MUTABLE_INDICES {
        let name=registrations()[index as usize].name;
        assert_eq!(values.view_bool_raw(name),Some(0));assert!(!values.default_bool(name));
        values.assign_view_bool(name,255,Some(true)).unwrap();assert!(values.get_bool(name));
    }
    let mut calls=0;values.reset_view_for_loaded_map(||{calls+=1;Ok(7)}).unwrap();assert_eq!(calls,1);
    assert_eq!(values.renderer_pre_request_all_vthings().unwrap(),1);
    for &index in &MUTABLE_INDICES {assert_eq!(values.view_bool_raw(registrations()[index as usize].name),Some(if index==44 {255}else{0}));}
    assert_eq!(values.overrides().get("CameraInformation").map(String::as_str),Some("255"));
    let saved=values.overrides();let mut copy=UVars::load(&mut store);copy.apply_overrides(&saved);
    assert_eq!(copy.view_bool_raw("CameraInformation"),Some(255));assert!(copy.get_bool("CameraInformation"));
    calls=0;values.reset_view_for_loaded_map(||{calls+=1;Ok(0)}).unwrap();assert_eq!(calls,2);
    assert_eq!(values.view_bool_raw("CameraInformation"),Some(0));assert!(!values.get_bool("CameraInformation"));
    assert!(!values.overrides().contains_key("CameraInformation"));
    values.set_bool("CameraInformation",true);assert_eq!(values.view_bool_raw("CameraInformation"),Some(1));
    values.set_bool("CameraInformation",false);assert_eq!(values.view_bool_raw("CameraInformation"),Some(0));
    values.assign_view_bool("RendererPreRequestAllVThings",128,None).unwrap();
    values.reset_view_for_loaded_map(||Ok(0)).unwrap();
    assert_eq!(values.renderer_pre_request_all_vthings().unwrap(),128);
    assert_eq!(values.overrides().get("RendererPreRequestAllVThings").map(String::as_str),Some("128"));
    values.set_i32("TerrainQuadSize",2);assert_eq!(values.terrain_quad_size_current().unwrap(),2);
}
