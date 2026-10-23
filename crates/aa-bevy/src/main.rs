//! Axis & Allies on Bevy: app wiring. Owner: UI (menus, HUD, camera, input).
//! The battle scene (lead) is added as `scene::ScenePlugin`.
mod audio;
mod battle_view;
mod camera;
mod debug_battle;
mod effects;
mod model_fixes;
mod look;
mod input;
mod scene;
mod session;
mod terrain;
mod textures;
mod ui;
mod units;

use bevy::prelude::*;

fn main() -> anyhow::Result<()> {
    let scene = scene::prepare()?;
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        // the UI ends the process when the window closes (ui::Quit: the
        // 2560x1440 exit crash)
        exit_condition: bevy::window::ExitCondition::DontExit,
        primary_window: Some(Window {
            title: scene::TITLE.into(),
            // AA_BEVY_WINDOW=2560x1440 opens a window of that many pixels
            // (for side-by-side checks with the original's screenshots)
            resolution: match std::env::var("AA_BEVY_WINDOW").ok().and_then(|s| {
                let (w, h) = s.split_once('x')?;
                Some((w.trim().parse::<u32>().ok()?, h.trim().parse::<u32>().ok()?))
            }) {
                Some((w, h)) => bevy::window::WindowResolution::new(w, h).with_scale_factor_override(1.0),
                None => (1600u32, 900u32).into(),
            },
            present_mode: if std::env::var_os("AA_BEVY_VSYNC").is_some() {
                bevy::window::PresentMode::AutoVsync
            } else {
                bevy::window::PresentMode::AutoNoVsync
            },
            ..default()
        }),
        ..default()
    }))
    .add_plugins(scene)
    .add_plugins(ui::UiPlugin)
    // The original front end first; AA_BEVY_START=loading starts straight
    // in the loader (display tests).
    .insert_state(match std::env::var("AA_BEVY_START").as_deref() {
        Ok("loading") => session::AppState::Loading,
        _ => session::AppState::Menu,
    })
    .add_systems(Update, session::step_battle)
    // the original camera, selection and orders (after the UI took its input)
    .add_plugins(input::InputPlugin)
    // original effects and sound (effects.rs adds the sound plugin)
    .add_plugins(effects::EffectsPlugin);
    app.run();
    Ok(())
}
