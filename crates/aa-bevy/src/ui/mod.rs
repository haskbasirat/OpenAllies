//! The original user interface on Bevy: aa-ui (the ported AA.exe UI engine
//! with the original TGI layouts) builds every screen and emits a draw
//! list; this module feeds it input, runs the screen state machine and
//! paints the list. Owner: UI.
pub mod banners;
pub mod custom;
pub mod datafile;
pub mod dialogs;
pub mod front;
pub mod gamedata;
pub mod glyphs;
pub mod hud;
pub mod render;
pub mod script;
pub mod selection;
pub mod minimap;
pub mod nifart;
pub mod tex;
pub mod video;

use bevy::prelude::*;
use std::path::PathBuf;

fn find_up(rel: &str) -> Option<PathBuf> {
    let mut starts = vec![];
    if let Ok(exe) = std::env::current_exe() {
        starts.push(exe);
    }
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    starts.iter().flat_map(|s| s.ancestors()).map(|a| a.join(rel)).find(|p| p.exists())
}

/// The reconstruction folder (original Data, converted textures, skins).
pub fn reconstruction_root() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("AA_RECONSTRUCTION") {
        return Some(PathBuf::from(p));
    }
    find_up("reconstruction/assets/original/Data").and_then(|p| p.parent()?.parent()?.parent().map(PathBuf::from))
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let Some(root) = reconstruction_root() else {
            error!("the reconstruction folder (original Data) was not found: no UI");
            return;
        };
        let video = find_up("assets-cache/video").unwrap_or_else(|| root.join("assets-cache/video"));
        let data = aa_ui::fs::DataRoot::with_overlay(root.join("assets/original/Data"), root.join("native-game/runtime-expanded-v14/Data"));
        app.insert_resource(render::UiRenderer::new(&root, video, data))
            .insert_resource(render::UiDraw::default())
            .insert_resource(front::Front::new(&root))
            .add_systems(Startup, render::spawn_camera)
            .init_resource::<selection::SelectionArt>()
            .init_resource::<banners::BannerArt>()
            .init_resource::<minimap::MinimapArt>()
            .init_resource::<selection::Cursors>()
            .add_systems(Update, (script::run, front::frame, minimap_update, render::update_camera, render::paint, selection::cursor).chain())
            .add_systems(
                Update,
                (selection::update, banners::update).after(crate::input::battle_input).run_if(in_state(crate::session::AppState::Battle)),
            );
        if let Some(s) = script::Script::from_env() {
            app.insert_resource(s);
        }
    }
}

/// The minimap's terrain image and points for this frame's draw list.
fn minimap_update(
    front: Res<front::Front>,
    battle: Option<Res<crate::session::Battle>>,
    mut art: ResMut<minimap::MinimapArt>,
    mut draw: ResMut<render::UiDraw>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(b), true) = (battle, front.page == front::Page::Game) else {
        draw.minimap = None;
        return;
    };
    let rect = draw.list.commands.iter().find_map(|c| match &c.kind {
        aa_ui::draw::DrawKind::External { ve_type, rect, .. } if ve_type == "MiniMap" => Some(*rect),
        _ => None,
    });
    let Some(rect) = rect else {
        draw.minimap = None;
        return;
    };
    art.terrain(&front.ui.store.root, &b, &mut images);
    draw.minimap = minimap::draw(&front.ui, &art, &b, rect);
    draw.minimap_image = art.image.clone();
}
