//! The original user interface on Bevy: aa-ui (the ported AA.exe UI engine
//! with the original TGI layouts) builds every screen and emits a draw
//! list; this module feeds it input, runs the screen state machine and
//! paints the list. Owner: UI.
pub mod abilities;
pub mod banners;
pub mod custom;
pub mod datafile;
pub mod dialogs;
pub mod fog;
pub mod front;
pub mod gamedata;
pub mod glyphs;
pub mod hud;
pub mod render;
pub mod script;
pub mod selection;
pub mod minimap;
pub mod nifart;
pub mod panels;
pub mod portrait;
pub mod tex;
pub mod tooltip;
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
            .init_resource::<fog::FogDraw>()
            .init_resource::<portrait::Portraits>()
            .init_resource::<selection::Cursors>()
            .init_resource::<Quit>()
            .add_systems(Last, quit)
            .add_systems(Update, (script::run, front::frame, minimap_update, portrait::update, render::update_camera, render::paint, selection::cursor).chain())
            .add_systems(
                Update,
                (selection::update, banners::update).after(crate::input::battle_input).run_if(in_state(crate::session::AppState::Battle)),
            )
            // fog: the staging rule and the first fog state before the first
            // tick and frame; the texture and hidden actors after the tick
            .add_systems(Update, fog::start.before(crate::session::step_battle).run_if(in_state(crate::session::AppState::Battle)))
            // the generals of the battle before its first tick (combat owner)
            .add_systems(Update, abilities::start.before(crate::session::step_battle).run_if(in_state(crate::session::AppState::Battle)))
            .add_systems(Update, fog::update.after(crate::session::step_battle))
            .add_systems(
                Update,
                fog::hide_unseen.after(crate::session::step_battle).after(crate::battle_view::sync_actors).run_if(in_state(crate::session::AppState::Battle)),
            );
        if let Some(s) = script::Script::from_env() {
            app.insert_resource(s);
        }
    }
}

/// A quit asked for by the UI (Quit, the game menu, scripts), or the
/// window closed with its close button.
///
/// The exit crash of the 2560x1440 runs (the window covers the whole
/// 2560x1440 screen; Vulkan without vsync): leaving the winit loop drops the
/// App, and destroying the render world's GPU objects after winit destroyed
/// the window faulted inside the NVIDIA Vulkan driver (nvoglv64.dll, read
/// access violation, exit code 0xC0000005; the stack ran through bevy_winit's
/// runner, the thread-local WinitWindows destructor and the render resources'
/// drop into vulkan-1.dll). The process now ends right away once the UI asks
/// to quit (or the window is gone), before the App is torn down: the driver
/// releases its objects with the process, as after any process exit.
/// Nothing the game keeps needs a destructor then (settings are saved when
/// the options close). AA_BEVY_QUIT_DROP=1 keeps the old path.
#[derive(Resource, Default)]
pub struct Quit {
    pub requested: bool,
    had_window: bool,
}

fn quit(mut q: ResMut<Quit>, windows: Query<(), With<bevy::window::PrimaryWindow>>, mut exit: MessageWriter<AppExit>) {
    let window = !windows.is_empty();
    let closed = q.had_window && !window;
    q.had_window |= window;
    if !q.requested && !closed {
        return;
    }
    if std::env::var_os("AA_BEVY_QUIT_DROP").is_some() {
        exit.write(AppExit::Success);
        return;
    }
    info!("quit");
    std::process::exit(0);
}

/// The minimap's terrain image and points for this frame's draw list.
fn minimap_update(
    front: Res<front::Front>,
    battle: Option<Res<crate::session::Battle>>,
    mut art: ResMut<minimap::MinimapArt>,
    mut draw: ResMut<render::UiDraw>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&bevy::window::Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::camera::NativeCamera>>,
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
    // the camera's view on the ground: the window corners' rays on the
    // ground plane at the terrain height under the view centre
    let view = (|| {
        let window = windows.single().ok()?;
        let (camera, tf) = cameras.single().ok()?;
        let (w, h) = (window.width(), window.height());
        let ray = |x: f32, y: f32| camera.viewport_to_world(tf, Vec2::new(x, y)).ok();
        let centre = ray(w * 0.5, h * 0.5)?;
        let t0 = if centre.direction.y.abs() > 1e-4 { -centre.origin.y / centre.direction.y } else { 0.0 };
        let c = centre.origin + *centre.direction * t0;
        let ground = b.world.map.height_at(Vec2::new(c.x, -c.z));
        let mut out = [Vec2::ZERO; 4];
        for (i, (x, y)) in [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)].into_iter().enumerate() {
            let r = ray(x, y)?;
            if r.direction.y.abs() < 1e-4 {
                return None;
            }
            let t = (ground - r.origin.y) / r.direction.y;
            if t <= 0.0 {
                return None;
            }
            let p = r.origin + *r.direction * t;
            out[i] = Vec2::new(p.x, -p.z);
        }
        Some(out)
    })();
    draw.minimap = minimap::draw(&front.ui, &mut art, &b, rect, view);
    draw.minimap_image = art.image.clone();
}
