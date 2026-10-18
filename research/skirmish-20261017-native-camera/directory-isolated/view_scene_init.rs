//! Retained semantic scene for native43F948..43FCA4.
//!
//! Rust allocations replace Ni reference-counted allocations and linked-list
//! pools. The actual scene relationships, constructor values, transform
//! arithmetic and ordered writes are retained. This does not implement native
//! debug allocation counters, an SDK ABI, terrain, AD or view completion.
use crate::{
    view_grid_state::Globals,
    world_revision::{ConstructedWorldRevision, Witness},
};
use aa_numeric::{Binary, Precision};
use std::{
    cell::RefCell,
    cmp::Ordering,
    rc::{Rc, Weak},
    sync::{Arc, Weak as SyncWeak},
};
type Result<T> = std::result::Result<T, String>;
const ONE: u32 = 0x3f800000;

///Thirteen binary32 words: rotation9, translation3, scale1 (8751C0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transform([u32; 13]);
impl Transform {
    fn constructed(g: &mut impl Globals) -> Result<Self> {
        let mut w = [0; 13];
        for i in [0, 4, 8, 12] {
            w[i] = ONE;
        }
        for (i, a) in [0xb43cf8, 0xb43cfc, 0xb43d00].into_iter().enumerate() {
            w[9 + i] = g.word(a)?;
        }
        Ok(Self(w))
    }
    pub fn words(&self) -> &[u32; 13] {
        &self.0
    }
    ///A typed value, not a scene/readiness constructor. Camera math writes
    ///these same local fields through its source-proven setter.
    pub(crate) fn from_words(words: [u32; 13]) -> Result<Self> {
        for w in words {
            Binary::from_f32_bits(w).map_err(|e| e.to_string())?;
        }
        Ok(Self(words))
    }
}
fn b(w: u32) -> Result<Binary> {
    Binary::from_f32_bits(w).map_err(|e| e.to_string())
}
fn store(v: Binary) -> Result<u32> {
    v.store_f32().map_err(|e| e.to_string())
}
fn product(a: u32, c: u32, p: Precision) -> Result<Binary> {
    b(a)?.mul(&b(c)?, p).map_err(|e| e.to_string())
}
fn add(a: Binary, c: Binary, p: Precision) -> Result<Binary> {
    a.add(&c, p).map_err(|e| e.to_string())
}
///45CAB0/870590/870720, including nonuniform native summation order and
///the intermediate binary32 stores of translation Y/Z before addition.
fn compose(parent: Transform, local: Transform, p: Precision) -> Result<Transform> {
    let a = parent.0;
    let c = local.0;
    let mut out = [0; 13];
    out[12] = store(product(c[12], a[12], p)?)?;
    const ORDERS: [[usize; 3]; 9] = [
        [1, 0, 2],
        [2, 0, 1],
        [2, 0, 1],
        [2, 1, 0],
        [2, 1, 0],
        [2, 1, 0],
        [2, 0, 1],
        [0, 2, 1],
        [0, 2, 1],
    ];
    for r in 0..3 {
        for col in 0..3 {
            let [i, j, k] = ORDERS[r * 3 + col];
            out[r * 3 + col] = store(add(
                add(
                    product(a[r * 3 + i], c[i * 3 + col], p)?,
                    product(a[r * 3 + j], c[j * 3 + col], p)?,
                    p,
                )?,
                product(a[r * 3 + k], c[k * 3 + col], p)?,
                p,
            )?)?;
        }
    }
    for r in 0..3 {
        let [i, j, k] = if r == 0 { [2, 1, 0] } else { [2, 0, 1] };
        let rotated = store(add(
            add(
                product(a[r * 3 + i], c[9 + i], p)?,
                product(a[r * 3 + j], c[9 + j], p)?,
                p,
            )?,
            product(a[r * 3 + k], c[9 + k], p)?,
            p,
        )?)?;
        let scaled = product(a[12], rotated, p)?;
        let scaled = if r == 0 { scaled } else { b(store(scaled)?)? };
        out[9 + r] = store(add(scaled, b(a[9 + r])?, p)?)?;
    }
    Ok(Transform(out))
}

struct NodeFields {
    local: Transform,
    world: Transform,
    parent: Weak<RefCell<NodeFields>>,
    children: Vec<Child>,
    properties: Vec<Rc<RefCell<TexturingProperty>>>,
    effects: Vec<Weak<RefCell<DirectionalLight>>>,
}
enum Child {
    Node(Rc<RefCell<NodeFields>>),
    Light(Rc<RefCell<DirectionalLight>>),
}
///One867690 NiNode semantic allocation. There is no public Clone or raw-ID
///constructor; parent/child loans preserve the same allocation.
pub struct Node(Rc<RefCell<NodeFields>>);
impl Node {
    pub fn from_native_constructor(g: &mut impl Globals) -> Result<Self> {
        let local = Transform::constructed(g)?;
        let world = Transform::constructed(g)?;
        //86F76D..78E: independent bounding-center global reads.
        for a in [0xb43cf8, 0xb43cfc, 0xb43d00] {
            g.word(a)?;
        }
        Ok(Self(Rc::new(RefCell::new(NodeFields {
            local,
            world,
            parent: Weak::new(),
            children: Vec::new(),
            properties: Vec::new(),
            effects: Vec::new(),
        }))))
    }
    pub fn local_transform(&self) -> Transform {
        self.0.borrow().local
    }
    pub fn world_transform(&self) -> Transform {
        self.0.borrow().world
    }
    ///Readonly identity of the actual retained parent relation, used by the
    ///current native camera render binding; equal transforms are insufficient.
    pub(crate) fn is_parent_of(&self,child:&Node)->bool{
        child.0.borrow().parent.upgrade().is_some_and(|p|Rc::ptr_eq(&p,&self.0))
    }
    pub fn child_count(&self) -> usize {
        self.0.borrow().children.len()
    }
    pub fn property_count(&self) -> usize {
        self.0.borrow().properties.len()
    }
    pub fn effect_count(&self) -> usize {
        self.0.borrow().effects.len()
    }
    pub fn has_parent(&self) -> bool {
        self.0.borrow().parent.upgrade().is_some()
    }
    pub(crate) fn set_local_transform(&mut self, value: Transform) {
        self.0.borrow_mut().local = value;
    }
    ///The concrete current-camera constructor/update may write its own world
    ///transform. This does not create/replace a scene owner or completion mark.
    pub(crate) fn set_world_transform(&mut self, value: Transform) {
        self.0.borrow_mut().world = value;
    }
    pub fn attach_node(&mut self, child: &mut Node) -> Result<()> {
        if Rc::ptr_eq(&self.0, &child.0) {
            return Err("NiNode cannot parent itself".into());
        }
        if child.0.borrow().parent.upgrade().is_some() {
            return Err("existing NiNode parent requires native detach binding".into());
        }
        child.0.borrow_mut().parent = Rc::downgrade(&self.0);
        self.0
            .borrow_mut()
            .children
            .push(Child::Node(Rc::clone(&child.0)));
        Ok(())
    }
    pub(crate) fn update_world(&mut self, p: Precision) -> Result<()> {
        update_node(&self.0, p)
    }
}
fn update_node(node: &Rc<RefCell<NodeFields>>, p: Precision) -> Result<()> {
    let parent = node.borrow().parent.upgrade().map(|v| v.borrow().world);
    update_node_parent(node, parent, p)
}
fn update_node_parent(
    node: &Rc<RefCell<NodeFields>>,
    parent: Option<Transform>,
    p: Precision,
) -> Result<()> {
    let mut n = node.borrow_mut();
    n.world = if let Some(parent) = parent {
        compose(parent, n.local, p)?
    } else {
        n.local
    };
    let parent = n.world;
    for child in &n.children {
        match child {
            Child::Node(child) => update_node_parent(child, Some(parent), p)?,
            Child::Light(light) => {
                let mut l = light.borrow_mut();
                l.world = compose(parent, l.local, p)?;
                l.direction = [l.world.0[0], l.world.0[3], l.world.0[6]];
            }
        }
    }
    Ok(())
}
pub struct WireframeProperty {
    flags: u16,
}
impl WireframeProperty {
    pub fn enabled(&self) -> bool {
        self.flags & 1 != 0
    }
}
pub struct TexturingProperty {
    apply_mode: u32,
    slots: [Option<TextureMap>; 7],
}
pub struct TextureMap {
    filter: u32,
}
impl TexturingProperty {
    fn constructed() -> Self {
        Self {
            apply_mode: 2,
            slots: std::array::from_fn(|_| None),
        }
    }
    pub fn apply_mode_word(&self) -> u32 {
        self.apply_mode
    }
    pub fn base_filter_word(&self) -> Option<u32> {
        self.slots[0].as_ref().map(|v| v.filter)
    }
}
struct DirectionalLight {
    local: Transform,
    world: Transform,
    direction: [u32; 3],
    dimmer: u32,
    colors: [[u32; 3]; 3],
}
///89B510 NiLight constructor values, retained with the same light allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightInfo {
    pub direction: [u32; 3],
    pub dimmer: u32,
    pub ambient: [u32; 3],
    pub diffuse: [u32; 3],
    pub specular: [u32; 3],
}
struct LightContext {
    nesting: u32,
    flags: u32,
    light: Rc<RefCell<DirectionalLight>>,
}
pub struct TextureEffect {
    local: Transform,
    world: Transform,
    source_texture: Option<()>,
    clamp: u32,
    filter: u32,
    kind: u32,
    enabled: u8,
}
impl TextureEffect {
    fn constructed(g: &mut impl Globals) -> Result<Self> {
        Ok(Self {
            local: Transform::constructed(g)?,
            world: Transform::constructed(g)?,
            source_texture: None,
            clamp: 3,
            filter: 0,
            kind: 0,
            enabled: 1,
        })
    }
    pub fn source_texture_present(&self) -> bool {
        self.source_texture.is_some()
    }
    pub fn filter_word(&self) -> u32 {
        self.filter
    }
    pub fn clamp_word(&self) -> u32 {
        self.clamp
    }
    pub fn kind_word(&self) -> u32 {
        self.kind
    }
    pub fn enabled(&self) -> bool {
        self.enabled != 0
    }
    pub fn world_transform(&self) -> Transform {
        self.world
    }
}
///Only the actual current camera implementation can provide this loan in the
///product path. The coordinator never substitutes an inspection camera.
pub trait Camera {
    fn reset_current_7010d0(&mut self) -> Result<()>;
    ///Read the current camera AGAIN after Reset, then lend its real+164 node.
    fn current_root_164(&mut self) -> Result<&mut Node>;
}
#[derive(Clone)]
pub struct SceneWitness {
    identity: SyncWeak<()>,
    world: Witness,
}
impl SceneWitness {
    pub fn is_for(&self, other: &Self) -> bool {
        SyncWeak::ptr_eq(&self.identity, &other.identity) && self.identity.upgrade().is_some()
    }
    pub fn require_world(&self, world: &ConstructedWorldRevision) -> Result<()> {
        if self.identity.upgrade().is_some() && self.world.is_for(world) {
            Ok(())
        } else {
            Err("scene renderer witness belongs to a dead/replaced World".into())
        }
    }
}
///A concrete backend stores its same renderer-allocation/epoch witness here.
///This trait grants no AD, loading or application activation capability.
pub trait RendererBinding {
    fn validate(
        &self,
        scene: &SceneWitness,
        world: &ConstructedWorldRevision,
        current: &State,
    ) -> Result<()>;
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Constructed,
    Initializing,
    CpuComplete,
    Failed,
}
pub struct State {
    identity: Arc<()>,
    world: Option<Witness>,
    root: Option<Node>,
    wireframe: Option<WireframeProperty>,
    texturing: Option<Rc<RefCell<TexturingProperty>>>,
    light: Option<LightContext>,
    effect: Option<TextureEffect>,
    planar_direction: Option<[u32; 2]>,
    phase: Phase,
    renderer: Option<Box<dyn RendererBinding>>,
}
impl State {
    ///The five native442C00 pointer fields start null. No whole constructor or
    ///view initialization is inferred from this bounded field projection.
    pub fn from_constructor_fields() -> Self {
        Self {
            identity: Arc::new(()),
            world: None,
            root: None,
            wireframe: None,
            texturing: None,
            light: None,
            effect: None,
            planar_direction: None,
            phase: Phase::Constructed,
            renderer: None,
        }
    }
    pub fn initialize(
        &mut self,
        world: &ConstructedWorldRevision,
        g: &mut impl Globals,
        camera: &mut impl Camera,
        p: Precision,
    ) -> Result<()> {
        if self.phase != Phase::Constructed {
            return Err("scene segment requires fresh constructor fields; native teardown is required for retry".into());
        }
        self.world = Some(world.witness());
        self.phase = Phase::Initializing;
        let r = self.initialize_inner(g, camera, p);
        self.phase = if r.is_ok() {
            Phase::CpuComplete
        } else {
            Phase::Failed
        };
        r
    }
    fn initialize_inner(
        &mut self,
        g: &mut impl Globals,
        camera: &mut impl Camera,
        p: Precision,
    ) -> Result<()> {
        self.root = Some(Node::from_native_constructor(g)?);
        self.wireframe = Some(WireframeProperty { flags: 0 }); //Cached, never attached here.
        let tex = Rc::new(RefCell::new(TexturingProperty::constructed()));
        self.texturing = Some(Rc::clone(&tex));
        self.root
            .as_mut()
            .unwrap()
            .0
            .borrow_mut()
            .properties
            .insert(0, Rc::clone(&tex));
        tex.borrow_mut().slots[0] = Some(TextureMap { filter: 2 });
        let local = Transform::constructed(g)?;
        let world = Transform::constructed(g)?;
        for a in [0xb43cf8, 0xb43cfc, 0xb43d00] {
            g.word(a)?;
        }
        let light = Rc::new(RefCell::new(DirectionalLight {
            local,
            world,
            direction: [ONE, 0, 0],
            dimmer: ONE,
            colors: [[ONE; 3]; 3],
        }));
        self.light = Some(LightContext {
            nesting: 0,
            flags: 0,
            light: Rc::clone(&light),
        });
        let context = self.light.as_mut().unwrap();
        context.nesting = context.nesting.wrapping_add(1);
        context.flags |= 1;
        context.flags |= 2;
        let root = self.root.as_mut().unwrap();
        root.0
            .borrow_mut()
            .children
            .push(Child::Light(Rc::clone(&light)));
        root.0.borrow_mut().effects.insert(0, Rc::downgrade(&light));
        camera.reset_current_7010d0()?;
        camera.current_root_164()?.attach_node(root)?;
        root.0.borrow_mut().local.0[9..12].fill(0);
        root.update_world(p)?;
        self.planar_direction = Some(planar(light.borrow().direction, p)?);
        if self.effect.is_none() {
            self.effect = Some(TextureEffect::constructed(g)?);
            let effect = self.effect.as_mut().unwrap();
            effect.source_texture = None;
            effect.filter = 2;
            effect.enabled = 1;
            effect.world = effect.local;
        }
        Ok(())
    }
    pub fn root(&self) -> Option<&Node> {
        self.root.as_ref()
    }
    pub fn wireframe(&self) -> Option<&WireframeProperty> {
        self.wireframe.as_ref()
    }
    pub fn texturing(&self) -> Option<std::cell::Ref<'_, TexturingProperty>> {
        self.texturing.as_ref().map(|v| v.borrow())
    }
    pub fn texture_effect(&self) -> Option<&TextureEffect> {
        self.effect.as_ref()
    }
    pub fn light_direction(&self) -> Option<[u32; 3]> {
        self.light.as_ref().map(|v| v.light.borrow().direction)
    }
    pub fn light_info(&self) -> Option<LightInfo> {
        self.light.as_ref().map(|v| {
            let l = v.light.borrow();
            LightInfo {
                direction: l.direction,
                dimmer: l.dimmer,
                ambient: l.colors[0],
                diffuse: l.colors[1],
                specular: l.colors[2],
            }
        })
    }
    pub fn planar_light_direction(&self) -> Option<[u32; 2]> {
        self.planar_direction
    }
    pub fn witness(&self) -> Result<SceneWitness> {
        Ok(SceneWitness {
            identity: Arc::downgrade(&self.identity),
            world: self
                .world
                .as_ref()
                .ok_or("scene has no World owner")?
                .clone(),
        })
    }
    pub fn require_cpu_complete(&self, world: &ConstructedWorldRevision) -> Result<()> {
        self.witness()?.require_world(world)?;
        if self.phase == Phase::CpuComplete {
            Ok(())
        } else {
            Err("native scene CPU segment has not completed".into())
        }
    }
    pub fn install_renderer(
        &mut self,
        world: &ConstructedWorldRevision,
        binding: Box<dyn RendererBinding>,
    ) -> std::result::Result<(), (Box<dyn RendererBinding>, String)> {
        let valid = (|| {
            self.require_cpu_complete(world)?;
            if self.renderer.is_some() {
                return Err("scene renderer is already retained".into());
            }
            binding.validate(&self.witness()?, world, self)
        })();
        if let Err(e) = valid {
            return Err((binding, e));
        }
        self.renderer = Some(binding);
        Ok(())
    }
    pub fn require_renderer(&self, world: &ConstructedWorldRevision) -> Result<()> {
        self.require_cpu_complete(world)?;
        self.renderer
            .as_ref()
            .ok_or("real scene renderer binding is missing")?
            .validate(&self.witness()?, world, self)
    }
}
fn planar(direction: [u32; 3], p: Precision) -> Result<[u32; 2]> {
    let y2 = product(direction[1], direction[1], p)?;
    let x2 = product(direction[0], direction[0], p)?;
    let length = add(y2, x2, p)?.sqrt(p).map_err(|e| e.to_string())?;
    //Exact native A886A8 threshold, pinned by the fixture producer.
    if length.compare(&b(0x38d1b717)?) != Ordering::Greater {
        return Ok([0, 0]);
    }
    let reciprocal = b(ONE)?.div(&length, p).map_err(|e| e.to_string())?;
    Ok([
        store(
            b(direction[0])?
                .mul(&reciprocal, p)
                .map_err(|e| e.to_string())?,
        )?,
        store(
            reciprocal
                .mul(&b(direction[1])?, p)
                .map_err(|e| e.to_string())?,
        )?,
    ])
}
