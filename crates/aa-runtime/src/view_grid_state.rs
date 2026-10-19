//! KKC_VGrid430290, retained by VWorld+94. This is a view-spatial owner,
//! not the keyboard/input controller. Native unwritten cells remain unknown.
use aa_numeric::{Binary, Precision};
use std::marker::PhantomData;
pub type Result<T> = std::result::Result<T, String>;

///The real process-global producer must supply these reached reads. A missing
///global owner is an error, never a zero-vector/default geometry substitution.
pub trait Globals {
    fn word(&mut self, address: u32) -> Result<u32>;
}
#[path = "view_grid_occupancy.rs"]
mod occupancy;
#[path = "view_grid_cell_views.rs"]
mod cell_views;
pub use occupancy::ViewFields;
enum Word {
    Unwritten,
    Bits(u32),
    GridAllocation,
}
pub struct Cell<V> {
    head: Option<NodeId>,
    tail: Option<NodeId>,
    views: PhantomData<V>,
}
impl<V> Cell<V> {
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }
    pub fn head(&self) -> Option<NodeId> {
        self.head
    }
    pub fn tail(&self) -> Option<NodeId> {
        self.tail
    }
}
///Logical identity of one actual cell-list node; never a native address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeId(usize);
struct Node<V> {
    view: Option<V>,
    next: Option<Option<NodeId>>,
}

///One native allocation. The same value survives constructor/init failures.
///No Clone/Default/Deserialize or public raw-word constructor is provided.
pub struct State<V> {
    words: [Word; 68],
    cells: Option<Vec<Cell<V>>>,
    nodes: Vec<Option<Node<V>>>,
    free_nodes: Vec<NodeId>,
    views: PhantomData<V>,
}
pub struct Failure<V> {
    pub state: State<V>,
    pub error: String,
}
impl<V> std::fmt::Debug for Failure<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VGridFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
impl<V> State<V> {
    fn allocation() -> Self {
        Self {
            words: std::array::from_fn(|_| Word::Unwritten),
            cells: None,
            nodes: Vec::new(),
            free_nodes: Vec::new(),
            views: PhantomData,
        }
    }
    fn store(&mut self, offset: u32, value: u32) -> Result<()> {
        let slot = self
            .words
            .get_mut((offset / 4) as usize)
            .filter(|_| offset % 4 == 0)
            .ok_or("VGrid DWORD store outside allocation")?;
        *slot = Word::Bits(value);
        Ok(())
    }
    ///A native null pointer is exposed only as0. A live portable allocation is
    ///not converted into a guessed32-bit native pointer or logical fake address.
    pub fn word(&self, offset: u32) -> Result<Option<u32>> {
        let word = self
            .words
            .get((offset / 4) as usize)
            .filter(|_| offset % 4 == 0)
            .ok_or("VGrid DWORD read outside allocation")?;
        match word {
            Word::Unwritten => Ok(None),
            Word::Bits(v) => Ok(Some(*v)),
            Word::GridAllocation => {
                Err("VGrid+0 is an owned allocation, not a raw native DWORD".into())
            }
        }
    }
    pub fn grid_present(&self) -> Result<bool> {
        match &self.words[0] {
            Word::Unwritten => Err("VGrid grid pointer is uninitialized".into()),
            Word::Bits(0) => Ok(false),
            Word::GridAllocation => Ok(true),
            _ => Err("VGrid grid-pointer representation differs".into()),
        }
    }
    pub fn dimensions(&self) -> Result<[i32; 2]> {
        Ok([self.required(0x1c)? as i32, self.required(0x20)? as i32])
    }
    pub fn extent_bits(&self) -> Result<[u32; 2]> {
        Ok([self.required(0x14)?, self.required(0x18)?])
    }
    pub fn coordinate_shift(&self) -> Result<u32> {
        self.required(0x24)
    }
    pub fn cells(&self) -> Result<&[Cell<V>]> {
        self.cells
            .as_deref()
            .ok_or("VGrid has no initialized cell allocation".into())
    }
    fn required(&self, offset: u32) -> Result<u32> {
        self.word(offset)?
            .ok_or_else(|| format!("VGrid+{offset:X} has not been written"))
    }
    pub fn from_constructor(globals: &mut impl Globals) -> std::result::Result<Self, Failure<V>> {
        let mut state = Self::allocation();
        let result = construct(&mut Constructor {
            state: &mut state,
            globals,
        });
        match result {
            Ok(()) => Ok(state),
            Err(error) => Err(Failure { state, error }),
        }
    }
    ///The exact World+3C/+40 value owner is the source (not ceiled GMap extents
    ///or final selected Creator). Shift belongs to43FD1x's current UVar path.
    pub fn initialize_from_world(
        &mut self,
        world: &crate::world_creator_extents::State,
        shift: u32,
        pc: Precision,
    ) -> Result<()> {
        self.initialize_bits(world.bits(), shift, pc)
    }
    fn initialize_bits(&mut self, extents: [u32; 2], shift: u32, pc: Precision) -> Result<()> {
        if self.grid_present()? {
            return Err(
                "VGrid repeated42F540 allocation requires native old-allocation lifetime binding"
                    .into(),
            );
        }
        initialize(&mut Initializer { state: self }, extents, shift, pc)
    }
}
pub(crate) trait ConstructorWorld {
    fn global(&mut self, address: u32) -> Result<u32>;
    fn store(&mut self, offset: u32, value: u32) -> Result<()>;
}
///4302B2..315 /869C30: the original read/store sequence, including six
///independent reads of each point global.28..9F is deliberately unwritten.
pub(crate) fn construct(world: &mut impl ConstructorWorld) -> Result<()> {
    for offset in [4, 8, 12, 16] {
        world.store(offset, 0)?;
    }
    let width = world.global(0xb43dbc)?;
    world.store(0x14, width)?;
    let height = world.global(0xb43dc0)?;
    world.store(0x18, height)?;
    for index in 0..6 {
        for (axis, address) in [0xb43cf8, 0xb43cfc, 0xb43d00].into_iter().enumerate() {
            let value = world.global(address)?;
            world.store(0xa0 + 16 * index + 4 * axis as u32, value)?;
        }
        world.store(0xac + 16 * index, 0)?;
    }
    for offset in [0x100, 0x104, 0x108, 0x10c, 0, 0x1c, 0x20, 0x24] {
        world.store(offset, 0)?;
    }
    Ok(())
}
struct Constructor<'a, V, G> {
    state: &'a mut State<V>,
    globals: &'a mut G,
}
impl<V, G: Globals> ConstructorWorld for Constructor<'_, V, G> {
    fn global(&mut self, address: u32) -> Result<u32> {
        self.globals.word(address)
    }
    fn store(&mut self, offset: u32, value: u32) -> Result<()> {
        self.state.store(offset, value)
    }
}
pub(crate) trait InitWorld {
    fn store(&mut self, offset: u32, value: u32) -> Result<()>;
    fn allocate_zero_cells(&mut self, dimensions: [i32; 2]) -> Result<()>;
}
fn dimension(bits: u32, shift: u32) -> Result<i32> {
    let value = Binary::from_f32_bits(bits).map_err(|e| e.to_string())?;
    let mut ceiling = value.truncate_i32().map_err(|e| e.to_string())?;
    if value.compare(&Binary::from_i32(ceiling)) == std::cmp::Ordering::Greater {
        ceiling = ceiling
            .checked_add(1)
            .ok_or("VGrid native ceil integer is outside admitted domain")?;
    }
    let shift = shift & 31;
    let cell = 1i32.wrapping_shl(shift);
    let aligned = ceiling.wrapping_sub(1).wrapping_add(cell) & !cell.wrapping_sub(1);
    Ok(aligned >> shift)
}
///42F540 uses native ceil then96CC4C,32-bit wrapping align arithmetic and
///arithmetic shift. No FP arithmetic depends on PC for admitted binary32 bits.
pub(crate) fn initialize(
    world: &mut impl InitWorld,
    extents: [u32; 2],
    shift: u32,
    _pc: Precision,
) -> Result<()> {
    world.store(0x14, extents[0])?;
    world.store(0x18, extents[1])?;
    world.store(0x24, shift)?;
    let width = dimension(extents[0], shift)?;
    world.store(0x1c, width as u32)?;
    let height = dimension(extents[1], shift)?;
    world.store(0x20, height as u32)?;
    world.allocate_zero_cells([width, height])
}
struct Initializer<'a, V> {
    state: &'a mut State<V>,
}
impl<V> InitWorld for Initializer<'_, V> {
    fn store(&mut self, offset: u32, value: u32) -> Result<()> {
        self.state.store(offset, value)
    }
    fn allocate_zero_cells(&mut self, [width, height]: [i32; 2]) -> Result<()> {
        let width = usize::try_from(width).map_err(|_| "VGrid negative native grid width")?;
        let height = usize::try_from(height).map_err(|_| "VGrid negative native grid height")?;
        let count = width
            .checked_mul(height)
            .ok_or("VGrid allocation dimensions overflow")?;
        let mut cells = Vec::new();
        cells.try_reserve_exact(count).map_err(|e| e.to_string())?;
        cells.resize_with(count, || Cell {
            head: None,
            tail: None,
            views: PhantomData,
        });
        self.state.cells = Some(cells);
        self.state.words[0] = Word::GridAllocation;
        Ok(())
    }
}

#[cfg(test)]
#[path = "view_grid_state_tests.rs"]
mod tests;
