//! Menu cinematics (`[VideoResource]` lists of Cinematic widgets): the five
//! nation parade movies behind the main menu (menu_background.tgi
//! IntroCinematic) and the globe (MiniMenuBackground, AAGlobeBig).
//!
//! The original plays Bink files; the frames come from the JPEG sequences
//! baked by research/bake_menu_video.py and research/phase8-ui/bake_globe.py
//! into assets-cache/video/<folder>/<movie>/NNNN.jpg (same source and
//! player rules as aa-client `menu_video.rs`: a random first movie, then the
//! next one in authored order, each looping at its own frame rate). Frames
//! are decoded on a worker thread and written into one image per player.
use super::tex::{Variant, make_image};
use bevy::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

#[derive(Deserialize)]
struct Index {
    movies: Vec<Movie>,
}

#[derive(Deserialize, Clone)]
struct Movie {
    name: String,
    fps: f32,
    frames: usize,
    /// The baked sound track (research/bake_intro_video.py), if any.
    #[serde(default)]
    audio: Option<String>,
}

type Frame = (u32, u32, Vec<u8>);

pub struct VideoPlayer {
    dir: PathBuf,
    movies: Vec<Movie>,
    current: usize,
    started: f64,
    image: Option<(Handle<Image>, [u32; 2])>,
    shown: Option<(usize, usize)>,
    pending: Option<(usize, usize)>,
    req: Option<mpsc::Sender<((usize, usize), PathBuf)>>,
    rx: Option<std::sync::Mutex<mpsc::Receiver<((usize, usize), Option<Frame>)>>>,
    /// Play the movie once (a FullscreenVideoWidget, `repeating_video =
    /// false`) instead of looping the list.
    once: bool,
    finished: bool,
}

fn read_index(dir: &Path) -> Vec<Movie> {
    std::fs::read(dir.join("index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Index>(&b).ok())
        .map(|i| i.movies.into_iter().filter(|m| m.frames > 0 && m.fps > 0.0).collect())
        .unwrap_or_default()
}

impl VideoPlayer {
    /// `dir` holds index.json and one folder per movie; an absent folder
    /// gives an empty player (drawn black).
    pub fn new(dir: &Path, seed: usize) -> Self {
        let movies = read_index(dir);
        let current = if movies.is_empty() { 0 } else { seed % movies.len() };
        Self { dir: dir.to_path_buf(), movies, current, started: -1.0, image: None, shown: None, pending: None, req: None, rx: None, once: false, finished: false }
    }

    /// One movie of `dir` played once (`None` when it was not baked).
    pub fn once(dir: &Path, name: &str) -> Option<Self> {
        let movies: Vec<Movie> = read_index(dir).into_iter().filter(|m| m.name.eq_ignore_ascii_case(name)).take(1).collect();
        if movies.is_empty() {
            return None;
        }
        Some(Self { dir: dir.to_path_buf(), movies, current: 0, started: -1.0, image: None, shown: None, pending: None, req: None, rx: None, once: true, finished: false })
    }

    /// A once-played movie reached its last frame (the Bink frame number
    /// equals the frame count, 0x79DB30).
    pub fn finished(&self) -> bool {
        self.finished
    }

    fn worker(&mut self) {
        if self.req.is_some() {
            return;
        }
        let (req_tx, req_rx) = mpsc::channel::<((usize, usize), PathBuf)>();
        let (img_tx, img_rx) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(mut job) = req_rx.recv() {
                while let Ok(newer) = req_rx.try_recv() {
                    job = newer;
                }
                let frame = std::fs::read(&job.1)
                    .ok()
                    .and_then(|b| image::load_from_memory_with_format(&b, image::ImageFormat::Jpeg).ok())
                    .map(|img| {
                        let rgba = img.to_rgba8();
                        (rgba.width(), rgba.height(), rgba.into_raw())
                    });
                if img_tx.send((job.0, frame)).is_err() {
                    break;
                }
            }
        });
        self.req = Some(req_tx);
        self.rx = Some(std::sync::Mutex::new(img_rx));
    }

    /// Advance to `now` (seconds) and return the image to draw, if any.
    pub fn frame(&mut self, now: f64, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        if self.movies.is_empty() {
            return None;
        }
        self.worker();
        if self.started < 0.0 {
            self.started = now;
        }
        let movie = &self.movies[self.current];
        let mut index = ((now - self.started) * movie.fps as f64) as usize;
        if index >= movie.frames {
            if self.once {
                self.finished = true;
                index = movie.frames - 1;
            } else {
                self.current = (self.current + 1) % self.movies.len();
                self.started = now;
                index = 0;
            }
        }
        let want = (self.current, index);
        let received: Vec<_> = self.rx.as_ref().and_then(|rx| rx.lock().ok()).map(|rx| rx.try_iter().collect()).unwrap_or_default();
        {
            for (at, frame) in received {
                if self.pending == Some(at) {
                    self.pending = None;
                }
                let Some((w, h, data)) = frame else { continue };
                match &self.image {
                    Some((handle, size)) if *size == [w, h] => {
                        if let Some(mut img) = images.get_mut(handle) {
                            img.data = Some(data);
                        }
                    }
                    _ => {
                        // the movie is stretched over its rect with bilinear filtering
                        let mut image = make_image(w, h, data, Variant::default(), true);
                        image.sampler = bevy::image::ImageSampler::linear();
                        let handle = images.add(image);
                        self.image = Some((handle, [w, h]));
                    }
                }
                self.shown = Some(at);
            }
        }
        if self.shown != Some(want) && self.pending.is_none() {
            if let Some(req) = &self.req {
                let m = &self.movies[want.0];
                let path = self.dir.join(&m.name).join(format!("{:04}.jpg", want.1));
                if req.send((want, path)).is_ok() {
                    self.pending = Some(want);
                }
            }
        }
        self.image.as_ref().map(|(h, _)| h.clone())
    }

    /// The current movie's baked sound track file.
    fn audio(&self) -> Option<PathBuf> {
        self.movies.get(self.current).and_then(|m| m.audio.as_ref()).map(|a| self.dir.join(a))
    }
}

/// The intro movies at startup: AVars IntroVideo1..N (AVars_version.tgi:
/// /Video/Intro/ESRB, TimeGate, Atari, ATI), collected by 0x5C4910 at the
/// end of the application init and played one after another by 0x5C4AF0 in
/// the FullscreenVideoWidget (UI/Shared/video.tgi: a full-screen VideoVE,
/// disconnect_on_completion, solo_audio_video_widget, not repeating); each
/// completion (0x5C1560) starts the next, and after the last the
/// application shows the main menu (0x40CE10 with state 3). The widget
/// class (0x5E0FD0) handles no input and the video is only done at its
/// last frame (0x79DB30), so the movies are not skipped. Frames and sound
/// come from research/bake_intro_video.py (assets-cache/video/Intro); a
/// movie that was not baked is passed over. The v14 runtime's
/// AVars_version.tgi comments IntroVideo1..4 out ("Expanded fast startup"):
/// on the accepted baseline the list is empty and the main menu shows at
/// once, exactly like its AA.exe; the movies only play on stock data.
#[derive(Default)]
pub struct IntroRun {
    movies: Vec<String>,
    next: usize,
    player: Option<VideoPlayer>,
    sprite: Option<Entity>,
    audio: Option<(Entity, crate::audio::mixer::Mixer)>,
    started: bool,
}

#[derive(Component)]
pub struct IntroSprite;

/// The baked movie's name: the Bink file's stem (`/Video/Intro/ESRB.bik`).
fn movie_stem(path: &str) -> String {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem).to_string()
}

fn intro_root() -> Option<PathBuf> {
    super::find_up("assets-cache/video/Intro")
}

/// Whether this run starts with the intro movies: not for scripted or
/// display-test runs (AA_UI_SCRIPT, AA_BEVY_START) or AA_BEVY_NO_INTRO,
/// unless AA_BEVY_INTRO=1 asks for them (a scripted check of the intro).
pub fn intro_enabled() -> bool {
    let forced = std::env::var_os("AA_BEVY_INTRO").is_some();
    (forced || ["AA_UI_SCRIPT", "AA_BEVY_START", "AA_BEVY_NO_INTRO"].iter().all(|v| std::env::var_os(v).is_none())) && intro_root().is_some()
}

#[allow(clippy::too_many_arguments)]
pub fn intro(
    front: Option<ResMut<super::front::Front>>,
    time: Res<Time>,
    mut images: ResMut<Assets<Image>>,
    mixers: Option<ResMut<Assets<crate::audio::mixer::MixerAsset>>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut sprites: Query<&mut Sprite, With<IntroSprite>>,
    mut commands: Commands,
    mut run: Local<IntroRun>,
) {
    let Some(mut front) = front else { return };
    let playing = front.page == super::front::Page::Intro;
    if !playing {
        if run.started {
            if let Some(e) = run.sprite.take() {
                commands.entity(e).despawn();
            }
            if let Some((e, m)) = run.audio.take() {
                m.stop(1);
                commands.entity(e).despawn();
            }
            *run = IntroRun::default();
        }
        return;
    }
    let Some(dir) = intro_root() else {
        front.show(super::front::Page::MainMenu);
        return;
    };
    if !run.started {
        run.started = true;
        // 0x5C4910: IntroVideo1, IntroVideo2, ... while the AVar exists
        let mut i = 1;
        while let Some(path) = front.ui.avars.get(&format!("IntroVideo{i}")).map(aa_ui::node::unquote) {
            run.movies.push(movie_stem(&path));
            i += 1;
        }
    }
    let now = time.elapsed_secs_f64();
    // 0x5C4AF0: the next movie once the current one completed
    if run.player.as_ref().is_none_or(|p| p.finished()) {
        if let Some((_, m)) = &run.audio {
            m.stop(1);
        }
        run.player = None;
        while run.player.is_none() && run.next < run.movies.len() {
            let name = run.movies[run.next].clone();
            run.next += 1;
            run.player = VideoPlayer::once(&dir, &name);
        }
        let Some(player) = run.player.as_ref() else {
            front.show(super::front::Page::MainMenu);
            return;
        };
        // its sound track, solo (the other audio groups are silent while it
        // plays, 0x79D530 -> 0x747150(7)), at the main volume
        let pcm = player.audio().and_then(|p| std::fs::read(p).ok()).and_then(|b| crate::audio::mixer::decode_wav(&b).ok());
        if let (Some(pcm), Some(mut mixers)) = (pcm, mixers) {
            if run.audio.is_none() {
                let mixer = crate::audio::mixer::Mixer::default();
                let handle = mixers.add(crate::audio::mixer::MixerAsset { mixer: mixer.clone() });
                let e = commands.spawn((bevy::audio::AudioPlayer::<crate::audio::mixer::MixerAsset>(handle), bevy::audio::PlaybackSettings::ONCE)).id();
                run.audio = Some((e, mixer));
            }
            let gain = front.ui.uvars.get_f32("AudioMainVolume").clamp(0.0, 1.0);
            if let Some((_, m)) = &run.audio {
                m.start(1, std::sync::Arc::new(pcm), [gain, gain], 1.0);
            }
        }
    }
    let Some(frame) = run.player.as_mut().and_then(|p| p.frame(now, &mut images)) else { return };
    // the full-screen VideoVE: the movie stretched over the window
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
                    bevy::camera::visibility::RenderLayers::layer(super::render::UI_LAYER),
                    IntroSprite,
                ))
                .id();
            run.sprite = Some(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movie_names_come_from_the_avar_paths() {
        assert_eq!(movie_stem("/Video/Intro/ESRB.bik"), "ESRB");
        assert_eq!(movie_stem("Videos\\project_01.bik"), "project_01");
        assert_eq!(movie_stem("plain"), "plain");
    }

    /// A once-played movie runs to its last frame and then reports done; a
    /// movie that was not baked gives no player.
    #[test]
    fn a_once_played_movie_finishes() {
        let dir = std::env::temp_dir().join(format!("aa-bevy-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.json"), r#"{"movies":[{"name":"ESRB","fps":30.0,"frames":90},{"name":"Atari","fps":30.0,"frames":120,"audio":"Atari.wav"},{"name":"Empty","fps":30.0,"frames":0}]}"#).unwrap();
        assert!(VideoPlayer::once(&dir, "ATI").is_none());
        assert!(VideoPlayer::once(&dir, "Empty").is_none());
        assert_eq!(VideoPlayer::once(&dir, "atari").unwrap().audio(), Some(dir.join("Atari.wav")));
        let mut images = Assets::<Image>::default();
        let mut p = VideoPlayer::once(&dir, "esrb").unwrap();
        assert_eq!(p.audio(), None);
        p.frame(10.0, &mut images);
        assert!(!p.finished());
        // frame 89 of 90 at 30 frames per second
        p.frame(10.0 + 89.0 / 30.0, &mut images);
        assert!(!p.finished());
        p.frame(10.0 + 3.0, &mut images);
        assert!(p.finished());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
