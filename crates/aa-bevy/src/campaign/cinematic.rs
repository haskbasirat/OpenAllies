//! The FullscreenVideoWidget (`UI/Shared/video.tgi`, `0x79D8A0`) of the scripted
//! campaigns: a cinematic mission (`0x641D10`, with the end `0x641400`) and the
//! trigger action play_cutscene (`0x545C60` -> PlayVideo `0x411070`, whose end
//! `SendOrderOnVideoCompletion 0`, `0x60E7A0`, does nothing). The widget is
//! `disconnect_on_completion`, `solo_audio_video_widget` (the other audio groups
//! silent while it plays, `0x79D530` -> `0x747150(7)`) and not repeating.
//!
//! The frames and sound are the flipbooks of research/phase8-ui/bake_campaign_video.py
//! in `assets-cache/video/Campaign`; a movie that was not baked ends at once with a
//! log line (the movies are the original's: the bake needs an install).
use crate::ui::front::Front;
use crate::ui::video::{IntroSprite, VideoPlayer};
use bevy::prelude::*;
use std::path::PathBuf;

/// The folder of the bake.
pub fn root() -> Option<PathBuf> {
    crate::ui::find_up("assets-cache/video/Campaign")
}

/// `/Video/Campaign/AlliesIntro.bik` -> `AlliesIntro`.
pub fn stem(path: &str) -> String {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(s, _)| s).to_string()
}

/// Whether the movie can play.
pub fn baked(name: &str) -> bool {
    root().and_then(|d| VideoPlayer::once(&d, name)).is_some()
}

#[derive(Default)]
pub struct Run {
    name: Option<String>,
    player: Option<VideoPlayer>,
    sprite: Option<Entity>,
    audio: Option<(Entity, crate::audio::mixer::Mixer)>,
}

impl Run {
    fn stop(&mut self, commands: &mut Commands) {
        if let Some(e) = self.sprite.take() {
            commands.entity(e).despawn();
        }
        if let Some((e, m)) = self.audio.take() {
            m.stop(1);
            commands.entity(e).despawn();
        }
        self.player = None;
        self.name = None;
    }
}

/// One frame of the movie asked for by [`crate::campaign::CampaignUi::movie`].
#[allow(clippy::too_many_arguments)]
pub fn play(
    mut front: ResMut<Front>,
    time: Res<Time>,
    mut images: ResMut<Assets<Image>>,
    mixers: Option<ResMut<Assets<crate::audio::mixer::MixerAsset>>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut sprites: Query<&mut Sprite, With<IntroSprite>>,
    mut commands: Commands,
    mut run: Local<Run>,
) {
    let Some(name) = front.campaign.movie.clone() else {
        if run.name.is_some() {
            run.stop(&mut commands);
        }
        return;
    };
    let now = time.elapsed_secs_f64();
    if run.name.as_deref() != Some(name.as_str()) {
        run.stop(&mut commands);
        run.name = Some(name.clone());
        run.player = root().and_then(|d| VideoPlayer::once(&d, &name));
        if run.player.is_none() {
            warn!("the campaign movie {name} is not baked (research/phase8-ui/bake_campaign_video.py)");
        }
        let pcm = run.player.as_ref().and_then(|p| p.audio()).and_then(|p| std::fs::read(p).ok()).and_then(|b| crate::audio::mixer::decode_wav(&b).ok());
        if let (Some(pcm), Some(mut mixers)) = (pcm, mixers) {
            let mixer = crate::audio::mixer::Mixer::default();
            let handle = mixers.add(crate::audio::mixer::MixerAsset { mixer: mixer.clone() });
            let e = commands.spawn((bevy::audio::AudioPlayer::<crate::audio::mixer::MixerAsset>(handle), bevy::audio::PlaybackSettings::LOOP)).id();
            let gain = front.ui.uvars.get_f32("AudioMainVolume").clamp(0.0, 1.0);
            mixer.start(1, std::sync::Arc::new(pcm), [gain, gain], 1.0);
            run.audio = Some((e, mixer));
        }
    }
    // disconnect_on_completion, then the end's callback
    if run.player.as_ref().is_none_or(|p| p.finished()) {
        run.stop(&mut commands);
        front.campaign.movie = None;
        front.ui.events.push(aa_ui::ui::UiEvent::Command { widget: String::new(), command: super::CINEMATIC_ENDED.into() });
        return;
    }
    let Some(frame) = run.player.as_mut().and_then(|p| p.frame(now, &mut images)) else { return };
    // the VideoVE: the movie stretched over the window, above the interface
    let Ok(window) = windows.single() else { return };
    let size = Vec2::new(window.physical_width().max(1) as f32, window.physical_height().max(1) as f32);
    match run.sprite.and_then(|e| sprites.get_mut(e).ok()) {
        Some(mut s) => {
            if s.image != frame {
                s.image = frame;
            }
            if s.custom_size != Some(size) {
                s.custom_size = Some(size);
            }
        }
        None => {
            let e = commands
                .spawn((
                    Sprite { image: frame, custom_size: Some(size), ..default() },
                    Transform::from_xyz(0.0, 0.0, 900.0),
                    bevy::camera::visibility::RenderLayers::layer(crate::ui::render::UI_LAYER),
                    IntroSprite,
                ))
                .id();
            run.sprite = Some(e);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn movie_names_are_the_file_stems() {
        assert_eq!(super::stem("/Video/Campaign/AlliesVictory.bik"), "AlliesVictory");
    }
}
