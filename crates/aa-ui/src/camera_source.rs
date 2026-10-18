//!Immutable receipt for the source-CREATED RendererUseRenderLists Bool.
//!The current/default value lives only in UVars' existing maps. There is no
//!invented code default: without its actual source declaration it is unbound.
use crate::{
    fs::DataRoot,
    view_variables::{SourceFile, SourceOverrides},
};
use aa_rules::{
    native::index::native_enumerate,
    tgi::{Document, Item, Limits},
};
pub(crate) const NAME: &str = "RendererUseRenderLists";
///41F9D0 appended Bool registrations, kind1/default1/flags9; actual6-row proof.
pub(crate) const RENDER_LIST_NAMES: [&str; 13] = [
    "RenderlistDrawTerrain",
    "RenderlistDrawTerrainDecals",
    "RenderlistDrawLines",
    "RenderlistDrawSkinned",
    "RenderlistDrawUnskinned",
    "RenderlistDrawWater",
    "RenderlistDrawAlphaBlended",
    "RenderlistDrawParticles",
    "RenderlistDrawStencilShadows",
    "RenderlistDrawUIVFX",
    "RenderlistDrawWorldCatchAll",
    "RenderlistDrawUI",
    "RenderlistDrawUICatchAll",
];
pub(crate) const INPUT_SPECS:&[crate::uvars::Def]=&[
 crate::uvars::Def{name:"CameraScrollSpeed",kind:crate::uvars::Kind::Float,default:"2",range:Some((1.,20.))},
 crate::uvars::Def{name:"CameraScrollEdgePadding",kind:crate::uvars::Kind::Int,default:"4",range:Some((0.,10.))},
 crate::uvars::Def{name:"CameraScalePan",kind:crate::uvars::Kind::Float,default:"10",range:None},
 crate::uvars::Def{name:"CameraScaleZoom",kind:crate::uvars::Kind::Float,default:"0.5",range:None},
 crate::uvars::Def{name:"MapDefaultWidth",kind:crate::uvars::Kind::Int,default:"128",range:Some((16.,512.))},
 crate::uvars::Def{name:"MapDefaultHeight",kind:crate::uvars::Kind::Int,default:"128",range:Some((16.,512.))},
 crate::uvars::Def{name:"CameraRotate",kind:crate::uvars::Kind::Bool,default:"false",range:None},
 crate::uvars::Def{name:"EditorCameraUnlocked",kind:crate::uvars::Kind::Bool,default:"false",range:None},
];
pub(crate) struct Bound {
    pub use_lists: bool,
    pub switches: [bool; 13],
    pub input_values: Vec<String>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Receipt {
    files: Vec<SourceFile>,
    bound: bool,
    failure: Option<String>,
}
impl Receipt {
    pub(crate) fn bind(
        &mut self,
        root: &DataRoot,
        scope: SourceOverrides,
    ) -> Result<Bound, String> {
        if self.bound {
            return Err("camera UVar source already bound".into());
        }
        if scope != SourceOverrides::ControlledNone {
            return Err("camera current UVar profile/console owner is unbound".into());
        }
        let paths = native_enumerate("UVars*.tgi", &root.list_dir(""));
        if !paths.iter().any(|p| p.eq_ignore_ascii_case("UVars.tgi")) {
            return Err("camera UVars.tgi source absent".into());
        }
        let mut current = None;
        let mut switches = [true; 13];
        let mut input_values=INPUT_SPECS.iter().map(|s|s.default.to_owned()).collect::<Vec<_>>();
        for path in paths {
            let bytes = root.read(&path).map_err(|e| e.to_string())?;
            let hash = aa_rules::sources::sha256(&bytes);
            let doc =
                Document::parse_native(bytes, Limits::default()).map_err(|e| e.to_string())?;
            self.files.push(SourceFile { path, sha256: hash });
            let mut seen = std::collections::BTreeSet::new();
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|n| n.eq_ignore_ascii_case("Vars"))
                {
                    continue;
                }
                if node.header.tokens.len() != 1 {
                    return Err("camera UVar inherited Vars loader requires native binding".into());
                }
                for item in &node.items {
                    let Item::Property(p) = item else { continue };
                    let Some(name) = doc.property_name(p) else {
                        continue;
                    };
                    let slot = RENDER_LIST_NAMES
                        .iter()
                        .position(|n| n.eq_ignore_ascii_case(name));
                    let input_slot=INPUT_SPECS.iter().position(|s|s.name.eq_ignore_ascii_case(name));
                    if !name.eq_ignore_ascii_case(NAME) && slot.is_none() && input_slot.is_none() {
                        continue;
                    }
                    if !seen.insert(name.to_ascii_lowercase()) {
                        return Err("camera UVar duplicate field needs native hash traversal".into());
                    }
                    if let Some(i)=input_slot{
                        let spec=&INPUT_SPECS[i];
                        let kind=p.left.first().and_then(|t|doc.text(t.span)).ok_or("camera input source type absent")?;
                        if p.left.len()!=2{return Err("camera input source declaration needs actual explicit type".into())}
                        let raw=doc.property_value(p).ok_or("camera input source value absent")?.trim();
                        input_values[i]=match spec.kind{
                            crate::uvars::Kind::Bool if kind.eq_ignore_ascii_case("flag")||kind.eq_ignore_ascii_case("bool")=>{
                                if raw.eq_ignore_ascii_case("true"){"true".into()}else if raw.eq_ignore_ascii_case("false"){"false".into()}else{return Err("camera input source Bool conversion unbound".into())}
                            },
                            crate::uvars::Kind::Float if kind.eq_ignore_ascii_case("float")=>{
                                let mut value=raw.parse::<f32>().map_err(|_|"camera input source Float literal conversion unbound")?;
                                if !value.is_finite(){return Err("camera input source nonfinite Float unbound".into())}
                                if let Some((lo,hi))=spec.range{value=value.clamp(lo as f32,hi as f32)}value.to_string()
                            },
                            crate::uvars::Kind::Int if kind.eq_ignore_ascii_case("int")=>{
                                let mut value=raw.parse::<i32>().map_err(|_|"camera input source Int literal conversion unbound")?;
                                if let Some((lo,hi))=spec.range{value=value.clamp(lo as i32,hi as i32)}value.to_string()
                            },
                            _=>return Err("camera input source typed variable mismatch".into()),
                        };
                        continue;
                    }
                    if p.left.len() != 2
                        || !doc.text(p.left[0].span).is_some_and(|kind| {
                            kind.eq_ignore_ascii_case("flag") || kind.eq_ignore_ascii_case("bool")
                        })
                    {
                        return Err(
                            "RendererUseRenderLists source requires actual Bool declaration".into(),
                        );
                    }
                    let raw = doc
                        .property_value(p)
                        .ok_or("RendererUseRenderLists source value absent")?
                        .trim();
                    let value = if raw.eq_ignore_ascii_case("true") {
                        true
                    } else if raw.eq_ignore_ascii_case("false") {
                        false
                    } else {
                        return Err("camera render-list source Bool conversion unbound".into());
                    };
                    if let Some(slot) = slot {
                        switches[slot] = value
                    } else {
                        current = Some(value)
                    };
                }
            }
        }
        let current = current.ok_or("source-created RendererUseRenderLists variable is absent")?;
        self.bound = true;
        Ok(Bound {
            use_lists: current,
            switches,
            input_values,
        })
    }
    pub(crate) fn fail(&mut self, error: String) {
        if self.failure.is_none() {
            self.failure = Some(error)
        }
    }
    pub(crate) fn require(&self) -> Result<(), String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.bound {
            return Err("RendererUseRenderLists current source ownership is unbound".into());
        }
        Ok(())
    }
    pub(crate) fn files(&self) -> &[SourceFile] {
        &self.files
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_created_bool_has_no_default_and_uses_same_current_map() {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        for text in [None, Some("false"), Some("true")] {
            let path = std::env::temp_dir().join(format!(
                "aa-camera-source-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            let file = path.join("UVars.tgi");
            std::fs::write(
                &file,
                format!(
                    "[Vars]\n{{\n{}\n}}\n",
                    text.map(|v| format!("flag {NAME} = {v}"))
                        .unwrap_or_default()
                ),
            )
            .unwrap();
            let mut store = crate::tgi::TgiStore::new(DataRoot::new(&path));
            let mut vars = crate::uvars::UVars::load(&mut store);
            if let Some(value) = text {
                let expected = value == "true";
                assert_eq!(vars.renderer_use_render_lists_current().unwrap(), expected);
                assert_eq!(vars.default_bool(NAME), expected);
                vars.set_bool(NAME, !expected);
                assert_eq!(vars.renderer_use_render_lists_current().unwrap(), !expected);
                assert_eq!(vars.default_bool(NAME), expected)
            } else {
                assert!(vars.renderer_use_render_lists_current().is_err());
                assert!(vars.def(NAME).is_none())
            }
            let foreign =
                crate::uvars::UVars::load_with_view_scope(&mut store, SourceOverrides::Unbound);
            assert!(foreign.renderer_use_render_lists_current().is_err());
            std::fs::remove_file(file).unwrap();
            std::fs::remove_dir(path).unwrap();
        }
    }
    #[test]
    fn native_registered_switches_are_same_owner_live_references() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-native-camera/native-camera-renderlist-vars.json"
        ))
        .unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            let vars = row["variables"].as_array().unwrap();
            assert_eq!(vars.len(), 13);
            for (name, var) in RENDER_LIST_NAMES.iter().zip(vars) {
                assert_eq!(var["name"], *name);
                assert_eq!(var["kind"], 1);
                assert_eq!(var["flags"], 9);
                assert_eq!(var["word"].as_u64().unwrap() & 255, 1)
            }
        }
    }
    #[test]
    fn exact_native_source_bool_and_point3_create_setter_policy() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-native-camera/native-camera-render-source-policy.json")).unwrap();
        assert_eq!(fixture["executions"], 24);
        for row in fixture["rows"].as_array().unwrap() {
            let value = &row["output"][1];
            assert_eq!(value["name"], NAME);
            assert_eq!(value["kind"], 1);
            assert_eq!(value["flags"], 0);
            assert_eq!(
                value["word"].as_u64().unwrap() & 255,
                u64::from(row["flag"].as_bool().unwrap())
            )
        }
    }
    #[test]
    #[ignore = "requires actual original/v14 source data; no native engine/GPU"]
    fn actual_camera_uvar_same_ui_source_owner() {
        let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let root = DataRoot::with_overlay(
            game.join("reconstruction/assets/original/Data"),
            game.join("reconstruction/native-game/runtime-expanded-v14/Data"),
        );
        let mut store = crate::tgi::TgiStore::new(root);
        let mut vars = crate::uvars::UVars::load(&mut store);
        assert!(vars.renderer_use_render_lists_current().unwrap());
        assert!(!vars.camera_variable_source_files().is_empty());
        vars.set_bool(NAME, false);
        assert!(!vars.renderer_use_render_lists_current().unwrap());
        assert!(vars.default_bool(NAME));
        for name in RENDER_LIST_NAMES {
            let key = vars.camera_render_list_switch(name).unwrap();
            assert!(key.read_current(&vars).unwrap());
            vars.set_bool(name, false);
            assert!(!key.read_current(&vars).unwrap());
            let foreign = vars.clone();
            assert!(key.read_current(&foreign).is_err());
        }
    }
}

#[cfg(test)]mod input_tests{
 use super::*;
 #[test]fn exact_registered_camera_input_cells_and_current_ui_source(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-native-camera/native-camera-input-vars.json")).unwrap();
  for row in fixture["rows"].as_array().unwrap(){let vars=row["variables"].as_array().unwrap();assert_eq!(vars.len(),8);for v in vars{let spec=INPUT_SPECS.iter().find(|s|s.name==v["name"].as_str().unwrap()).unwrap();let(kind,word)=match spec.kind{crate::uvars::Kind::Bool=>(1,u32::from(spec.default=="true")),crate::uvars::Kind::Int=>(3,spec.default.parse::<i32>().unwrap()as u32),crate::uvars::Kind::Float=>(2,spec.default.parse::<f32>().unwrap().to_bits()),_=>panic!()};assert_eq!(v["kind"],kind);assert_eq!(v["word"],word);}}
  let path=std::env::temp_dir().join(format!("aa-camera-inputs-{}",std::process::id()));std::fs::create_dir_all(&path).unwrap();let file=path.join("UVars.tgi");
  std::fs::write(&file,"[Vars]\n{\nflag RendererUseRenderLists = true\nint MapDefaultWidth = 1\nfloat CameraScaleZoom = 0.75\nflag CameraRotate = true\n}\n").unwrap();
  let mut store=crate::tgi::TgiStore::new(DataRoot::new(&path));let mut vars=crate::uvars::UVars::load(&mut store);
  assert_eq!(vars.camera_input_word(0xa8).unwrap(),16);assert_eq!(vars.camera_input_word(0xac).unwrap(),128);assert_eq!(vars.camera_input_word(0xb4).unwrap(),1);assert_eq!(vars.camera_input_word(0x288).unwrap(),0);assert_eq!(vars.camera_input_word(0x34).unwrap(),0.75f32.to_bits());
  vars.store_camera_scale_zoom_34((-0f32).to_bits()).unwrap();assert_eq!(vars.camera_input_word(0x34).unwrap(),(-0f32).to_bits());assert!(vars.camera_input_word(0).is_err());
  std::fs::remove_file(file).unwrap();std::fs::remove_dir(path).unwrap();
 }
}
