//! In-game HUD behaviour (`hud.rs`) over the PLACEHOLDER worlds of the
//! original screenshots 04..07.

use aa_ui::fs::DataRoot;
use aa_ui::hud::{Hud, HudSelection, HudWorld, PlaceholderScene, PlaceholderWorld, GAME};
use aa_ui::layout::Screen;
use aa_ui::ui::Ui;
use aa_ui::widget::Widget;

fn new_ui() -> Ui {
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: 2560, height: 1440 });
    ui.set_skin(Some("USA"), DataRoot::workspace_skins());
    ui
}

fn run(ui: &mut Ui, world: &dyn HudWorld) -> Hud {
    let mut hud = Hud::show(ui, world);
    for t in [0u64, 1000, 2000] {
        ui.update(t);
        hud.update(ui, world);
    }
    hud
}

fn text(ui: &Ui, path: &str) -> String {
    let w = ui.widget(path).unwrap_or_else(|| panic!("{path}"));
    match &w.sub_ve.as_ref().unwrap().kind {
        aa_ui::ve::VeKind::Label(l) => l.text.clone(),
        _ => panic!(),
    }
}

fn shown(ui: &Ui, path: &str) -> bool {
    ui.widget(path).is_some_and(|w: &Widget| w.flags & 1 != 0)
}

fn glyph(ui: &Ui, ids: &str) -> char {
    *ui.strings.glyphs.get(ids).unwrap()
}

#[test]
fn economy_bar_texts_and_the_impact_slide() {
    let mut ui = new_ui();
    let world = PlaceholderWorld { scene: PlaceholderScene::Ingame };
    let _hud = run(&mut ui, &world);
    let bar = format!("{GAME}/EconomyBar");
    let money = glyph(&ui, "ch_money");
    let ammo = glyph(&ui, "ch_ammo");
    let unit = glyph(&ui, "ch_unitlimit");
    let bld = glyph(&ui, "ch_building");
    // 0x5C9830 / 0x5C8F10
    assert_eq!(text(&ui, &format!("{bar}/money/ResourceLabel")), format!("{money} 935(+60)"));
    assert_eq!(text(&ui, &format!("{bar}/ammo/ResourceLabel")), format!("{ammo} +20"));
    assert_eq!(text(&ui, &format!("{bar}/unit_limit_consumed/ResourceLabel")), format!("{unit}1/5 "));
    assert_eq!(text(&ui, &format!("{bar}/building_limit_consumed/ResourceLabel")), format!("{bld}2/100 "));
    // limit providers are shown but never written (resource+0x2C)
    assert!(shown(&ui, &format!("{bar}/unit_limit_provided")));
    assert_eq!(text(&ui, &format!("{bar}/unit_limit_provided/ResourceLabel")), "");
    // hidden offset 0,25: the bar sits at T = 520 + 25 without an impact
    assert_eq!(ui.raw_position(&bar).unwrap().1, 545.0);
    assert!(!shown(&ui, &format!("{GAME}/EconomicImpact")));

    // 07: an impact slides the bar up and shows the projection
    let mut ui = new_ui();
    let world = PlaceholderWorld { scene: PlaceholderScene::Recruit };
    let _hud = run(&mut ui, &world);
    assert_eq!(ui.raw_position(&bar).unwrap().1, 520.0);
    let imp = format!("{GAME}/EconomicImpact");
    assert!(shown(&ui, &imp));
    assert_eq!(text(&ui, &format!("{imp}/money/ResourceLabel")), format!("{money} 964(+60)"));
    assert_eq!(text(&ui, &format!("{imp}/ammo/ResourceLabel")), format!("{ammo} +15"));
    assert_eq!(text(&ui, &format!("{imp}/unit_limit_consumed/ResourceLabel")), format!("{unit}2/5 "));
    // unchanged resources are hidden in the impact bar
    assert!(!shown(&ui, &format!("{imp}/oil")));
    assert!(!shown(&ui, &format!("{imp}/building_limit_consumed")));
}

#[test]
fn faction_experience_time_and_idle_buttons() {
    let mut ui = new_ui();
    let world = PlaceholderWorld { scene: PlaceholderScene::Ingame };
    let _hud = run(&mut ui, &world);
    // 0x4CE3A0 with FontFactionExperienceGlyphIndex = 133
    assert_eq!(text(&ui, &format!("{GAME}/FactionExperience")), "\u{85} 1021/5000");
    // ViewShowElapsedGameTime defaults to false
    assert!(!shown(&ui, &format!("{GAME}/TimeMessage")));
    assert_eq!(text(&ui, &format!("{GAME}/TimeMessage")), "0:00:00");
    assert!(shown(&ui, &format!("{GAME}/IdleButtonBuilder")));
    assert!(!shown(&ui, &format!("{GAME}/IdleButtonDetachedOrganization")));
    let cmd = ui.widget(&format!("{GAME}/IdleButtonBuilder/IdleButton")).unwrap().command.clone();
    assert_eq!(cmd.as_deref(), Some("CycleSelection 1"));
}

#[test]
fn code_placed_grids() {
    let mut ui = new_ui();
    let world = PlaceholderWorld { scene: PlaceholderScene::Regiment };
    let _hud = run(&mut ui, &world);
    // ActionButtons 0x5F4540: cells floor(185/4) x floor(138/3) = 46 x 46,
    // 42 px buttons centred by floor(2)
    let ab = format!("{GAME}/ControlPanel/ActionButtons");
    for i in 0..12usize {
        let seg = if i == 0 { "ActionButtonsButton".to_string() } else { format!("ActionButtonsButton#{i}") };
        let (x, y) = ui.raw_position(&format!("{ab}/{seg}")).unwrap();
        assert_eq!((x, y), ((i % 4) as f32 * 46.0 + 2.0, (i / 4) as f32 * 46.0 + 2.0), "{i}");
    }
    // Groups 0x6065D0: x = i * 339 * 0.1
    let g = format!("{GAME}/GroupFloater/Groups");
    for i in 0..10usize {
        let seg = if i == 0 { "GroupButton".to_string() } else { format!("GroupButton#{i}") };
        let x = ui.raw_position(&format!("{g}/{seg}")).unwrap().0;
        assert!((x - i as f32 * 339.0 * 0.1f32).abs() < 1e-3, "{i}: {x}");
    }
    // FormationButtons 0x623D30: y = i * floor(142 / 3)
    let d = format!("{GAME}/ControlPanel/ObjectDisplay/CompanyDisplay");
    for i in 0..3usize {
        let seg = if i == 0 { "FormationButton".to_string() } else { format!("FormationButton#{i}") };
        assert_eq!(ui.raw_position(&format!("{d}/FormationButtons/{seg}")).unwrap().1, i as f32 * 47.0);
    }
    // CompanyUnits 0x62F690: two rows, the second at border + height +
    // vertical separation
    let rows: Vec<f32> = (0..8usize)
        .map(|i| {
            let seg = if i == 0 { "CompanyUnit".to_string() } else { format!("CompanyUnit#{i}") };
            ui.raw_position(&format!("{d}/CompanyUnits/{seg}")).unwrap().1
        })
        .collect();
    assert_eq!(rows.iter().filter(|y| **y == rows[0]).count(), 4);
    assert!(rows.iter().all(|y| *y == rows[0] || *y > rows[0]));
    // texts
    assert_eq!(text(&ui, &format!("{d}/Name")), "1st Infantry Regiment");
    assert_eq!(text(&ui, &format!("{d}/State")), "Entrenched");
    let ammo = glyph(&ui, "ch_ammo");
    let unit = glyph(&ui, "ch_unitlimit");
    assert_eq!(text(&ui, &format!("{d}/Economy")), format!("{ammo}-10  {unit}1  "));
    let gem = glyph(&ui, "ch_gem");
    assert_eq!(text(&ui, &format!("{d}/Owner")), format!("<color=0,0.2,1>{gem}<color=0,1,0> Player"));
}

#[test]
fn hq_attachment_grid_and_selection_changes() {
    let mut ui = new_ui();
    let corps = PlaceholderWorld { scene: PlaceholderScene::Corps };
    let mut hud = run(&mut ui, &corps);
    let d = format!("{GAME}/ControlPanel/ObjectDisplay/CompanyDisplay");
    // 0x62D010: steps floor((148 - 42) / 2) = 53 and floor(101 - 50) = 51
    let want = [(0.0, 0.0), (53.0, 0.0), (106.0, 0.0), (0.0, 51.0), (53.0, 51.0)];
    for (i, w) in want.iter().enumerate() {
        let seg = if i == 0 { "HQAttachedOrganization".to_string() } else { format!("HQAttachedOrganization#{i}") };
        let p = format!("{d}/HQAttached/{seg}");
        assert_eq!(ui.raw_position(&p).unwrap(), *w, "{i}");
        assert!(shown(&ui, &p));
    }
    assert!(!shown(&ui, &format!("{d}/HQAttached/HQAttachedOrganization#5")));
    assert!(ui.widget(&format!("{d}/CompanyUnits")).is_none());
    // nothing selected: Clear 0x624160 removes the display
    struct NoSel(PlaceholderWorld);
    impl HudWorld for NoSel {
        fn resources(&self) -> Vec<aa_ui::hud::HudResource> {
            self.0.resources()
        }
        fn economy(&self) -> Option<aa_ui::hud::HudEconomy> {
            self.0.economy()
        }
        fn economic_impact(&self) -> Option<aa_ui::hud::HudImpact> {
            None
        }
        fn faction_experience(&self) -> Option<aa_ui::hud::FactionExperience> {
            None
        }
        fn faction_abilities(&self) -> Vec<aa_ui::hud::HudAbility> {
            Vec::new()
        }
        fn group_textures(&self) -> Vec<Option<String>> {
            Vec::new()
        }
        fn idle_builder(&self) -> bool {
            false
        }
        fn idle_detached_organization(&self) -> bool {
            false
        }
        fn game_time(&self) -> f32 {
            3725.0
        }
        fn selection(&self) -> HudSelection {
            HudSelection::None
        }
        fn action_buttons(&self) -> Vec<Option<aa_ui::hud::HudAction>> {
            vec![None; 12]
        }
    }
    let none = NoSel(PlaceholderWorld { scene: PlaceholderScene::Ingame });
    ui.uvars.set_bool("ViewShowElapsedGameTime", true);
    ui.update(5000);
    hud.update(&mut ui, &none);
    assert!(ui.widget(&d).is_none());
    assert!(!shown(&ui, &format!("{GAME}/FactionExperience")));
    assert!(shown(&ui, &format!("{GAME}/TimeMessage")));
    assert_eq!(text(&ui, &format!("{GAME}/TimeMessage")), "1:02:05");
    // and back: the stashed display returns
    ui.update(6000);
    hud.update(&mut ui, &corps);
    assert!(ui.widget(&d).is_some());
}
