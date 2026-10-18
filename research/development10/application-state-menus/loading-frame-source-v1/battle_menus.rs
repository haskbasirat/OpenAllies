//!Actual TGI interface allocations for the modern menu presentation backend.
//!Native AppState selectors retain weak handles; Widget forests stay on the UI
//!thread and move between this owner and the SAME Ui root. Legacy interface
//!animation-manager ABI is deliberately replaced by immediate presentation.
use aa_ui::{
    ui::Ui,
    widget::{Widget, WidgetClass},
};
use std::sync::{Arc, Weak};
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
        let mut owner=State::new();let menus=owner.bind(&mut ui).unwrap();
        owner.show(&mut ui,&menus.loading).unwrap();assert!(!owner.ready_button_visible(&ui,&menus.loading).unwrap());
        owner.show_ready_button(&mut ui,&menus.loading).unwrap();assert!(owner.ready_button_visible(&ui,&menus.loading).unwrap());
        owner.show(&mut ui,&menus.game).unwrap();assert!(ui.widget_by("Game").is_some());
        assert!(owner.ready_button_visible(&ui,&menus.loading).is_err());
        owner.show(&mut ui,&menus.loading).unwrap();assert!(owner.ready_button_visible(&ui,&menus.loading).unwrap());
        owner.show(&mut ui,&menus.main).unwrap();owner.show(&mut ui,&menus.game).unwrap();
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
    pub fn bind(&mut self, ui: &mut Ui) -> Result<Selectors> {
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
                    Kind::Splash|Kind::Game => {
                        //The modern Game root is an actual source-bound
                        //allocation. Positive HUD code-built children require
                        //their current live owners; no PlaceholderWorld is read.
                        ui.clear();
                        let mut widget = Widget::new(WidgetClass::Interface, spec, false);
                        ui.load_widget(&mut widget, None);
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
        Ok(())
    }
    pub fn initialized_callback_e8(&self, handle: &Handle) -> Result<()> {
        let kind = self.require(handle)?;
        //The base Interface+E8 child is a bare RET on these concrete menus.
        //The vtable pin is verified by the callback fixture, not inferred
        //from a rendered frame or from Loading15/17.
        match kind {
            Kind::Loading | Kind::Splash | Kind::Main => Ok(()),
            Kind::Game=>Err("actual Game interface E8 child requires its native binding".into()),
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
