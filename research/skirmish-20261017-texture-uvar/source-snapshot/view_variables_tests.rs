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
                let initial=if matches!(metadata.index,108|113){1}else{0};
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
        assert_eq!(state.raw("RendererUseHWSkinningShader"),Some(c["before"].as_u64().unwrap()as u8));
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
fn native_dev_mode_flags_two_denies_ordinary_writes_without_other_reads(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-frame-leaves/native-dev-mode.json")).unwrap();
    for row in fixture["cases"].as_array().unwrap(){
        let c=&row["input"];let expected=&row["output"];let mut state=State::from_native_constructors();
        let slot=State::slot("DevMode").unwrap();state.cells[slot].current=c["before"].as_u64().unwrap()as u8;
        state.cells[slot].reset_default=77;
        state.assign("DevMode",c["requested"].as_u64().unwrap()as u8,None).unwrap();
        assert_eq!(expected["allowed"],0);assert_eq!(state.cells[slot].current as u64,expected["current"].as_u64().unwrap());
        assert_eq!(state.cells[slot].reset_default as u64,expected["default"].as_u64().unwrap());
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(),144);
}

struct NamedBoolFiles{path:std::path::PathBuf,files:Vec<std::path::PathBuf>}
impl NamedBoolFiles{
    fn new(declarations:&[String])->Self{
        static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
        let path=std::env::temp_dir().join(format!("aa-named-texture-uvar-{}-{}",std::process::id(),NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();let mut files=Vec::new();
        for (i,declaration)in declarations.iter().enumerate(){
            let file=path.join(if i==0{"UVars.tgi".into()}else{format!("UVars_{i}.tgi")});
            std::fs::write(&file,format!("[Vars]\n{{\n{declaration}\n}}\n")).unwrap();files.push(file);
        }
        Self{path,files}
    }
}
impl Drop for NamedBoolFiles{fn drop(&mut self){for file in &self.files{std::fs::remove_file(file).unwrap()}std::fs::remove_dir(&self.path).unwrap()}}

#[test]
fn native_named_texture_bool_creation_override_and_live_raw_byte(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-texture-uvar/native-texture-uvar.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),15);
    assert!(!registrations().iter().any(|v|v.name.eq_ignore_ascii_case(RESOURCE_NO_NI_TEXTURES)));
    for row in rows{
        let declarations=row["input"]["sources"].as_array().unwrap().iter().map(|v|v.as_str().unwrap().to_owned()).collect::<Vec<_>>();
        let files=NamedBoolFiles::new(&declarations);let mut state=State::from_native_constructors();
        assert!(state.raw(RESOURCE_NO_NI_TEXTURES).is_none());
        state.bind_fresh_source(&DataRoot::new(&files.path),SourceOverrides::ControlledNone).unwrap();
        let final_native=row["native"]["stages"].as_array().unwrap().last().unwrap();
        assert_eq!(state.raw(RESOURCE_NO_NI_TEXTURES),Some(final_native["current"].as_u64().unwrap()as u8));
        assert_eq!(state.default_raw(RESOURCE_NO_NI_TEXTURES),state.raw(RESOURCE_NO_NI_TEXTURES));
        for sample in row["native"]["samples"].as_array().unwrap(){
            state.assign(RESOURCE_NO_NI_TEXTURES,sample["requested"].as_u64().unwrap()as u8,None).unwrap();
            let cell=state.resource_no_ni_textures.as_ref().unwrap();
            assert_eq!(cell.current as u64,sample["current"].as_u64().unwrap());
            assert_eq!(cell.reset_default as u64,sample["default"].as_u64().unwrap());
            //The texture backend owns these Resource flag stores separately.
            let before=sample["before_flags"].as_u64().unwrap();
            assert_eq!(if cell.current==1{before|0x40}else{before},sample["after_flags"].as_u64().unwrap());
            let current=cell.current;
            state.reset(||Ok(0)).unwrap();assert_eq!(state.raw(RESOURCE_NO_NI_TEXTURES),Some(current));
        }
        assert_eq!(row["native"]["lookups"],serde_json::json!([[RESOURCE_NO_NI_TEXTURES,1],[RESOURCE_NO_NI_TEXTURES,1]]));
    }
}

#[test]
fn named_texture_bool_has_no_missing_source_default_or_parallel_ui_map(){
    use crate::{tgi::TgiStore,uvars::UVars};
    let missing=NamedBoolFiles::new(&[String::new()]);let mut store=TgiStore::new(DataRoot::new(&missing.path));
    let absent=UVars::load(&mut store);assert!(absent.resource_no_ni_textures_current().is_err());assert!(absent.def(RESOURCE_NO_NI_TEXTURES).is_none());
    let files=NamedBoolFiles::new(&[format!("flag {RESOURCE_NO_NI_TEXTURES} = false"),format!("{RESOURCE_NO_NI_TEXTURES} = true")]);
    let mut store=TgiStore::new(DataRoot::new(&files.path));let mut vars=UVars::load(&mut store);
    assert_eq!(vars.resource_no_ni_textures_current().unwrap(),1);assert!(vars.default_bool(RESOURCE_NO_NI_TEXTURES));
    vars.assign_view_bool(RESOURCE_NO_NI_TEXTURES,128,None).unwrap();assert_eq!(vars.resource_no_ni_textures_current().unwrap(),128);
    vars.reset_view_for_loaded_map(||Ok(0)).unwrap();assert_eq!(vars.resource_no_ni_textures_current().unwrap(),128);
    let saved=vars.overrides();assert_eq!(saved.get(RESOURCE_NO_NI_TEXTURES).map(String::as_str),Some("128"));
    let mut copy=UVars::load(&mut store);copy.apply_overrides(&saved);assert_eq!(copy.resource_no_ni_textures_current().unwrap(),128);
    copy.set_bool(RESOURCE_NO_NI_TEXTURES,false);assert_eq!(copy.resource_no_ni_textures_current().unwrap(),0);
    assert_eq!(vars.resource_no_ni_textures_current().unwrap(),128);
    let unbound=UVars::load_with_view_scope(&mut store,SourceOverrides::Unbound);assert!(unbound.resource_no_ni_textures_current().is_err());
    let bad=NamedBoolFiles::new(&[format!("flag {RESOURCE_NO_NI_TEXTURES} = true"),format!("int {RESOURCE_NO_NI_TEXTURES} = 0")]);
    let mut state=State::from_native_constructors();assert!(state.bind_fresh_source(&DataRoot::new(&bad.path),SourceOverrides::ControlledNone).is_err());
    assert_eq!(state.raw(RESOURCE_NO_NI_TEXTURES),Some(1));assert!(state.require_current().is_err());
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
    assert_eq!(values.renderer_use_hw_skinning_shader().unwrap(),1);
    assert_eq!(values.dev_mode_current().unwrap(),0);
    assert_eq!(values.resource_no_ni_textures_current().unwrap(),0);
    values.set_bool("DevMode",true);assert_eq!(values.dev_mode_current().unwrap(),0);
    assert_eq!(values.view_bool_raw("DevMode"),Some(0));
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
    values.assign_view_bool("RendererUseHWSkinningShader",128,None).unwrap();
    values.reset_view_for_loaded_map(||Ok(0)).unwrap();
    assert_eq!(values.renderer_pre_request_all_vthings().unwrap(),128);
    assert_eq!(values.renderer_use_hw_skinning_shader().unwrap(),128);
    assert_eq!(values.dev_mode_current().unwrap(),0);
    assert_eq!(values.overrides().get("RendererPreRequestAllVThings").map(String::as_str),Some("128"));
    values.set_i32("TerrainQuadSize",2);assert_eq!(values.terrain_quad_size_current().unwrap(),2);
}
