//! Axis & Allies on Bevy: app wiring. Owner: UI (menus, HUD, camera, input).
//! The battle scene (lead) is added as `scene::ScenePlugin`.
mod battle_view;
mod camera;
mod demo_world;
mod scene;
mod session;
mod terrain;
mod textures;
mod units;

use bevy::prelude::*;

fn main() -> anyhow::Result<()> {
    let scene = scene::prepare()?;
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: scene::TITLE.into(),
            resolution: (1600u32, 900u32).into(),
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
    // Until the original menus are drawn in Bevy, start straight in loading (Berlin).
    .insert_state(session::AppState::Loading)
    .add_systems(Update, session::step_battle)
    .add_systems(Update, camera::update.run_if(in_state(session::AppState::Battle)));
    app.run();
    Ok(())
}
