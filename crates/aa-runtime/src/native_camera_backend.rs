//! Render-only transport sealed behind the actual current native camera.
//! The private producer is called only by the normalized current Directory
//! after same-World scene/root and actual renderer/target checks. A bare
//! partial Camera State cannot mint this.
use crate::world_revision::{ConstructedWorldRevision, Witness};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Weak,
};

///A sealed backend target implementation must validate its actual retained
///renderer/target allocation and epoch. This never confers view readiness.
pub trait Viewport: Send + Sync + 'static {
    fn pixels(&self) -> [u32; 2];
    fn require_current(&self) -> Result<(), String>;
}
pub trait ViewportFor<R>: Viewport {
    fn require_device(&self, device: &R) -> Result<(), String>;
}
///Immutable render words and nonowning correlation metadata. This is not a
///view-completion or simulation-ready capability and keeps no World alive.
///Private fields/no raw constructor prevent renderer-side fabrication.
pub struct Snapshot {
    camera_revision: Weak<AtomicU64>,
    observed_revision: u64,
    world: Witness,
    scene: crate::view_scene_init::SceneWitness,
    matrix: [u32; 16],
    eye: [u32; 3],
    direction: [u32; 3],
    viewport: [u32; 2],
    normalized_viewport: [u32; 4],
    backend: Arc<dyn Viewport>,
}
impl Snapshot {
    pub(super) fn from_normalized_current(
        camera: &super::State,
        world: &ConstructedWorldRevision,
        scene: crate::view_scene_init::SceneWitness,
        backend: Arc<dyn Viewport>,
    ) -> Result<Self, String> {
        backend.require_current()?;
        let viewport = backend.pixels();
        if viewport.contains(&0) {
            return Err("native camera backend viewport is empty".into());
        }
        fn words<const N: usize>(camera: &super::State, start: u32) -> Result<[u32; N], String> {
            let mut words = [0; N];
            for (i, w) in words.iter_mut().enumerate() {
                *w = camera.word(start + 4 * i as u32)?;
            }
            Ok(words)
        }
        let matrix = words(camera, 0xc8)?;
        let eye = words(camera, 0x90)?;
        let direction = words(camera, 0x114)?;
        let normalized_viewport = words(camera, 0x154)?;
        Ok(Self {
            camera_revision: Arc::downgrade(&camera.revision),
            observed_revision: camera.revision.load(Ordering::Acquire),
            world: world.witness(),
            scene,
            matrix,
            eye,
            direction,
            viewport,
            normalized_viewport,
            backend,
        })
    }
    pub fn matrix_words(&self) -> [u32; 16] {
        self.matrix
    }
    pub fn eye_words(&self) -> [u32; 3] {
        self.eye
    }
    pub fn direction_words(&self) -> [u32; 3] {
        self.direction
    }
    pub fn viewport_pixels(&self) -> [u32; 2] {
        self.viewport
    }
    pub fn normalized_viewport_words(&self) -> [u32; 4] {
        self.normalized_viewport
    }
    ///The same retained camera must still exist without an intervening store.
    pub fn require_current(&self) -> Result<(), String> {
        self.backend.require_current()?;
        if !self.scene.is_for(&self.scene) {
            return Err("native camera scene allocation was destroyed".into());
        }
        if self.backend.pixels() != self.viewport {
            return Err("native camera viewport was replaced/resized".into());
        }
        if self.world.owner_identity().upgrade().is_none() {
            return Err("native camera render World allocation is destroyed".into());
        }
        let current = self
            .camera_revision
            .upgrade()
            .ok_or("native current camera allocation is destroyed")?;
        if current.load(Ordering::Acquire) != self.observed_revision {
            return Err("native camera snapshot was superseded by a camera store".into());
        }
        Ok(())
    }
    pub fn require_world(&self, world: &ConstructedWorldRevision) -> Result<(), String> {
        self.require_current()?;
        if !self.world.is_for(world) {
            return Err("native camera render binding belongs to another World allocation".into());
        }
        Ok(())
    }
}
