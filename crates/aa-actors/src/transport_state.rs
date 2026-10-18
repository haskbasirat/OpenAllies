//! Original Transport component (slot 24, 0x60 bytes, vtable A40100):
//! constructor `58B9A0`, saved Load `58C810` (writer `58B380`). Oracle:
//! research/phase8-saved-components/native-transport.json (constructor then
//! Load under Unicorn, every read and registry lookup recorded).
//!
//! Unlike the Updatable components, Transport has no Updatable record: `+8`
//! and `+18` are the cached cost/upkeep ResourceVectors (`4CD440`, virtual
//! Load `4CD3F0` -> `4CD0F0`), `+28` the carried ActorData (`4C2E60`) and
//! `+2C` a reference-counted `KKC_OrganizationLayout` (`4C3050` LayoutData,
//! `4C2110`, virtual Load `4C3180`) describing the carried organization.
use crate::{
    ActorId,
    components::ComponentSlot,
    ground_states::{LayoutDefinition, ResourceVector},
    organization_state::{Cursor, Definitions, LoadError, Result, load_layout, read_actor_ref, read_layout_ref},
};
use aa_body::F32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportState<D> {
    /// `+4`.
    pub owner: ActorId,
    /// `+8` cost and `+18` upkeep ResourceVectors (vector at `+C`/`+1C`).
    pub cost: ResourceVector,
    pub upkeep: ResourceVector,
    /// `+28` carried ActorData.
    pub cargo_definition: Option<D>,
    /// `+2C` carried layout (null when the saved flag byte is zero).
    pub cargo_layout: Option<LayoutDefinition<D>>,
    /// `+30/+34`, constructor copy of the static zero vector B279C0/B279C4
    /// (no instruction writes either word; initialized .data zero).
    pub word_30: [u32; 2],
    pub byte_38: u8,
    pub byte_39: u8,
    pub word_3c: u32,
    pub word_40: u32,
    pub word_44: u32,
    pub word_48: u32,
    pub byte_4c: u8,
    /// `+50/+54`, loaded only when `+4C` is nonzero (version >= 3).
    pub words_50: [u32; 2],
    /// `+58` CString (constructor empty), version >= 5.
    pub text_58: Vec<u16>,
    pub word_5c: u32,
}

/// Component virtual `+20` of vtable A40100 is `7C7840` (`mov eax, 3`):
/// KKC_GActor::ValidateComponents `4CFB00` requires the component in slot 3,
/// the Mover; without it the actor fails validation with "Actor %s
/// component %d requires component %d which is missing" (404ED0, line
/// 0x5E8). Oracle: native-transport-requirement.json.
pub const REQUIRED_COMPONENT: ComponentSlot = ComponentSlot::Mover;

/// `4CD440`: `[B3A660]+28` registered resources; capacity 8 when zero, and
/// `45F180` sizes the vector to the count with zeros.
fn resource_vector(count: usize) -> ResourceVector {
    ResourceVector {
        values: vec![F32::ZERO; count],
        initial_capacity: if count == 0 { 8 } else { count },
    }
}

/// `4CD3F0` ResourceVector Load: version 0, then `4CD0F0` (signed count,
/// then one 4-byte read per value). The native stores a negative count as
/// the vector size; that state is refused.
fn load_resources(c: &mut Cursor<'_>, vector: &mut ResourceVector) -> Result<()> {
    c.version(0)?;
    let count = c.i32()?;
    let count = usize::try_from(count).map_err(|_| {
        LoadError::Unsupported(format!("negative ResourceVector count {count} (4CD133 stores it as the size)"))
    })?;
    // The size is stored before the values are read; entries not yet read
    // keep the previous buffer (constructor zeros) or uninitialized heap
    // (4849F0 growth), represented as zero.
    vector.values.resize(count, F32::ZERO);
    for i in 0..count {
        vector.values[i] = F32(c.u32()?);
    }
    Ok(())
}

impl<D: Clone> TransportState<D> {
    /// `58B9A0(owner)` with the registered resource count `[B3A660]+28`.
    pub fn new(owner: ActorId, registered_resource_count: usize) -> Self {
        Self {
            owner,
            cost: resource_vector(registered_resource_count),
            upkeep: resource_vector(registered_resource_count),
            cargo_definition: None,
            cargo_layout: None,
            word_30: [0; 2],
            byte_38: 0,
            byte_39: 0,
            word_3c: 0,
            word_40: 0,
            word_44: 0,
            word_48: 0,
            byte_4c: 0,
            words_50: [0; 2],
            text_58: Vec::new(),
            word_5c: 0,
        }
    }

    /// `58C810` Load. Returns the bytes consumed; on error every earlier
    /// store remains, as in the original.
    pub fn load(&mut self, bytes: &[u8], defs: &mut impl Definitions<D>) -> Result<usize> {
        let mut c = Cursor::new(bytes);
        let version = c.version(6)?;
        self.cargo_definition = read_actor_ref(&mut c, defs)?;
        let flag = c.u8()?;
        if flag != 0 {
            let data = read_layout_ref(&mut c, defs)?.ok_or(LoadError::NullDereference(0x4c2141))?;
            // 4C2110: +8 LayoutData, parameters sized to its +20 with nulls;
            // replaces (and releases) the previous +2C object.
            let info = defs.layout_info(&data)?;
            self.cargo_layout = Some(LayoutDefinition {
                definition: data,
                parameters: vec![None; info.parameter_count()],
                slots: info.slot_count(),
            });
            let layout = self.cargo_layout.as_mut().expect("just stored");
            load_layout(&mut c, layout, defs)?;
        } else {
            self.cargo_layout = None;
        }
        self.word_30 = c.pair()?;
        self.byte_38 = c.u8()?;
        self.byte_39 = c.u8()?;
        self.word_3c = c.u32()?;
        self.word_40 = c.u32()?;
        if version == 0 {
            let count = c.i32()?;
            for _ in 0..count.max(0) {
                c.u32()?;
            }
        }
        if version >= 4 {
            self.word_44 = c.u32()?;
        }
        self.word_48 = c.u32()?;
        load_resources(&mut c, &mut self.cost)?;
        load_resources(&mut c, &mut self.upkeep)?;
        if version >= 3 {
            self.byte_4c = c.u8()?;
            if self.byte_4c != 0 {
                self.words_50 = c.pair()?;
            }
        }
        if version >= 5 {
            self.text_58 = c.string()?;
        }
        if version >= 6 {
            self.word_5c = c.u32()?;
        }
        Ok(c.position())
    }
}

/// Callbacks of the payload load `58BA90`.
pub trait PayloadWorld<D> {
    /// `56F580(out, cargo, layout, 0)`: the cargo's upkeep vector, i.e.
    /// `economy_values::layout_values(.., VectorKind::Upkeep, ..)` over the
    /// cargo ActorData and the layout's slot definitions (null context).
    fn cargo_upkeep(&mut self, cargo: &D, layout: Option<&LayoutDefinition<D>>) -> Result<Vec<F32>>;
    /// Fact: LayoutData `+28` group index of every slot (`+2C` slots).
    fn slot_groups(&self, layout: &D) -> Result<Vec<u32>>;
    /// `4CF0F0` on the owner actor.
    fn refresh_owner(&mut self) -> Result<()>;
}

impl<D: Clone> TransportState<D> {
    /// `58BA90(cargo, name, layout, start, drop)`: load a payload (summon
    /// effect `4CCA70`, trigger actions). Two fresh ResourceVectors
    /// (`4CD440`); `56F580` fills the second with the cargo upkeep; `58B2A0`
    /// copies them to `+8` (cost: the fresh registered-count zeros) and `+18`
    /// (upkeep), sets `+28` cargo, `+2C` layout (reference swap), `+30/+34`
    /// drop point, `+38 = 0`, `+39 = 1`, `+40/+44/+48 = 0`, `+4C = 0`, `+3C`
    /// = number of layout slots whose group parameter is nonnull (1 without
    /// a layout), calls `4CF0F0` on the owner and returns true; `58BA90`
    /// then sets `+4C = 1`, `+50/+54` = start and `+58` = name. Returns the
    /// native result (always true: `58B2A0` has no failing path).
    #[allow(clippy::too_many_arguments)]
    pub fn load_payload(
        &mut self,
        cargo: D,
        name: &[u16],
        layout: Option<LayoutDefinition<D>>,
        start: [u32; 2],
        drop: [u32; 2],
        registered_resource_count: usize,
        world: &mut impl PayloadWorld<D>,
    ) -> Result<bool> {
        let cost = resource_vector(registered_resource_count);
        let mut upkeep = resource_vector(registered_resource_count);
        upkeep.values = world.cargo_upkeep(&cargo, layout.as_ref())?;
        // 58B2A0
        self.cost.values = cost.values;
        self.upkeep.values = upkeep.values;
        self.cargo_definition = Some(cargo);
        self.word_30 = drop;
        self.byte_38 = 0;
        self.byte_39 = 1;
        self.word_40 = 0;
        self.word_44 = 0;
        self.word_48 = 0;
        self.byte_4c = 0;
        self.word_3c = match &layout {
            None => 1,
            Some(l) => {
                let mut filled = 0u32;
                for group in world.slot_groups(&l.definition)? {
                    let param = l.parameters.get(group as usize).ok_or_else(|| {
                        LoadError::Unsupported(format!("layout group {group} beyond the parameter array"))
                    })?;
                    if param.is_some() {
                        filled += 1;
                    }
                }
                filled
            }
        };
        self.cargo_layout = layout;
        world.refresh_owner()?;
        // 58BA90 after a successful 58B2A0.
        self.byte_4c = 1;
        self.words_50 = start;
        self.text_58 = name.to_vec();
        Ok(true)
    }
}
