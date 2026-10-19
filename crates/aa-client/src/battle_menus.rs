//!Actual TGI interface allocations for the modern menu presentation backend.
//!Native AppState selectors retain weak handles; Widget forests stay on the UI
//!thread and move between this owner and the SAME Ui root. Legacy interface
//!animation-manager ABI is deliberately replaced by immediate presentation.
use aa_ui::{
    ui::Ui,
    widget::{Widget, WidgetClass},
};
use std::sync::{Arc, Weak};
#[path="battle_game_ui.rs"]
mod game_ui;
#[path="battle_game_hud.rs"]
pub mod game_hud;
#[path="battle_hud_orders.rs"]
pub mod hud_orders;
pub use game_ui::Reference as GameWorldReference;
type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Splash,
    Main,
    Loading,
    Game,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore="requires installed original/v14 TGI source fixtures"]
    fn actual_four_menu_allocations_and_ready_widget_stay_in_same_ui(){
        let install=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap();
        let base=install.join("reconstruction");
        let data=aa_ui::fs::DataRoot::with_overlay(base.join("assets/original/Data"),base.join("native-game/runtime-expanded-v14/Data"));
        let mut ui=Ui::new(data,aa_ui::layout::Screen{width:1024,height:768});
        let mut sources=aa_rules::sources::V14Sources::open(install.join("Data.rwd"),base.join("native-game/runtime-expanded-v14"),
            aa_rules::sources::SourceLimits{max_member_bytes:u64::MAX}).unwrap();
        //Declaration order stands in for registry order in this tree-only test.
        let ids=aa_rules::resources::ResourceCatalog::load_v14(&mut sources).unwrap().resources.into_iter().map(|r|r.id).collect::<Vec<_>>();
        let mut owner=State::new();let menus=owner.bind(&mut ui,&ids).unwrap();
        owner.show(&mut ui,&menus.loading).unwrap();assert!(!owner.ready_button_visible(&ui,&menus.loading).unwrap());
        owner.show_ready_button(&mut ui,&menus.loading).unwrap();assert!(owner.ready_button_visible(&ui,&menus.loading).unwrap());
        owner.show(&mut ui,&menus.game).unwrap();assert!(ui.widget_by("Game").is_some());
        assert!(owner.game_active());owner.initialized_callback_e8(&menus.game).unwrap();
        for id in &ids{assert!(ui.widget_by(&format!("Game/EconomyBar/{id}/ResourceLabel")).is_some(),"{id}");}
        assert!(ui.widget_by("Game/ControlPanel/ActionButtons").is_some());
        assert!(ui.widget_by("Game/ControlPanel/GameMiniMap").is_some());
        assert!(owner.ready_button_visible(&ui,&menus.loading).is_err());
        owner.show(&mut ui,&menus.loading).unwrap();assert!(owner.ready_button_visible(&ui,&menus.loading).unwrap());
        owner.show(&mut ui,&menus.main).unwrap();owner.show(&mut ui,&menus.game).unwrap();
        //612010 + children on the production Game forest with laboratory
        //world reads: TimeMessage detaches (UVar false), the shortage icon
        //follows Kingdom+1F0, the economy bar shows the registry values.
        let economy=aa_ui::hud::HudEconomy{production:vec![60.0;ids.len()],preview_production:vec![60.0;ids.len()],
            upkeep:vec![0.0;ids.len()],stock:vec![935.0;ids.len()],limited_max:100.0};
        let mut sources=aa_rules::sources::V14Sources::open(install.join("Data.rwd"),base.join("native-game/runtime-expanded-v14"),
            aa_rules::sources::SourceLimits{max_member_bytes:u64::MAX}).unwrap();
        let rc=aa_rules::resources::ResourceCatalog::load_v14(&mut sources).unwrap();
        owner.bind_game_resources(rc.resources.iter().map(|r|aa_ui::hud::HudResource{ids:r.id.clone(),kind:r.category as u8,icon:String::new(),
            limited_by:None,limit_provider:r.limited_provider,purchase_cost:vec![0.0;ids.len()]}).collect()).unwrap();
        assert!(ui.widget_by("Game/TimeMessage").is_some());
        let frame=game_hud::LiveFrame::laboratory(Some(0.5f32.to_bits()),Some(economy),0,3725.0);
        owner.game_update(&mut ui,&menus.game,&frame).unwrap();
        assert!(ui.widget_by("Game/TimeMessage").is_none(),"612010 detaches TimeMessage while ViewShowElapsedGameTime is false");
        assert_eq!(ui.widget_by("Game/UpkeepShortageIcon").unwrap().flags&1,1);
        let label=|ui:&Ui,p:&str|{
            let w=ui.widget_by(p).unwrap_or_else(||panic!("{p} absent"));
            w.sub_ve.iter().chain(w.view.iter()).find_map(|v|match &v.kind{aa_ui::ve::VeKind::Label(l)=>Some(l.text.clone()),_=>None})
                .unwrap_or_else(||panic!("{p} has no label"))
        };
        let first=format!("Game/EconomyBar/{}/ResourceLabel",ids[0]);
        assert!(ui.widget_by(&first).is_some(),"{:?}",ui.widget_by("Game/EconomyBar").map(|w|w.children.iter().map(|c|c.name.clone()).collect::<Vec<_>>()));
        assert!(label(&ui,&first).contains("935"),"{}",label(&ui,&first));
        let frame=game_hud::LiveFrame::laboratory(Some(0),None,1,3725.0);
        owner.game_update(&mut ui,&menus.game,&frame).unwrap();
        assert_eq!(ui.widget_by("Game/UpkeepShortageIcon").unwrap().flags&1,0);
        ui.uvars.apply_overrides(&[("ViewShowElapsedGameTime".to_string(),"true".to_string())].into_iter().collect());
        assert!(ui.uvars.get_bool("ViewShowElapsedGameTime"));
        owner.game_update(&mut ui,&menus.game,&frame).unwrap();
        assert!(ui.widget_by("Game/TimeMessage").is_some(),"{:?}",ui.widget_by("Game").map(|w|w.children.iter().map(|c|c.name.clone()).collect::<Vec<_>>()));
        assert_eq!(label(&ui,"Game/TimeMessage"),"1:02:05");
        //LoadingScreen OnConnected 6E8730 on the production Loading forest.
        owner.show_connected(&mut ui,&menus.loading,Some(&game_hud::LoadingFrame::laboratory(Some("USA")))).unwrap();
        assert_eq!(label(&ui,"LoadingScreen/MapNameLabel"),"(2v2) Berlin");
        assert_eq!(label(&ui,"LoadingScreen/StatusLabel"),"Loading Resources");
        assert_eq!(ui.widget_by("LoadingScreen/ReadyButton").unwrap().flags&1,0);
        owner.show(&mut ui,&menus.game).unwrap();
        //Optional evidence: the SAME production Game forest after Show at the
        //GUI run's window size, written as an aa-ui draw list.
        if let Some(dir)=std::env::var_os("AA_GAME_INTERFACE_DUMP"){
            let dir=std::path::PathBuf::from(dir);std::fs::create_dir_all(&dir).unwrap();
            ui.set_screen(aa_ui::layout::Screen{width:1440,height:900});ui.layout();
            let list=ui.draw_list();let mut rects=Vec::new();
            ui.root.walk("",&mut|p,w|rects.push(serde_json::json!({"widget":p,"class":format!("{:?}",w.class),"rect":w.root.outer.rect,"shown":w.flags&2!=0,"loaded":w.loaded})));
            std::fs::write(dir.join("game_interface_1440x900.json"),serde_json::to_string_pretty(&list).unwrap()).unwrap();
            std::fs::write(dir.join("game_interface_1440x900_widgets.json"),serde_json::to_string_pretty(&serde_json::json!({"widgets":rects,"diagnostics":ui.root.all_diagnostics()})).unwrap()).unwrap();
        }
    }
}
#[derive(Clone)]
pub struct Handle {
    origin: Weak<()>,
    kind: Kind,
}
#[derive(Clone)]
pub struct Selectors {
    pub splash: Handle,
    pub main: Handle,
    pub loading: Handle,
    pub game: Handle,
}
struct Slot {
    spec: &'static str,
    forest: Option<Vec<Widget>>,
}
pub struct State {
    origin: Arc<()>,
    slots: [Slot; 4],
    active: Option<Kind>,
    bound: bool,
    game_world_10c: Option<game_ui::Owner>,
    ///Game interface (Application+BC = B3974C) code-built children from its
    ///Init6182B0 and their OnConnected placements. Lives with the forest.
    game_hud: Option<aa_ui::hud::Hud>,
    ///The same Game interface's 612010 fields (+0x19C..+0x1B0).
    game_runtime: Option<game_hud::GameRuntime>,
    ///B36548 +0x190 count written by LoadingScreen OnConnected 745580.
    loading_hold_745580: u32,
}
impl Kind {
    fn index(self) -> usize {
        match self {
            Self::Splash => 0,
            Self::Main => 1,
            Self::Loading => 2,
            Self::Game => 3,
        }
    }
}
impl State {
    pub fn new() -> Self {
        Self {
            origin: Arc::new(()),
            slots: [
                Slot {
                    spec: "UI/Loading/splash_screen_interface.tgi:Loading",
                    forest: None,
                },
                Slot {
                    spec: "UI/Menus/main.tgi:MainMenuInterface",
                    forest: None,
                },
                Slot {
                    spec: "UI/Loading/loading_screen.tgi:LoadingScreen",
                    forest: None,
                },
                Slot {
                    spec: "UI/Game/game_interface.tgi:Game",
                    forest: None,
                },
            ],
            active: None,
            bound: false,
            game_world_10c: None,
            game_hud: None,
            game_runtime: None,
            loading_hold_745580: 0,
        }
    }
    pub fn is_bound(&self) -> bool {
        self.bound
    }
    fn handle(&self, kind: Kind) -> Handle {
        Handle {
            origin: Arc::downgrade(&self.origin),
            kind,
        }
    }
    ///The frontend prelude is retired before these actual application menus
    ///are built once. Each source must resolve; no null menu stand-in exists.
    ///`resource_ids` is the actual native Resource registry in registry order
    ///([B36530]+0x34): the Game interface Init6182B0 builds one EconomyBar
    ///child per entry (603C40 -> 603B70).
    pub fn bind(&mut self, ui: &mut Ui, resource_ids: &[String]) -> Result<Selectors> {
        if !self.bound {
            if self.slots.iter().any(|s| s.forest.is_some()) {
                return Err("menu construction previously stopped; retained partial interfaces cannot be rebuilt".into());
            }
            for kind in [Kind::Main, Kind::Loading, Kind::Splash,Kind::Game] {
                let spec = self.slots[kind.index()].spec;
                ui.store
                    .node(spec)
                    .map_err(|e| format!("actual menu source {spec}: {e}"))?;
                match kind {
                    Kind::Main => aa_ui::screens::main_menu(ui),
                    Kind::Loading => aa_ui::screens::loading_screen(ui, false),
                    Kind::Splash => {
                        ui.clear();
                        let mut widget = Widget::new(WidgetClass::Interface, spec, false);
                        ui.load_widget(&mut widget, None);
                        ui.connect(widget);
                        ui.layout();
                    }
                    Kind::Game => {
                        //KKC_Application::Init 41AA81 runs the Game interface
                        //Init6182B0 once, right after 61A650 allocated it: the
                        //code-built HUD tree, loaded but not yet connected.
                        //Its per-frame world reads are a separate binding;
                        //no PlaceholderWorld is read.
                        ui.clear();
                        let (hud, widget) = aa_ui::hud::Hud::construct(ui, resource_ids);
                        self.game_hud = Some(hud);
                        self.game_runtime = Some(game_hud::GameRuntime::constructed(ui.now_ms));
                        ui.connect(widget);
                        ui.layout();
                    }
                }
                let forest = std::mem::take(&mut ui.root.children);
                let mut errors = Vec::new();
                fn collect(w: &Widget, errors: &mut Vec<String>) {
                    if !w.loaded {
                        errors.push(format!("menu widget {} was not loaded", w.name))
                    }
                    errors.extend(w.diagnostics.iter().cloned());
                    for child in &w.children {
                        collect(child, errors)
                    }
                }
                for widget in &forest {
                    collect(widget, &mut errors)
                }
                self.slots[kind.index()].forest = Some(forest);
                ui.clear();
                if !errors.is_empty() {
                    return Err(format!("actual menu {spec}: {}", errors.join("; ")));
                }
                if kind==Kind::Game {
                    //Retain this actual Game allocation's child before the
                    //first fallible timer/text operation; no retry rebuild.
                    self.game_world_10c=Some(game_ui::Owner::allocate());
                    self.game_world_10c.as_mut().ok_or("actual Game UIWorldState allocation absent")?.construct(ui)?;
                }
            }
            self.bound = true;
        }
        Ok(Selectors {
            splash: self.handle(Kind::Splash),
            main: self.handle(Kind::Main),
            loading: self.handle(Kind::Loading),
            game:self.handle(Kind::Game),
        })
    }
    fn require(&self, handle: &Handle) -> Result<Kind> {
        let origin = handle
            .origin
            .upgrade()
            .ok_or("menu allocation owner retired")?;
        if !Arc::ptr_eq(&origin, &self.origin) {
            return Err("menu belongs to a different frontend allocation".into());
        }
        if !self.bound {
            return Err("actual menu source construction has not completed".into());
        }
        Ok(handle.kind)
    }
    pub fn game_world_10c(&self,handle:&Handle)->Result<GameWorldReference>{
        if self.require(handle)?!=Kind::Game{return Err("UIWorldState10C belongs to the actual Game interface".into())}
        self.game_world_10c.as_ref().ok_or("actual Game UIWorldState10C constructor was not reached")?.reference()
    }
    pub fn show(&mut self, ui: &mut Ui, handle: &Handle) -> Result<()> {
        let kind = self.require(handle)?;
        if self.active == Some(kind) {
            return Ok(());
        }
        if let Some(old) = self.active {
            let names = ui
                .root
                .children
                .iter()
                .map(|w| w.name.clone())
                .collect::<Vec<_>>();
            let mut forest = Vec::new();
            for name in names {
                forest.push(
                    ui.disconnect(&name)
                        .ok_or("actual active menu forest changed during disconnect")?,
                )
            }
            self.slots[old.index()].forest = Some(forest);
        } else {
            ui.clear();
        }
        let forest = self.slots[kind.index()]
            .forest
            .take()
            .ok_or("actual menu forest is already lent")?;
        for widget in forest {
            ui.connect(widget)
        }
        ui.depth_mode = if kind == Kind::Loading {
            aa_ui::ui::DepthMode::TwoLists
        } else {
            aa_ui::ui::DepthMode::Layers
        };
        self.active = Some(kind);
        ui.layout();
        if kind == Kind::Game {
            //5C0440 connects the interface to the UI root: Game OnConnected
            //60BAE0 and its children's OnConnected placements.
            self.game_hud.as_mut().ok_or("actual Game interface Init6182B0 was not reached")?.on_connected(ui);
            //The interface's per-frame update 612010 and its children's
            //world reads (EconomyBar 602D10 via 5C1760, FactionExperience
            //604130, idle buttons 6233C0, ObjectDisplay, ActionButtons) need
            //a live-World HUD source that is not bound yet: the HUD stays in
            //its constructed/OnConnected state rather than invented values.
            log::warn!("Game interface shown; its 612010 per-frame live-World update is not bound yet");
        }
        Ok(())
    }
    ///Registry entries (48B800) read by the Game interface children updates.
    pub fn bind_game_resources(&mut self, resources: Vec<aa_ui::hud::HudResource>) -> Result<()> {
        self.game_hud.as_mut().ok_or("actual Game interface Init6182B0 was not reached")?.bind_resources(resources)
    }
    ///One Game interface frame (612010 and its children) on the installed
    ///Game forest, from simulation-thread reads of the same frame.
    pub fn game_update(&mut self, ui: &mut Ui, handle: &Handle, frame: &game_hud::LiveFrame) -> Result<()> {
        if self.require(handle)? != Kind::Game || self.active != Some(Kind::Game) {
            return Err("Game interface update requires the installed Game interface".into());
        }
        let hud = self.game_hud.as_mut().ok_or("actual Game interface Init6182B0 was not reached")?;
        let runtime = self.game_runtime.as_mut().ok_or("actual Game interface 61A650 fields are absent")?;
        game_hud::update(ui, hud, runtime, frame)
    }
    ///Show 5C0440 from an AppState OnEnter: the LoadingScreen (vtable
    ///A7EDD0) then runs its OnConnected 6E8730 with the session reads of
    ///the same transition. Other interfaces' OnConnected run inside show().
    pub fn show_connected(&mut self, ui: &mut Ui, handle: &Handle, loading: Option<&game_hud::LoadingFrame>) -> Result<()> {
        let kind = self.require(handle)?;
        let already = self.active == Some(kind);
        self.show(ui, handle)?;
        if kind == Kind::Loading && !already {
            let frame = loading.ok_or("LoadingScreen OnConnected 6E8730 requires its session reads")?;
            game_hud::loading_on_connected(ui, frame, &mut self.loading_hold_745580)?;
        }
        Ok(())
    }
    ///True while the actual Game interface forest is installed in the root.
    #[cfg(test)]
    pub fn game_active(&self) -> bool {
        self.active == Some(Kind::Game)
    }
    pub fn initialized_callback_e8(&self, handle: &Handle) -> Result<()> {
        let kind = self.require(handle)?;
        //The base Interface+E8 child is a bare RET on these concrete menus.
        //The vtable pin is verified by the callback fixture, not inferred
        //from a rendered frame or from Loading15/17.
        //Game interface vtable A5B0B0+E8 is the same bare RET 4B2830
        //(research/development10/game-state-entry/native_game_state.py).
        match kind {
            Kind::Loading | Kind::Splash | Kind::Main | Kind::Game => Ok(()),
        }
    }
    pub fn ready_button_visible(&self,ui:&Ui,handle:&Handle)->Result<bool>{
        if self.require(handle)?!=Kind::Loading||self.active!=Some(Kind::Loading){return Err("ReadyButton belongs to another/inactive interface".into())}
        Ok(ui.widget_by("LoadingScreen/ReadyButton").ok_or("actual ReadyButton allocation absent")?.flags&1!=0)
    }
    pub fn show_ready_button(&self,ui:&mut Ui,handle:&Handle)->Result<()>{
        self.ready_button_visible(ui,handle)?;
        if !ui.with_widget("LoadingScreen/ReadyButton",|w|w.set_visible(true)){return Err("actual ReadyButton allocation absent".into())}
        Ok(())
    }
    ///The modern loading presenter borrows the actual current Session148
    ///display value; this changes no Loading AppState flag.
    pub fn refresh_status(&mut self, ui: &mut Ui, handle: &Handle, text: &[u16]) -> Result<()> {
        let kind = self.require(handle)?;
        if kind != Kind::Loading {
            return Ok(());
        }
        if self.active != Some(kind) {
            return Err("current Loading menu is not installed in the actual Ui root".into());
        }
        let text = String::from_utf16(text)
            .map_err(|_| "current Session148 contains unsupported UTF16")?;
        if !ui.set_label("LoadingScreen/StatusLabel", &text) {
            return Err("actual LoadingScreen StatusLabel is absent".into());
        }
        Ok(())
    }
}
