//! Draw transport for the same installed VWorld. A camera/render packet never
//! writes native loading, task, application-state or simulation-ready fields.
use crate::{
    battle_controller::{Controller, Status},
    gpu::{SceneRenderer, camera_device::Viewport},
    live_scene::camera_backend::Frame,
};
use eframe::egui;
use std::sync::Arc;

struct Packet {
    frame: Arc<Frame>,
    world_revision: u32,
    init_word_b8: u32,
    map_id: String,
}

///The first actual window target remains fixed until a native camera resize
///operation is available. Window changes scale its image with aspect preserved;
///they cannot silently attach the old native projection to a new target ratio.
pub struct State {
    pixels: Option<[u32; 2]>,
    viewport: Option<Arc<Viewport>>,
    frame: Option<Packet>,
    #[cfg(windows)]
    pending: Option<aa_runtime::simulation_execution::JobTicket<Option<Packet>>>,
    error: Option<String>,
    presented: bool,
}
impl State {
    pub fn empty() -> Self {
        Self {
            pixels: None,
            viewport: None,
            frame: None,
            #[cfg(windows)]
            pending: None,
            error: None,
            presented: false,
        }
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    ///Run before pumping the actual camera's frontend RPC. In particular the
    ///first Renderer+40 reference must see this real allocated window target.
    pub fn prepare(&mut self, renderer: &mut SceneRenderer, pixels: [u32; 2]) -> Result<(), String> {
        let pixels = *self.pixels.get_or_insert(pixels);
        //Upload changes the renderer epoch. Reacquire its real target receipt
        //at the same dimensions; no projection or camera state is replaced.
        if self.viewport.as_ref().is_none_or(|v| v.require_current().is_err()) {
            self.frame = None;
            self.viewport = Some(Arc::new(
                renderer.prepare_camera_viewport(pixels).map_err(|e| e.to_string())?,
            ));
        }
        Ok(())
    }
    ///Only Send camera words/weak witnesses return from SimulationThread.
    ///The native VWorld/Directory/World stay on their retained thread.
    pub fn poll(&mut self, controller: &mut Controller) {
        #[cfg(windows)]
        {
            if let Some(ticket) = self.pending.as_mut() {
                match ticket.try_result() {
                    Ok(None) => return,
                    Ok(Some(frame)) => { self.frame = frame; self.pending = None; }
                    Err(error) => { self.error = Some(error); self.pending = None; }
                }
            }
            if self.error.is_some() || controller.status() != &Status::AwaitingNativeView {
                return;
            }
            let Some(viewport) = self.viewport.as_ref().map(Arc::clone) else { return };
            let map_id=controller.request().map(|r|r.map_id.clone()).unwrap_or_default();
            self.pending = match controller.submit_activated(move |loans, _| {
                //A queued upload/target change can legitimately supersede a
                //previous UI request. Defer only that packet; never infer AD.
                if viewport.require_current().is_err() { return Ok(None) }
                let Some(startup) = loans.view_startup.as_ref() else { return Ok(None) };
                let world = loans.session.world();
                let view = startup.native_view();
                view.directory().require_world(world).map_err(|e| e.to_string())?;
                let native = view.native();
                use aa_runtime::view_init_world::Phase;
                if matches!(native.phase(), Phase::Constructed | Phase::PrefixComplete |
                    Phase::SceneComplete | Phase::InitializingTerrainInputs | Phase::AwaitingTerrainBackend) {
                    return Ok(None);
                }
                let revision = world.presentation_parts()?.revision;
                native.scene().require_renderer(revision)?;
                let (camera, _) = loans.application_views.camera_and_visual_map();
                let snapshot = camera.directory().snapshot_current_for_scene(
                    revision, native.scene(), viewport,
                )?;
                let frame=Frame::from_current(snapshot, revision).map_err(|e|e.to_string())?;
                Ok(Some(Packet{frame,world_revision:revision.value(),init_word_b8:native.prefix().init_word_b8(),map_id}))
            }) {
                Ok(ticket) => Some(ticket),
                Err(error) => { self.error = Some(error); None }
            };
        }
        #[cfg(not(windows))]
        let _ = controller;
    }
    ///SceneRenderer validates the exact native camera, actual scene and target
    ///again at draw time. A stale camera is discarded and queried afresh.
    pub fn draw(&mut self, renderer: &mut SceneRenderer) -> Option<egui::TextureId> {
        let packet = self.frame.as_ref()?;
        let frame = &packet.frame;
        let pixels = frame.viewport_pixels();
        if frame.require_current(pixels).is_err() {
            self.frame = None;
            return None;
        }
        match renderer.render_native(frame, pixels) {
            Ok((texture, _)) => {
                if !self.presented {
                    log::info!("actual battle native frame presented: selected_map={} world_revision={} init_word_b8={} camera_pixels={}x{}",
                        packet.map_id,packet.world_revision,packet.init_word_b8,pixels[0],pixels[1]);
                    self.presented=true;
                }
                Some(texture)
            },
            Err(error) => { self.error = Some(error.to_string()); self.frame = None; None }
        }
    }
    pub fn image_rect(&self, available: egui::Rect) -> egui::Rect {
        aspect_fit(available, self.pixels.unwrap_or([1, 1]))
    }
}

fn aspect_fit(available: egui::Rect, pixels: [u32; 2]) -> egui::Rect {
    let source = egui::vec2(pixels[0] as f32, pixels[1] as f32);
    let scale = (available.width() / source.x).min(available.height() / source.y);
    egui::Rect::from_center_size(available.center(), source * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_resize_preserves_the_actual_native_target_aspect() {
        let wide = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(1600.0, 900.0));
        let fit = aspect_fit(wide, [800, 600]);
        assert_eq!(fit.size(), egui::vec2(1200.0, 900.0));
        assert_eq!(fit.center(), wide.center());
        let tall = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 1000.0));
        assert_eq!(aspect_fit(tall, [1600, 900]).size(), egui::vec2(600.0, 337.5));
    }
}
