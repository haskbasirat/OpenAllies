//! The screenshot key of the original (hotkeys_core.txt `map PrintScreen
//! root screenshot`, so it works on every screen):
//! * the `screenshot` command (0x5C4E50) asks for a shot (UI+D8) while UVar
//!   RendererEnableScreenshots is set; the frame (0x5C7AC0) takes it after
//!   drawing (0x70FB00, KKC_Renderer::SaveScreenShot);
//! * 0x70FB00 names the file after the application id `A&A` with the
//!   extension `png`, in UVar RendererScreenshotFolder (`Screenshots`) of the
//!   work depot ([0xB36540], the profile's data folder), and 0x85A7D0 makes it
//!   unique as `A&A_%04d.png`, counting on from the last number used (0xB40224,
//!   zero at start) past every file that exists;
//! * the picture is the whole window, 8-bit RGBA with every alpha 255 (as the
//!   original's own `Profile/data/Screenshots/A&A_0001.png`, 1024 x 768);
//! * the message log then shows `screenshot_taken|s` with the file's path, or
//!   `screenshot_failed`, as GENERIC_EVENT (0x40BC50 kind 0xF).
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::path::{Path, PathBuf};

pub struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenshotKey>().add_systems(Last, take);
    }
}

/// 0xB40224 and the key's state.
#[derive(Resource, Default)]
struct ScreenshotKey {
    /// The number 0x85A7D0 starts from.
    next: u32,
    /// Windows often delivers only the release of PrintScreen to a window;
    /// a release without its press counts as the press.
    down: bool,
}

/// 0x85A7D0 for `A&A` + `png`: the first `A&A_NNNN.png` from `next` on that
/// does not exist in `folder`, and its number.
fn unique_name(folder: &Path, next: u32, exists: impl Fn(&Path) -> bool) -> (PathBuf, u32) {
    let mut n = next;
    loop {
        let path = folder.join(format!("A&A_{n:04}.png"));
        if !exists(&path) {
            return (path, n);
        }
        n += 1;
    }
}

/// The captured window as the original writes it: RGBA, alpha 255.
fn opaque_rgba(image: &Image) -> Option<(Vec<u8>, u32, u32)> {
    let dynamic = image.clone().try_into_dynamic().ok()?;
    let mut rgba = dynamic.to_rgba8();
    for pixel in rgba.pixels_mut() {
        pixel.0[3] = 255;
    }
    let (w, h) = rgba.dimensions();
    Some((rgba.into_raw(), w, h))
}

/// A UVar of `Data/UVars.tgi`: the UI's value where it registers the name,
/// else the file's declaration (the renderer's own variables, which the UI
/// does not register).
fn uvar(front: &mut crate::ui::front::Front, name: &str) -> Option<String> {
    if front.ui.uvars.def(name).is_some() {
        return Some(front.ui.uvars.get_str(name));
    }
    let node = front.ui.store.node("UVars.tgi:Vars").ok()?;
    node.fields.iter().find(|f| f.name.eq_ignore_ascii_case(name)).and_then(|f| f.raw.as_deref()).map(|r| aa_ui::node::unquote(r).trim().to_string())
}

fn take(
    mut commands: Commands,
    mut keys: MessageReader<KeyboardInput>,
    mut state: ResMut<ScreenshotKey>,
    mut front: Option<ResMut<crate::ui::front::Front>>,
) {
    let mut asked = false;
    for k in keys.read().filter(|k| k.key_code == KeyCode::PrintScreen) {
        match k.state {
            ButtonState::Pressed => {
                if !k.repeat && !state.down {
                    asked = true;
                }
                state.down = true;
            }
            ButtonState::Released => {
                if !state.down {
                    asked = true;
                }
                state.down = false;
            }
        }
    }
    if !asked {
        return;
    }
    // 0x5C4E50: only while RendererEnableScreenshots (UVars.tgi: true)
    let enabled = front.as_mut().and_then(|f| uvar(f, "RendererEnableScreenshots")).is_none_or(|v| v.eq_ignore_ascii_case("true") || v == "1");
    if !enabled {
        info!("screenshot: RendererEnableScreenshots is off");
        return;
    }
    let folder_name = front.as_mut().and_then(|f| uvar(f, "RendererScreenshotFolder")).filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "Screenshots".into());
    let Some(depot) = crate::ui::gamedata::work_depot() else {
        warn!("screenshot: no profile data folder");
        return;
    };
    let folder = depot.join(folder_name.trim());
    let (path, n) = unique_name(&folder, state.next, |p| p.exists());
    state.next = n;
    commands.spawn(Screenshot::primary_window()).observe(move |captured: On<ScreenshotCaptured>, mut front: Option<ResMut<crate::ui::front::Front>>| {
        let saved = opaque_rgba(&captured.image)
            .ok_or_else(|| "the frame has no RGBA form".to_string())
            .and_then(|(bytes, w, h)| {
                std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
                image::save_buffer_with_format(&path, &bytes, w, h, image::ExtendedColorType::Rgba8, image::ImageFormat::Png).map_err(|e| e.to_string())
            });
        let Some(front) = front.as_mut() else { return };
        let text = match saved {
            Ok(()) => {
                info!("screenshot: {}", path.display());
                front.ui.strings.resolve("#screenshot_taken|s").replacen("%s", &path.display().to_string(), 1)
            }
            Err(e) => {
                warn!("screenshot {}: {e}", path.display());
                front.ui.strings.resolve("#screenshot_failed").to_string()
            }
        };
        let uv = front.ui.uvars.clone();
        front.ui.messages.send(&uv, "GENERIC_EVENT", &text, None);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_counts_on_past_existing_files() {
        let folder = Path::new("Screenshots");
        let taken = [folder.join("A&A_0000.png"), folder.join("A&A_0001.png"), folder.join("A&A_0003.png")];
        let exists = |p: &Path| taken.iter().any(|t| t == p);
        assert_eq!(unique_name(folder, 0, exists), (folder.join("A&A_0002.png"), 2));
        // 0xB40224 keeps the last number: the next search starts there
        assert_eq!(unique_name(folder, 3, exists), (folder.join("A&A_0004.png"), 4));
        assert_eq!(unique_name(folder, 12345, exists).0, folder.join("A&A_12345.png"));
    }
}
