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
}

impl VideoPlayer {
    /// `dir` holds index.json and one folder per movie; an absent folder
    /// gives an empty player (drawn black).
    pub fn new(dir: &Path, seed: usize) -> Self {
        let movies: Vec<Movie> = std::fs::read(dir.join("index.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Index>(&b).ok())
            .map(|i| i.movies.into_iter().filter(|m| m.frames > 0 && m.fps > 0.0).collect())
            .unwrap_or_default();
        let current = if movies.is_empty() { 0 } else { seed % movies.len() };
        Self { dir: dir.to_path_buf(), movies, current, started: -1.0, image: None, shown: None, pending: None, req: None, rx: None }
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
            self.current = (self.current + 1) % self.movies.len();
            self.started = now;
            index = 0;
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
}
