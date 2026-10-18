//! Isolated interactive rendering exercise. This deliberately owns a preview
//! Character and local clock, not an ActorStore or a simulated battlefield.
use crate::{
    camera::Camera,
    gpu::{CpuScene, SceneRenderer},
    inspection::SceneObjects,
    live_scene::{
        ActorVisual, LiveScene,
        spawn::{self, PreparedSpawn, SpawnActor, animation::ActorRig},
    },
    loading::LoadSettings,
};
use aa_animation::{
    controller_clock::ClockCache,
    definitions::BoundAnimation,
    runtime::{self, Character},
};
use aa_assets::AssetStore;
use aa_body::F32;
use aa_numeric::Precision;
use anyhow::{Context, Result, ensure};
use eframe::egui;

pub const ACTORS: [(&str, &str); 5] = [
    ("usa_infantry", "USA infantry"),
    ("ger_infantry", "German infantry"),
    ("gbr_infantry", "British infantry"),
    ("jpn_infantry", "Japanese infantry"),
    ("rus_infantry", "Soviet infantry"),
];
const P: Precision = Precision::Double53;
const ID: u32 = 1; // Isolated preview identity only, never registered gameplay state.
const RESOURCES: [&str; 4] = ["Idle", "Jog", "Attack1", "Die1"];
fn f(v: f32) -> F32 {
    F32(v.to_bits())
}
struct Clip {
    key: String,
    definition: BoundAnimation,
}
pub struct Prepared {
    spawn: PreparedSpawn,
    rig: ActorRig,
    clips: Vec<Clip>,
    selected: usize,
    epoch: u64,
    visual: ActorVisual,
    time: f32,
}
pub struct Active {
    rig: ActorRig,
    clips: Vec<Clip>,
    selected: usize,
    state: Character<String>,
    visual: ActorVisual,
    time: f32,
    sequence: u64,
    cache: ClockCache,
    playing: bool,
    dirty: bool,
    pub source: String,
    notices: Vec<String>,
    heading: f32,
    position: [f32; 2],
}
pub fn prepare(
    store: &AssetStore,
    settings: LoadSettings,
    id: &str,
    epoch: u64,
    clip: Option<&str>,
    time: f32,
) -> Result<(Prepared, CpuScene, Camera)> {
    ensure!(
        ACTORS.iter().any(|(v, _)| *v == id),
        "Preview supports the five recovered original infantry models"
    );
    ensure!(
        time.is_finite() && time >= 0.,
        "Preview time must be finite and nonnegative"
    );
    let start = std::time::Instant::now();
    let catalog = crate::authored_rules::load(store.root(), settings.memory_policy)?;
    let definitions = aa_animation::definitions::character(&catalog, id, P)?
        .context("No original Character definition")?;
    let visual = ActorVisual {
        actor_id: ID,
        native_position: [0.; 3],
        native_facing: [0., -1., 0.],
        scale: 1.,
        visible: true,
        team_color: Some([0.35, 0.7, 0.95]),
    };
    let spawn = spawn::prepare(
        store,
        &catalog,
        settings.clone(),
        SpawnActor {
            thing_id: id.into(),
            object_variant: -1,
            resolved_aspect: None,
            visual,
        },
        P,
    )?;
    let source = spawn.binding().model_path.clone();
    let bounds = store.load_model(&source)?.prepare()?.bounds;
    //45E690 selects the final/base animation set for the ordinary -1 variant.
    //The model's normal aspect index is a different selector.
    let effective_set = u32::try_from(definitions.aspect_ids.len())?;
    let rows = definitions.for_variant(effective_set)?;
    let mut rig = spawn.prepare_rig(store, &settings, epoch)?;
    let mut clips = Vec::new();
    for resource in RESOURCES {
        let path = format!("/Shared/Animations/Units/Infantry/{resource}.kf");
        let (row, definition) = rows
            .iter()
            .enumerate()
            .find(|(_, a)| a.kf_expression.eq_ignore_ascii_case(&path))
            .with_context(|| {
                format!("Original {id} aspect has no recovered source binding for {resource}")
            })?;
        let key = format!("preview:{id}:set{effective_set}:row{row}");
        rig.bind_clip(store, &key, definition)?;
        clips.push(Clip {
            key,
            definition: definition.clone(),
        });
    }
    let selected = if let Some(name) = clip {
        clips
            .iter()
            .position(|c| c.definition.id.eq_ignore_ascii_case(name))
            .with_context(|| format!("No recovered original clip named {name}"))?
    } else {
        0
    };
    let scene=CpuScene { objects:Some(SceneObjects::empty()?),live:Some(LiveScene::empty()),
        name:format!("Live actor preview · {id}"),map_id:source, size:[bounds.size[0],bounds.size[2]],
        draws:Vec::new(), textures:Default::default(),placements:1,rendered_instances:1,
        warnings:vec!["Isolated visual preview: local test identity and clock; no movement orders, combat, AI or gameplay simulation.".into(),
            "Original source KF controller modes determine loop/clamp. Blending and root motion are not implemented in this preview.".into()],load_seconds:start.elapsed().as_secs_f64() };
    Ok((
        Prepared {
            spawn,
            rig,
            clips,
            selected,
            epoch,
            visual,
            time,
        },
        scene,
        Camera::for_model(&bounds),
    ))
}
impl Prepared {
    pub fn activate(
        self,
        renderer: &mut SceneRenderer,
        live: &mut LiveScene,
        objects: &mut SceneObjects,
    ) -> Result<Active> {
        ensure!(
            renderer.scene_epoch() == self.epoch,
            "Preview preparation belongs to a different scene"
        );
        let source = self.spawn.binding().model_path.clone();
        let report = live.append(renderer, objects, self.spawn)?;
        let mut state = Character::new(
            aa_actors::ActorId::from_raw(ID),
            &[Some(self.clips[self.selected].key.clone())],
        );
        runtime::start_direct(
            &mut state,
            self.clips[self.selected].key.clone(),
            false,
            F32::ZERO,
        )?;
        let mut active = Active {
            rig: self.rig,
            clips: self.clips,
            selected: self.selected,
            state,
            visual: self.visual,
            time: 0.,
            sequence: 0,
            cache: ClockCache::default(),
            playing: true,
            dirty: true,
            source,
            notices: report.warnings,
            heading: 0.,
            position: [0.; 2],
        };
        active.tick(0., renderer, live, objects)?;
        if self.time > 0. {
            active.tick(self.time, renderer, live, objects)?;
        }
        Ok(active)
    }
}
impl Active {
    fn restart(&mut self) -> Result<()> {
        self.time = 0.;
        self.cache = ClockCache::default();
        self.rig.reset_controllers();
        runtime::start_direct(
            &mut self.state,
            self.clips[self.selected].key.clone(),
            false,
            F32::ZERO,
        )?;
        if !self.playing {
            runtime::pause(&mut self.state, F32::ZERO)?;
        }
        self.dirty = true;
        Ok(())
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {
        ui.separator();
        ui.strong("Live actor preview");
        ui.small("Visual test only. This is not a playable unit or battle.");
        let old = self.selected;
        egui::ComboBox::from_id_salt("live-clip")
            .selected_text(&self.clips[self.selected].definition.id)
            .show_ui(ui, |ui| {
                for (i, c) in self.clips.iter().enumerate() {
                    ui.selectable_value(&mut self.selected, i, &c.definition.id);
                }
            });
        if old != self.selected {
            self.restart()?;
        }
        let playing = self.playing;
        let mut restart = false;
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.playing, "Play");
            restart = ui.button("Restart clip").clicked();
        });
        if playing != self.playing {
            if self.playing {
                runtime::resume(&mut self.state, f(self.time), P)?;
            } else {
                runtime::pause(&mut self.state, f(self.time))?;
            }
            self.dirty = true;
        }
        if restart {
            self.restart()?;
        }
        ui.small(format!(
            "Preview clock {:.2} s · original controller loop/clamp",
            self.time
        ));
        self.dirty |= ui.checkbox(&mut self.visual.visible, "Visible").changed();
        ui.horizontal(|ui| {
            ui.label("Actor color");
            self.dirty |= ui
                .color_edit_button_rgb(self.visual.team_color.as_mut().unwrap())
                .changed();
        });
        ui.small("Manual visual transform (does not issue movement orders)");
        self.dirty |= ui
            .add(egui::Slider::new(&mut self.position[0], -5.0..=5.0).text("X"))
            .changed();
        self.dirty |= ui
            .add(egui::Slider::new(&mut self.position[1], -5.0..=5.0).text("Y"))
            .changed();
        self.dirty |= ui
            .add(egui::Slider::new(&mut self.heading, -180.0..=180.0).text("Facing"))
            .changed();
        if ui.button("Reset transform").clicked() {
            self.position = [0.; 2];
            self.heading = 0.;
            self.dirty = true;
        }
        ui.small(&self.source);
        if !self.notices.is_empty() {
            ui.collapsing(format!("Source model notes ({})", self.notices.len()), |ui| {
                for notice in &self.notices { ui.small(notice); }
            });
        }
        ui.small("Four recovered clips per faction. No blend transitions, root motion, gameplay effects or simulation.");
        Ok(())
    }
    pub fn tick(
        &mut self,
        elapsed: f32,
        renderer: &mut SceneRenderer,
        live: &mut LiveScene,
        objects: &mut SceneObjects,
    ) -> Result<()> {
        ensure!(
            elapsed.is_finite() && elapsed >= 0.,
            "Invalid preview frame time"
        );
        self.time += elapsed;
        //While paused the authoritative preview Character retains its pause time.
        //Skip redundant pose writes until a control changes or playback resumes.
        if !self.playing && !self.dirty {
            return Ok(());
        }
        let yaw = self.heading.to_radians();
        self.visual.native_position = [self.position[0], self.position[1], 0.];
        self.visual.native_facing = [yaw.sin(), -yaw.cos(), 0.];
        self.sequence = self
            .sequence
            .checked_add(1)
            .context("Preview sequence overflow")?;
        live.apply(renderer, objects, self.sequence, &[self.visual])?;
        self.rig.pose(
            live,
            renderer,
            objects,
            &self.state,
            f(self.time),
            &mut self.cache,
            P,
        )?;
        self.dirty = false;
        Ok(())
    }
    pub fn stop_on_error(&mut self) {
        self.playing = false;
        self.dirty = false;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    #[ignore = "requires original installation and graphics adapter via AA_GAME_ROOT"]
    fn live_preview_gui_path_spawns_all_factions_and_preserves_pause_visibility_and_restart() {
        let game =
            std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"));
        let store = AssetStore::open(game.join("reconstruction")).unwrap();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .unwrap();
        let egui_renderer =
            egui_wgpu::Renderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, Default::default());
        let mut renderer = SceneRenderer::new(egui_wgpu::RenderState {
            adapter,
            available_adapters: vec![],
            device,
            queue,
            target_format: wgpu::TextureFormat::Rgba8Unorm,
            renderer: Arc::new(egui::mutex::RwLock::new(egui_renderer)),
        })
        .unwrap();
        let settings =
            LoadSettings::new(&store, 4096, aa_resources::MemoryPolicy::default()).unwrap();
        let folder = std::env::temp_dir().join(format!(
            "aa-live-preview-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        for (id, _) in ACTORS {
            let (prepared, mut scene, camera) = prepare(
                &store,
                settings.clone(),
                id,
                renderer.scene_epoch() + 1,
                Some("jog"),
                0.,
            )
            .unwrap();
            assert_eq!(prepared.clips.len(), 4);
            assert_eq!(prepared.visual.scale, 1.);
            let mut live = scene.live.take().unwrap();
            let mut objects = scene.objects.take().unwrap();
            renderer.upload(&scene).unwrap();
            live.attach(renderer.scene_epoch()).unwrap();
            let mut active = prepared
                .activate(&mut renderer, &mut live, &mut objects)
                .unwrap();
            let bytes = renderer.uploaded_bytes;
            let capture = |renderer: &mut SceneRenderer, name: &str| {
                renderer.render(&camera, [512, 384]).unwrap();
                let path = folder.join(format!("{id}-{name}.png"));
                renderer.save_view(&path).unwrap();
                image::open(path).unwrap().to_rgba8()
            };
            let start = capture(&mut renderer, "start");
            active
                .tick(0.5, &mut renderer, &mut live, &mut objects)
                .unwrap();
            let frame = capture(&mut renderer, "frame");
            assert_ne!(start, frame);
            runtime::pause(&mut active.state, f(active.time)).unwrap();
            active.playing = false;
            active.dirty = true;
            active
                .tick(2., &mut renderer, &mut live, &mut objects)
                .unwrap();
            assert_eq!(frame, capture(&mut renderer, "paused"));
            active.visual.visible = false;
            active.dirty = true;
            active
                .tick(0., &mut renderer, &mut live, &mut objects)
                .unwrap();
            assert!(!objects.objects[0].visible);
            assert_ne!(frame, capture(&mut renderer, "hidden"));
            active.visual.visible = true;
            active.position = [2., -1.];
            active.heading = 90.;
            active.dirty = true;
            active
                .tick(0., &mut renderer, &mut live, &mut objects)
                .unwrap();
            assert_eq!(objects.objects[0].native_position, [2., -1., 0.]);
            active.position = [0.; 2];
            active.heading = 0.;
            active.restart().unwrap();
            active
                .tick(0., &mut renderer, &mut live, &mut objects)
                .unwrap();
            assert_eq!(start, capture(&mut renderer, "restart"));
            assert_eq!(bytes, renderer.uploaded_bytes);
            for selected in 0..4 {
                active.selected = selected;
                active.playing = true;
                active.restart().unwrap();
                active
                    .tick(0., &mut renderer, &mut live, &mut objects)
                    .unwrap();
                active
                    .tick(3., &mut renderer, &mut live, &mut objects)
                    .unwrap();
            }
            let ctx = egui::Context::default();
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| active.ui(ui).unwrap());
            });
            assert!(
                prepare(
                    &store,
                    settings.clone(),
                    "unknown_actor",
                    renderer.scene_epoch() + 1,
                    None,
                    0.
                )
                .is_err()
            );
            assert_eq!(bytes, renderer.uploaded_bytes);
        }
        println!("Preview captures: {}", folder.display());
    }
}
