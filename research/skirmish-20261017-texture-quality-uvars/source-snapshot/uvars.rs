//! User variables (`UVars`, manager `0xB39938`, registered in `0x41F9D0`)
//! as far as the options dialog reads and writes them.
//!
//! * Registration `(name, default[, max, min], flags)`: range variables
//!   clamp on write (`0x40E0A0` for floats, inline for ints); writes go
//!   through the guard `0x4B29B0`.
//! * The defaults copy (`0xB39960`) is the code default overlaid by
//!   `Data/UVars.tgi` (INFERRED from the options dialog's Defaults, which
//!   read it). OK saves the user values against it to the AVar
//!   `UserUVarsFilename` (User/UVars.tgi); this port never writes the
//!   original profile: the caller persists [`UVars::overrides`] in its own
//!   settings file instead.
//!
//! Code defaults and ranges below come from the registration calls
//! (research/phase8-ui/scratch-options/uvars.tsv); float range registers
//! the disassembly left symbolic (`ebp` = 1.0, `ebx` = 0) follow the
//! options-dialog research.

use crate::tgi::TgiStore;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bool,
    Int,
    Float,
    Str,
}

/// One registered variable.
#[derive(Clone, Copy, Debug)]
pub struct Def {
    pub name: &'static str,
    pub kind: Kind,
    /// Code default (text).
    pub default: &'static str,
    /// (min, max) for range variables.
    pub range: Option<(f64, f64)>,
}

const fn d(name: &'static str, kind: Kind, default: &'static str, range: Option<(f64, f64)>) -> Def {
    Def { name, kind, default, range }
}

use Kind::*;

/// The variables the options dialog uses.
pub const DEFS: &[Def] = &[
    // message logs
    d("MessageLogMaximumLines", Int, "50", None),
    d("UIMessageLogCycleTimeout", Float, "1", None),
    d("MetaGameUIMessageLogMaximumLines", Int, "10", Some((1.0, 15.0))),
    d("UIMessageFilterHero", Bool, "true", None),
    d("UIMessageFilterCompany", Bool, "true", None),
    d("UIMessageFilterRegiment", Bool, "true", None),
    d("UIMessageFilterBuilding", Bool, "true", None),
    d("UIMessageFilterGeneric", Bool, "true", None),
    d("UIMessageFilterTutorial", Bool, "true", None),
    // HUD (registration 0x41F9D0: slots 0x2C, 0x7D..0x7F, 0x80, 0x82)
    d("CameraInformation", Bool, "false", None),
    d("UIRecruitFloaterIsOpen", Bool, "true", None),
    d("UIGroupFloaterIsOpen", Bool, "true", None),
    d("UIAbilityFloaterIsOpen", Bool, "true", None),
    d("UIColorHighlight", Str, "1,1,0", None),
    d("UIColorOptionValid", Str, "0,1,0", None),
    d("UIColorOptionInvalid", Str, "1,0,0", None),
    // game play
    d("UIMessageLogMaximumLines", Int, "5", Some((1.0, 15.0))),
    d("UIMessageLogFadeTime", Int, "10000", Some((1.0, 20000.0))),
    d("CameraScrollSpeed", Float, "2", Some((1.0, 20.0))),
    d("CameraScrollAcceleration", Float, "0.25", Some((0.0, 20.0))),
    d("CameraScrollSpeedMax", Float, "20", Some((1.0, 30.0))),
    d("CameraScaleZoom", Float, "0.5", None),
    d("CameraRotate", Bool, "false", None),
    d("UIToolTipsShow", Bool, "true", None),
    d("UIMessageBoxConfirmation", Bool, "true", None),
    d("UIStructureAutoPlacement", Bool, "false", None),
    d("ViewShowVFXObjectName", Bool, "true", None),
    d("UIEventFilterSiege", Bool, "true", None),
    d("UIEventFilterEngagement", Bool, "true", None),
    d("UIEventFilterEngagementMessages", Bool, "false", None),
    d("UIEventFilterProduction", Bool, "true", None),
    d("ViewShowDamage", Bool, "false", None),
    d("ViewShowZoneFriendlySupply", Bool, "false", None),
    d("ViewShowElapsedGameTime", Bool, "false", None),
    d("HotkeyConfiguration", Str, "classic_hotkey_config", None),
    // video
    d("ResolutionX", Int, "800", None),
    d("ResolutionY", Int, "600", None),
    d("RenderDrawFootsteps", Bool, "true", None),
    d("TerrainExtraQuality", Bool, "true", None),
    d("TerrainQuadSize", Int, "2", Some((1.0, 2.0))),
    d("TerrainTrilinearFilter", Bool, "true", None),
    d("RenderLODShadows", Int, "1", Some((0.0, 3.0))),
    d("ParticleGeneratePCG", Float, "1", Some((0.0, 1.0))),
    d("UnitFogSimplified", Bool, "false", None),
    d("RendererUseHighQualityTextures", Bool, "true", None),
    d("PerformanceDisableDecals", Bool, "false", None),
    d("RendererPreRequestAllVThings", Bool, "true", None),
    // audio
    d("AudioMainVolume", Float, "1", Some((0.0, 1.0))),
    d("Audio2DVolume", Float, "1", Some((0.0, 1.0))),
    d("Audio3DVolume", Float, "1", Some((0.0, 1.0))),
    d("AudioSpeechVolume", Float, "1", Some((0.0, 1.0))),
    d("AudioMusicVolume", Float, "1", Some((0.0, 1.0))),
    d("UnitAcknowledgement", Bool, "true", None),
    d("UIEventFilterChatSound", Bool, "true", None),
    // network
    d("NetworkReportGameInProgress", Bool, "true", None),
    d("AFKTime", Int, "20", None),
    d("UIMessageFilterBuddy", Bool, "true", None),
    d("UIMessageFilterChat", Bool, "true", None),
    d("UIMessageFilterAFK", Bool, "true", None),
    d("UIMessageFilterPlayerJoinLeave", Bool, "true", None),
    d("NetworkFilterSameVersion", Bool, "true", None),
    d("NetworkFilterSameGroupRoom", Bool, "false", None),
    d("NetworkFilterFullGames", Bool, "true", None),
    d("NetworkFilterPlayingGames", Bool, "false", None),
    d("NetworkFilterSameLanguage", Bool, "false", None),
    d("NetworkAutoRefresh", Bool, "false", None),
    d("NetworkRefreshTime", Int, "30", None),
    // metagame
    d("UIMessageFilterMetaGameGeneric", Bool, "true", None),
    d("UIMessageFilterMetaGameMovement", Bool, "false", None),
    d("UIMessageFilterMetaGamePurchases", Bool, "true", None),
    d("UIMessageFilterMetaGameBattles", Bool, "true", None),
    d("MetaGameViewShowVFXMovement", Bool, "false", None),
    d("MetaGameViewShowVFXObjectName", Bool, "false", None),
    d("MetaGameViewShowVFXArrows", Bool, "true", None),
    d("MetaGameUIShowHelpPopup", Bool, "true", None),
    d("MetaGameUIFastMGAI", Bool, "true", None),
    d("MetaGameUIComputerQuickBattle", Bool, "false", None),
    d("MetaGameUIPurchaseConfirm", Bool, "true", None),
];

/// Parse a TGI number (`80%` = 0.8).
pub fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim().trim_end_matches('f');
    match t.strip_suffix('%') {
        Some(p) => p.trim().parse::<f64>().ok().map(|v| v / 100.0),
        None => t.parse::<f64>().ok(),
    }
}

#[derive(Debug, Default)]
pub struct UVars {
    defs: HashMap<String, Def>,
    /// Defaults copy (`0xB39960`), by lower-case name.
    defaults: HashMap<String, String>,
    /// User values (`0xB39938`).
    values: HashMap<String, String>,
    ///Sole byte owners for the eight native43F156 reset targets and the
    ///45B530 RendererPreRequestAllVThings getter. They never
    ///also appear in values/defaults string maps. Clones are UI option drafts.
    view: crate::view_variables::State,
    message_source:crate::message_variables::Receipt,
    camera_source:crate::camera_source::Receipt,
    identity:std::sync::Arc<()>,
}
///Actual712660 variable reference: resolves the same current UI owner lazily.
///No Bool snapshot/default/foreign-owner fallback is stored here.
pub struct CameraRenderListSwitch{owner:std::sync::Weak<()>,name:&'static str}
impl CameraRenderListSwitch{
 pub fn name(&self)->&'static str{self.name}
 pub fn read_current(&self,vars:&UVars)->Result<bool,String>{
  if !self.owner.upgrade().is_some_and(|owner|std::sync::Arc::ptr_eq(&owner,&vars.identity)){return Err("camera render-list variable belongs to another current UI owner".into())}
  vars.render_list_bool_current(self.name)
 }
}
impl Clone for UVars{
    fn clone(&self)->Self{Self{defs:self.defs.clone(),defaults:self.defaults.clone(),values:self.values.clone(),
        view:self.view.clone(),message_source:self.message_source.clone(),camera_source:self.camera_source.clone(),identity:std::sync::Arc::new(())}}
}

impl UVars {
    /// Code defaults overlaid by `Data/UVars.tgi`.
    pub fn load(store: &mut TgiStore) -> Self {
        Self::load_with_view_scope(store,crate::view_variables::SourceOverrides::ControlledNone)
    }

    ///Fresh UI source loading. Callers with unknown external profile/console
    ///ownership must pass Unbound; strict VWorld initialization then refuses.
    pub fn load_with_view_scope(store:&mut TgiStore,scope:crate::view_variables::SourceOverrides)->Self {
        let mut v = UVars::default();
        for def in DEFS {
            v.defs.insert(def.name.to_ascii_lowercase(), *def);
            if !crate::view_variables::State::contains(def.name) {
                v.defaults.insert(def.name.to_ascii_lowercase(), def.default.to_string());
            }
        }
        for &index in &crate::view_variables::OWNED_BOOL_INDICES {
            let name=crate::view_variables::registrations()[index as usize].name;
            v.defs.insert(name.to_ascii_lowercase(),d(name,Bool,if matches!(index,108|113|61){"true"}else{"false"},None));
        }
        for (name,default)in crate::view_variables::TEXTURE_QUALITY_NAMES.into_iter().zip(["false","true"]){
            v.defs.insert(name.to_ascii_lowercase(),d(name,Bool,default,None));
        }
        if let Ok(node) = store.node("UVars.tgi:Vars") {
            for f in &node.fields {
                if let Some(raw) = &f.raw {
                    let key = f.name.to_ascii_lowercase();
                    if v.defs.contains_key(&key) && !crate::view_variables::State::contains(&key) {
                        v.defaults.insert(key, crate::node::unquote(raw).trim().to_string());
                    }
                }
            }
        }
        if let Err(error)=v.view.bind_fresh_source(&store.root,scope) {v.view.record_failure(error)}
        if v.view.raw(crate::view_variables::RESOURCE_NO_NI_TEXTURES).is_some() {
            //Metadata only: the sole current/default bytes remain in view.
            let name=crate::view_variables::RESOURCE_NO_NI_TEXTURES;
            v.defs.insert(name.to_ascii_lowercase(),d(name,Bool,"",None));
        }
        match v.message_source.bind(&store.root,scope){
            Ok(source)=>{
                v.defaults.insert("messagelogmaximumlines".into(),source.maximum.to_string());
                v.defaults.insert("uimessagefiltergeneric".into(),if source.filter==0{"false"}else{"true"}.into());
            },
            Err(error)=>v.message_source.fail(error),
        }
        match v.camera_source.bind(&store.root,scope){
            Ok(value)=>{
                //Metadata has no synthetic constructor value. Only the real
                //4B3C10 source-created Bool supplies current/default storage.
                v.defs.insert(crate::camera_source::NAME.to_ascii_lowercase(),d(crate::camera_source::NAME,Bool,"",None));
                v.defaults.insert(crate::camera_source::NAME.to_ascii_lowercase(),if value.use_lists{"true"}else{"false"}.into());
                for (spec,current) in crate::camera_source::INPUT_SPECS.iter().zip(value.input_values){
                    v.defs.insert(spec.name.to_ascii_lowercase(),*spec);
                    v.defaults.insert(spec.name.to_ascii_lowercase(),current);
                }
                for (name,current) in crate::camera_source::RENDER_LIST_NAMES.into_iter().zip(value.switches){
                    v.defs.insert(name.to_ascii_lowercase(),d(name,Bool,"true",None));
                    v.defaults.insert(name.to_ascii_lowercase(),if current{"true"}else{"false"}.into());
                }
            },
            Err(error)=>v.camera_source.fail(error),
        }
        v.values = v.defaults.clone();
        v
    }

    fn raw(&self, name: &str) -> Option<&str> {
        self.values.get(&name.to_ascii_lowercase()).map(String::as_str)
    }

    fn raw_default(&self, name: &str) -> Option<&str> {
        self.defaults.get(&name.to_ascii_lowercase()).map(String::as_str)
    }

    pub fn def(&self, name: &str) -> Option<Def> {
        self.defs.get(&name.to_ascii_lowercase()).copied()
    }
    pub(crate) fn message_owner(&self)->std::sync::Weak<()>{std::sync::Arc::downgrade(&self.identity)}
    pub(crate) fn require_message_source(&self)->Result<(),String>{self.message_source.require()}
    pub fn message_variable_sources(&self)->&[crate::view_variables::SourceFile]{self.message_source.files()}
    ///Current same-owner UVar78; no get_bool missing/invalid=false fallback.
    pub fn message_filter_generic_current(&self)->Result<u8,String>{
        self.message_source.require()?;
        if !self.def("UIMessageFilterGeneric").is_some_and(|d|d.kind==Kind::Bool){return Err("native UVar78 Bool definition absent".into())}
        let raw=self.raw("UIMessageFilterGeneric").ok_or("native UVar78 current value absent")?.trim();
        if raw.eq_ignore_ascii_case("true")||raw=="1"{Ok(1)}else if raw.eq_ignore_ascii_case("false")||raw=="0"{Ok(0)}
        else{Err("native UVar78 noncanonical current Bool producer required".into())}
    }
    ///5C891C's same retained source-created Bool, after native source lookup.
    ///It is never the generic missing-name false fallback or a second option
    ///copy. Normal Bool writes mutate these same values/defaults maps.
    pub fn renderer_use_render_lists_current(&self)->Result<bool,String>{
        self.camera_source.require()?;
        if !self.def(crate::camera_source::NAME).is_some_and(|d|d.kind==Kind::Bool){return Err("RendererUseRenderLists current Bool definition absent".into())}
        let raw=self.raw(crate::camera_source::NAME).ok_or("RendererUseRenderLists current source-created value absent")?.trim();
        if raw.eq_ignore_ascii_case("true")||raw=="1"{Ok(true)}else if raw.eq_ignore_ascii_case("false")||raw=="0"{Ok(false)}else{Err("RendererUseRenderLists current Bool conversion is unbound".into())}
    }
    ///4B2C30(name,1) in712660, retaining the actual current manager identity.
    pub fn camera_render_list_switch(&self,name:&str)->Result<CameraRenderListSwitch,String>{
        let name=crate::camera_source::RENDER_LIST_NAMES.into_iter().find(|n|n.eq_ignore_ascii_case(name)).ok_or("native camera render-list Bool name unbound")?;
        self.camera_source.require()?;
        if !self.def(name).is_some_and(|d|d.kind==Kind::Bool)||self.raw(name).is_none(){return Err("camera render-list current Bool reference absent".into())}
        Ok(CameraRenderListSwitch{owner:std::sync::Arc::downgrade(&self.identity),name})
    }
    fn render_list_bool_current(&self,name:&str)->Result<bool,String>{
        self.camera_source.require()?;
        if !self.def(name).is_some_and(|d|d.kind==Kind::Bool){return Err("camera render-list current Bool definition absent".into())}
        let raw=self.raw(name).ok_or("camera render-list current variable absent")?.trim();
        if raw.eq_ignore_ascii_case("true")||raw=="1"{Ok(true)}else if raw.eq_ignore_ascii_case("false")||raw=="0"{Ok(false)}else{Err("camera render-list current Bool conversion unbound".into())}
    }
    ///Strict current camera UVar fast table: exactly the eight proved native
    ///registrations, actual source/current Maps, no missing-value fallback.
    pub fn camera_input_word(&self,offset:u32)->Result<u32,String>{
        self.camera_source.require()?;
        let name=match offset{0x18=>"CameraScrollSpeed",0x24=>"CameraScrollEdgePadding",0x28=>"CameraScalePan",0x34=>"CameraScaleZoom",0xa8=>"MapDefaultWidth",0xac=>"MapDefaultHeight",0xb4=>"CameraRotate",0x288=>"EditorCameraUnlocked",_=>return Err("unbound native camera UVar fast-table offset".into())};
        let spec=crate::camera_source::INPUT_SPECS.iter().find(|s|s.name==name).unwrap();
        if !self.def(name).is_some_and(|d|d.kind==spec.kind){return Err("camera current UVar type absent/different".into())}
        let raw=self.raw(name).ok_or("camera current UVar value absent")?.trim();
        match spec.kind{
            Kind::Bool=>if raw.eq_ignore_ascii_case("true")||raw=="1"{Ok(1)}else if raw.eq_ignore_ascii_case("false")||raw=="0"{Ok(0)}else{Err("camera current Bool conversion unbound".into())},
            Kind::Int=>raw.parse::<i32>().map(|v|v as u32).map_err(|_|"camera current Int conversion unbound".into()),
            Kind::Float=>{let v=raw.parse::<f32>().map_err(|_|"camera current Float conversion unbound")?;if !v.is_finite(){return Err("camera current Float is nonfinite".into())}Ok(v.to_bits())},
            _=>Err("camera current variable type unbound".into()),
        }
    }
    ///Native7005F0->40E0A0 reaches actual UVar34, flags10 (writable/no range).
    pub fn store_camera_scale_zoom_34(&mut self,value:u32)->Result<(),String>{
        self.camera_input_word(0x34)?;
        let value=f32::from_bits(value);if !value.is_finite(){return Err("camera scale zoom nonfinite native setter domain unbound".into())}
        self.set_f32("CameraScaleZoom",value);Ok(())
    }
    pub fn camera_variable_source_files(&self)->&[crate::view_variables::SourceFile]{self.camera_source.files()}
    ///Current same-owner UVar67, read at40B733 after diagnostics/sound.
    pub fn message_maximum_lines_current(&self)->Result<i32,String>{
        self.message_source.require()?;
        if !self.def("MessageLogMaximumLines").is_some_and(|d|d.kind==Kind::Int){return Err("native UVar67 Int definition absent".into())}
        self.raw("MessageLogMaximumLines").ok_or("native UVar67 current value absent")?.trim().parse()
            .map_err(|_|"native UVar67 current integer conversion required".into())
    }

    fn to_bool(s: &str) -> bool {
        s.trim().eq_ignore_ascii_case("true") || s.trim() == "1"
    }

    pub fn get_bool(&self, name: &str) -> bool {
        if let Some(value)=self.view.raw(name){return value!=0}
        self.raw(name).map(Self::to_bool).unwrap_or(false)
    }
    pub fn get_f32(&self, name: &str) -> f32 {
        if let Some(value)=self.view.raw(name){return value as f32}
        self.raw(name).and_then(parse_number).unwrap_or(0.0) as f32
    }
    pub fn get_i32(&self, name: &str) -> i32 {
        if let Some(value)=self.view.raw(name){return value as i32}
        self.raw(name).and_then(parse_number).unwrap_or(0.0) as i32
    }
    /// A `point3` variable (r,g,b); missing components are 0.
    pub fn get_point3(&self, name: &str) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for (i, part) in self.raw(name).unwrap_or("").split(',').take(3).enumerate() {
            out[i] = parse_number(part.trim()).unwrap_or(0.0) as f32;
        }
        out
    }
    pub fn get_str(&self, name: &str) -> String {
        if let Some(value)=self.view.raw(name){return if value!=0{"true"}else{"false"}.into()}
        self.raw(name).unwrap_or("").to_string()
    }
    pub fn default_bool(&self, name: &str) -> bool {
        if let Some(value)=self.view.default_raw(name){return value!=0}
        self.raw_default(name).map(Self::to_bool).unwrap_or(false)
    }
    pub fn default_f32(&self, name: &str) -> f32 {
        if let Some(value)=self.view.default_raw(name){return value as f32}
        self.raw_default(name).and_then(parse_number).unwrap_or(0.0) as f32
    }
    pub fn default_i32(&self, name: &str) -> i32 {
        if let Some(value)=self.view.default_raw(name){return value as i32}
        self.raw_default(name).and_then(parse_number).unwrap_or(0.0) as i32
    }
    pub fn default_str(&self, name: &str) -> String {
        if let Some(value)=self.view.default_raw(name){return if value!=0{"true"}else{"false"}.into()}
        self.raw_default(name).unwrap_or("").to_string()
    }

    /// Registered range (min, max).
    pub fn range(&self, name: &str) -> Option<(f32, f32)> {
        self.def(name).and_then(|d| d.range).map(|(a, b)| (a as f32, b as f32))
    }

    pub fn set_bool(&mut self, name: &str, v: bool) {
        if crate::view_variables::State::contains(name) {
            if let Err(error)=self.view.assign(name,u8::from(v),None){self.view.record_failure(error)}
            return
        }
        self.values.insert(name.to_ascii_lowercase(), if v { "true" } else { "false" }.into());
    }
    /// Float write with the range clamp (`0x40E0A0`).
    pub fn set_f32(&mut self, name: &str, v: f32) {
        if crate::view_variables::State::contains(name) {
            self.view.record_failure("view Bool float conversion requires its actual typed setter");return
        }
        let v = match self.range(name) {
            Some((lo, hi)) => v.clamp(lo, hi),
            None => v,
        };
        self.values.insert(name.to_ascii_lowercase(), format!("{v}"));
    }
    /// Int write with the range clamp.
    pub fn set_i32(&mut self, name: &str, v: i32) {
        if crate::view_variables::State::contains(name) {
            self.view.record_failure("view Bool integer conversion requires its actual typed setter");return
        }
        let v = match self.range(name) {
            Some((lo, hi)) => v.clamp(lo as i32, hi as i32),
            None => v,
        };
        self.values.insert(name.to_ascii_lowercase(), format!("{v}"));
    }
    pub fn set_str(&mut self, name: &str, v: &str) {
        if crate::view_variables::State::contains(name) {
            self.view.record_failure("view Bool string conversion requires its actual typed setter");return
        }
        self.values.insert(name.to_ascii_lowercase(), v.to_string());
    }

    /// Values that differ from the defaults copy, by registered name.
    pub fn overrides(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for def in DEFS {
            if let Some(value)=self.view.raw(def.name) {
                if Some(value)!=self.view.default_raw(def.name) {
                    out.insert(def.name.to_owned(),match value {0=>"false".into(),1=>"true".into(),_=>value.to_string()});
                }
                continue
            }
            let k = def.name.to_ascii_lowercase();
            let (v, d) = (self.values.get(&k), self.defaults.get(&k));
            if v.is_some() && v != d {
                let same_number = match (v.and_then(|x| parse_number(x)), d.and_then(|x| parse_number(x))) {
                    (Some(a), Some(b)) => (a - b).abs() < 1e-9,
                    _ => false,
                };
                if !same_number {
                    out.insert(def.name.to_string(), v.cloned().unwrap_or_default());
                }
            }
        }
        for name in [crate::view_variables::RESOURCE_NO_NI_TEXTURES,"DataCompatibilityMode",crate::view_variables::TEXTURE_QUALITY_NAMES[0]] {
            if let Some(value)=self.view.raw(name) {
                if Some(value)!=self.view.default_raw(name) {
                    out.insert(name.into(),match value {0=>"false".into(),1=>"true".into(),_=>value.to_string()});
                }
            }
        }
        out
    }

    /// Load saved user values (unknown names are ignored).
    pub fn apply_overrides(&mut self, map: &BTreeMap<String, String>) {
        for (k, v) in map {
            let key = k.to_ascii_lowercase();
            if crate::view_variables::State::contains(&key) {
                let value=if v.eq_ignore_ascii_case("true"){Some(1)}else if v.eq_ignore_ascii_case("false"){Some(0)}else{v.parse::<u8>().ok()};
                match value {
                    Some(value)=>if let Err(error)=self.view.assign(&key,value,None){self.view.record_failure(error)},
                    None=>self.view.record_failure("private view Bool override requires a raw byte or true/false"),
                }
                continue
            }
            if self.defs.contains_key(&key) {
                self.values.insert(key, v.clone());
            }
        }
    }

    ///Same canonical Bool allocation used by HUD/options. A positive editor
    ///write needs the actual native EDITORDEV gate, never a guessed permission.
    pub fn assign_view_bool(&mut self,name:&str,value:u8,editor_dev:Option<bool>)->Result<(),String> {
        self.view.assign(name,value,editor_dev)
    }
    pub fn view_bool_raw(&self,name:&str)->Option<u8> {self.view.raw(name)}
    ///Native45B530's first lazy UVar108 read, from the same source/profile
    ///owner used by UI writes. Unknown source ownership is an error.
    pub fn renderer_pre_request_all_vthings(&self)->Result<u8,String>{
        if let Some(error)=self.view.failure(){return Err(error.to_owned())}
        self.view.require_current()?;
        self.view.raw("RendererPreRequestAllVThings").ok_or_else(||"current UVar108 allocation absent".into())
    }
    ///Native73A560 reads the actual current flags0 Bool byte at index113.
    ///It is owned once here and is not one of43F0A0's reset targets.
    pub fn renderer_use_hw_skinning_shader(&self)->Result<u8,String>{
        self.view.require_current()?;
        self.view.raw("RendererUseHWSkinningShader").ok_or_else(||"current UVar113 allocation absent".into())
    }
    ///40EAA0 reads the SAME source-created/current flags2 Bool at index62.
    ///It is not reset by43F0A0 and cannot use the generic missing=false path.
    pub fn dev_mode_current(&self)->Result<u8,String>{
        self.view.require_current()?;
        self.view.raw("DevMode").ok_or_else(||"current UVar62 allocation absent".into())
    }
    ///Native5A8D5F named lookup(kind1) and5A8D77 raw byte read. The caller
    ///must preserve the native exact==1 test; nonzero is not equivalent.
    pub fn resource_no_ni_textures_current(&self)->Result<u8,String>{
        self.view.require_current()?;
        self.view.raw(crate::view_variables::RESOURCE_NO_NI_TEXTURES)
            .ok_or_else(||"ResourceNoNiTextures current source-created Bool allocation absent".into())
    }
    ///Native5A7976 reads SAME indexed UVar61 before its MissingResource
    ///catch/soft-request branch. Zero/nonzero semantics belong to that caller.
    pub fn data_compatibility_mode_current(&self)->Result<u8,String>{
        self.view.require_current()?;
        self.view.raw("DataCompatibilityMode")
            .ok_or_else(||"current indexed UVar61 DataCompatibilityMode allocation absent".into())
    }
    fn texture_quality_current(&self,name:&'static str)->Result<u8,String>{
        self.view.require_current()?;
        self.view.raw(name).ok_or_else(||format!("current named Texture quality Bool {name} allocation absent"))
    }
    pub fn renderer_force_low_quality_textures_current(&self)->Result<u8,String>{
        self.texture_quality_current(crate::view_variables::TEXTURE_QUALITY_NAMES[0])
    }
    pub fn renderer_use_high_quality_textures_current(&self)->Result<u8,String>{
        self.texture_quality_current(crate::view_variables::TEXTURE_QUALITY_NAMES[1])
    }
    ///The real5A8B40 lazy getter branch over this SAME Ui owner. True means
    ///invoke NiSourceTexture virtual3C(1); it is not a decoded/GPU-ready bit.
    pub fn texture_quality_source_mode(&self)->Result<bool,String>{
        crate::view_variables::texture_quality_source_mode(|name|self.texture_quality_current(name))
    }
    ///43FCF1..43FD35 reads UVar0 independently of the AVar grid size.
    ///Borrow its same current UI value; missing/unsupported values must not
    ///become the generic options getter's zero fallback.
    pub fn terrain_quad_size_current(&self)->Result<i32,String>{
        self.view.require_current()?;
        let raw=self.raw("TerrainQuadSize").ok_or("current UVar0 TerrainQuadSize allocation absent")?;
        raw.trim().parse::<i32>().map_err(|_|"current UVar0 requires its native integer value".into())
    }
    pub fn view_source_files(&self)->&[crate::view_variables::SourceFile] {self.view.files()}
    pub fn view_source_failure(&self)->Option<&str> {self.view.failure()}
    ///Call real retained Session+F8 at each reached native getter. This is only
    ///the reset child of43F0A0; it never writes AD or finishes loading.
    pub fn reset_view_for_loaded_map(&mut self,session_f8:impl FnMut()->Result<u32,String>)
        ->Result<crate::view_variables::ResetTrace,crate::view_variables::ResetFailure> {
        self.view.reset(session_f8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_numbers() {
        assert_eq!(parse_number("80%"), Some(0.8));
        assert_eq!(parse_number("0.5"), Some(0.5));
        assert_eq!(parse_number(" 10000 "), Some(10000.0));
    }

    #[test]
    fn view_cells_never_have_parallel_string_entries() {
        let mut store=TgiStore::new(crate::fs::DataRoot::workspace_default());
        let mut values=UVars::load(&mut store);
        for &index in &crate::view_variables::OWNED_BOOL_INDICES {
            let name=crate::view_variables::registrations()[index as usize].name.to_ascii_lowercase();
            assert!(!values.values.contains_key(&name));assert!(!values.defaults.contains_key(&name));
        }
        values.set_bool("CameraInformation",true);
        assert!(values.get_bool("CameraInformation"));assert!(!values.values.contains_key("camerainformation"));
        let named=crate::view_variables::RESOURCE_NO_NI_TEXTURES.to_ascii_lowercase();
        assert!(!values.values.contains_key(&named));assert!(!values.defaults.contains_key(&named));
        for name in crate::view_variables::TEXTURE_QUALITY_NAMES{
            let key=name.to_ascii_lowercase();assert!(!values.values.contains_key(&key));assert!(!values.defaults.contains_key(&key));
        }
    }
}
