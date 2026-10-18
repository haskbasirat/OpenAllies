//! The original menu backdrop: the five nation parade movies
//! (Data/Video/MainMenu/*IntroMovie.bik, listed in menu_background.tgi)
//! looping behind the menu column, stretched over the window as the
//! original does. Frames come from JPEG sequences baked by
//! research/bake_menu_video.py and are decoded on a worker thread.

use eframe::egui;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

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

pub struct MenuVideo {
    dir: PathBuf,
    movies: Vec<Movie>,
    current: usize,
    started: Instant,
    tex: Option<egui::TextureHandle>,
    shown: Option<(usize, usize)>,
    pending: Option<(usize, usize)>,
    req: Option<mpsc::Sender<(usize, PathBuf)>>,
    rx: Option<mpsc::Receiver<(usize, Option<egui::ColorImage>)>>,
}

impl MenuVideo {
    /// `dir` is assets-cache/video/MainMenu; returns an empty player (still
    /// art fallback) when the frames were not baked.
    pub fn new(dir: Option<&Path>) -> Self {
        let dir = dir.map(Path::to_path_buf).unwrap_or_default();
        let movies: Vec<Movie> = std::fs::read(dir.join("index.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Index>(&b).ok())
            .map(|i| i.movies.into_iter().filter(|m| m.frames > 0 && m.fps > 0.0).collect())
            .unwrap_or_default();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as usize)
            .unwrap_or(0);
        let current = if movies.is_empty() { 0 } else { seed % movies.len() };
        Self {
            dir,
            movies,
            current,
            started: Instant::now(),
            tex: None,
            shown: None,
            pending: None,
            req: None,
            rx: None,
        }
    }

    pub fn available(&self) -> bool {
        !self.movies.is_empty()
    }

    fn worker(&mut self, ctx: &egui::Context) {
        if self.req.is_some() {
            return;
        }
        let (req_tx, req_rx) = mpsc::channel::<(usize, PathBuf)>();
        let (img_tx, img_rx) = mpsc::channel();
        let c = ctx.clone();
        std::thread::spawn(move || {
            while let Ok(mut job) = req_rx.recv() {
                // Skip to the newest request if several queued up.
                while let Ok(newer) = req_rx.try_recv() {
                    job = newer;
                }
                let image = std::fs::read(&job.1)
                    .ok()
                    .and_then(|b| image::load_from_memory_with_format(&b, image::ImageFormat::Jpeg).ok())
                    .map(|img| {
                        let rgb = img.to_rgba8();
                        egui::ColorImage::from_rgba_unmultiplied([rgb.width() as usize, rgb.height() as usize], rgb.as_raw())
                    });
                if img_tx.send((job.0, image)).is_err() {
                    break;
                }
                c.request_repaint();
            }
        });
        self.req = Some(req_tx);
        self.rx = Some(img_rx);
    }

    /// Advance and paint the movie over `rect` on the background layer.
    /// Returns false when there is nothing to show (caller paints stills).
    pub fn paint(&mut self, ctx: &egui::Context, painter: &egui::Painter, rect: egui::Rect) -> bool {
        if self.movies.is_empty() {
            return false;
        }
        self.worker(ctx);
        let movie = &self.movies[self.current];
        let mut frame = (self.started.elapsed().as_secs_f32() * movie.fps) as usize;
        if frame >= movie.frames {
            // Next nation's parade.
            self.current = (self.current + 1) % self.movies.len();
            self.started = Instant::now();
            frame = 0;
        }
        let want = (self.current, frame);
        if let Some(rx) = &self.rx {
            while let Ok((key, image)) = rx.try_recv() {
                let decoded = self.pending.take().filter(|p| p.0 * 100_000 + p.1 == key);
                if let (Some(img), Some(at)) = (image, decoded) {
                    match &mut self.tex {
                        Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                        None => self.tex = Some(ctx.load_texture("menu-video", img, egui::TextureOptions::LINEAR)),
                    }
                    self.shown = Some(at);
                }
            }
        }
        if self.shown != Some(want)
            && self.pending.is_none()
            && let Some(req) = &self.req
        {
            let m = &self.movies[want.0];
            let path = self.dir.join(&m.name).join(format!("{:04}.jpg", want.1));
            if req.send((want.0 * 100_000 + want.1, path)).is_ok() {
                self.pending = Some(want);
            }
        }
        let Some(tex) = &self.tex else {
            painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
            ctx.request_repaint();
            return true;
        };
        painter.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        let m = &self.movies[self.current];
        ctx.request_repaint_after(std::time::Duration::from_secs_f32(0.5 / m.fps));
        true
    }
}
