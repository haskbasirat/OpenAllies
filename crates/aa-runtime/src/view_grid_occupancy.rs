//!42FD10/42F860 and430E20/430410 on the same VGrid cell/node allocation.
use super::*;

///The actual VThing scalar owner, never Thing GTIDs or render/picking copies.
pub trait ViewFields<V: Copy + Eq> {
    fn handle(&self) -> V;
    fn word(&mut self, offset: u32) -> Result<u32>;
    fn store(&mut self, offset: u32, value: u32) -> Result<()>;
    fn residual_occupancy_warning(&mut self, _count: i32) -> Result<()> {
        Err("required430587 residual VThing occupancy diagnostic is unbound".into())
    }
}
pub(super) trait World {
    type View: Copy + Eq;
    type Node: Copy + Eq;
    fn view(&self) -> Self::View;
    fn view_word(&mut self, offset: u32) -> Result<u32>;
    fn view_store(&mut self, offset: u32, value: u32) -> Result<()>;
    fn grid_word(&mut self, offset: u32) -> Result<u32>;
    fn head(&mut self, cell: usize) -> Result<Option<Self::Node>>;
    fn tail(&mut self, cell: usize) -> Result<Option<Self::Node>>;
    fn head_store(&mut self, cell: usize, node: Option<Self::Node>) -> Result<()>;
    fn tail_store(&mut self, cell: usize, node: Option<Self::Node>) -> Result<()>;
    fn allocate(&mut self) -> Result<Self::Node>;
    fn node_view(&mut self, node: Self::Node) -> Result<Self::View>;
    fn node_next(&mut self, node: Self::Node) -> Result<Option<Self::Node>>;
    fn node_view_store(&mut self, node: Self::Node, view: Self::View) -> Result<()>;
    fn node_next_store(&mut self, node: Self::Node, next: Option<Self::Node>) -> Result<()>;
    fn free(&mut self, node: Self::Node) -> Result<()>;
    fn residual_warning(&mut self, count: i32) -> Result<()>;
}
fn scalar(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| e.to_string())
}
fn integer(value: std::result::Result<Binary, aa_numeric::Error>, shift: u32) -> Result<i32> {
    Ok(value
        .map_err(|e| e.to_string())?
        .truncate_i32()
        .map_err(|e| e.to_string())?
        >> (shift & 31))
}
fn clip(value: i32, dimension: i32) -> i32 {
    if value < 0 {
        0
    } else if value > dimension.wrapping_sub(1) {
        dimension.wrapping_sub(1)
    } else {
        value
    }
}
///The wrappers read three scalar fields once; the callee reads shift and four
///dimension words lazily. X is the outer cell loop, Y the inner loop.
fn rectangle(world: &mut impl World, pc: Precision) -> Result<[i32; 4]> {
    let x = scalar(world.view_word(0x38)?)?;
    let y = scalar(world.view_word(0x3c)?)?;
    let radius = scalar(world.view_word(0x74)?)?;
    let shift = world.grid_word(0x24)?;
    let mut x0 = integer(x.sub(&radius, pc), shift)?;
    let mut x1 = integer(x.add(&radius, pc), shift)?;
    let mut y0 = integer(y.sub(&radius, pc), shift)?;
    let mut y1 = integer(y.add(&radius, pc), shift)?;
    x0 = clip(x0, world.grid_word(0x1c)? as i32);
    x1 = clip(x1, world.grid_word(0x1c)? as i32);
    y0 = clip(y0, world.grid_word(0x20)? as i32);
    y1 = clip(y1, world.grid_word(0x20)? as i32);
    Ok([x0.min(x1), y0.min(y1), x1, y1])
}
fn cell(world: &mut impl World, x: i32, y: i32) -> Result<usize> {
    let width = world.grid_word(0x1c)? as i32;
    usize::try_from(width.wrapping_mul(y).wrapping_add(x))
        .map_err(|_| "VGrid native cell index is outside the admitted allocation".into())
}
pub(super) fn occupy(world: &mut impl World, pc: Precision) -> Result<()> {
    let [x0, y0, x1, y1] = rectangle(world, pc)?;
    let view = world.view();
    let mut count = 0i32;
    for x in x0..=x1 {
        count = count.wrapping_add(y1.wrapping_sub(y0).wrapping_add(1));
        for y in y0..=y1 {
            let cell = cell(world, x, y)?;
            let next = world.head(cell)?;
            let node = world.allocate()?;
            world.node_view_store(node, view)?;
            world.node_next_store(node, next)?;
            let tail = world.tail(cell)?;
            world.head_store(cell, Some(node))?;
            if tail.is_none() {
                world.tail_store(cell, Some(node))?;
            }
        }
    }
    let flags = world.view_word(0x54)?;
    world.view_store(0x54, flags | 0x20)?;
    let prior = world.view_word(0x68)?;
    world.view_store(0x68, prior.wrapping_add(count as u32))
}
fn remove_first(world: &mut impl World, cell: usize) -> Result<bool> {
    let Some(head) = world.head(cell)? else {
        return Ok(false);
    };
    let view = world.view();
    if world.node_view(head)? == view {
        let next = world.node_next(head)?;
        world.head_store(cell, next)?;
        if next.is_none() {
            world.tail_store(cell, None)?;
        }
        world.free(head)?;
        return Ok(true);
    }
    let mut previous = None;
    let mut cursor = head;
    loop {
        if world.node_view(cursor)? == view {
            let previous = previous.ok_or("VGrid changed head payload during unlink")?;
            let next = world.node_next(cursor)?;
            world.node_next_store(previous, next)?;
            if world.tail(cell)? == Some(cursor) {
                world.tail_store(cell, Some(previous))?;
            }
            world.free(cursor)?;
            return Ok(true);
        }
        let Some(next) = world.node_next(cursor)? else {
            return Ok(false);
        };
        previous = Some(cursor);
        cursor = next;
    }
}
fn unlink<W: World>(world: &mut W, cell: usize, target: W::Node) -> Result<()> {
    let Some(head) = world.head(cell)? else {
        return Ok(());
    };
    if head == target {
        let next = world.node_next(head)?;
        world.head_store(cell, next)?;
        if next.is_none() {
            world.tail_store(cell, None)?;
        }
        return world.free(head);
    }
    let mut previous = None;
    let mut cursor = head;
    loop {
        if cursor == target {
            let previous = previous.ok_or("VGrid unlink predecessor absent")?;
            let next = world.node_next(cursor)?;
            world.node_next_store(previous, next)?;
            if world.tail(cell)? == Some(cursor) {
                world.tail_store(cell, Some(previous))?;
            }
            return world.free(cursor);
        }
        let Some(next) = world.node_next(cursor)? else {
            return Ok(());
        };
        previous = Some(cursor);
        cursor = next;
    }
}
///42FD40 removes every remaining matching node; it does not reset VThing+68.
fn cleanup(world: &mut impl World) -> Result<()> {
    let height = world.grid_word(0x20)? as i32;
    let width = world.grid_word(0x1c)? as i32;
    let count = height.wrapping_mul(width);
    let view = world.view();
    for cell in 0..count.max(0) as usize {
        let mut cursor = world.head(cell)?;
        while let Some(node) = cursor {
            if world.node_view(node)? == view {
                cursor = world.node_next(node)?;
                unlink(world, cell, node)?;
            } else {
                cursor = world.node_next(node)?;
            }
        }
    }
    Ok(())
}
pub(super) fn unoccupy(world: &mut impl World, pc: Precision) -> Result<()> {
    let [x0, y0, x1, y1] = rectangle(world, pc)?;
    let mut delta = 0i32;
    for x in x0..=x1 {
        for y in y0..=y1 {
            let index = cell(world, x, y)?;
            if remove_first(world, index)? {
                delta = delta.wrapping_sub(1);
            }
        }
    }
    let flags = world.view_word(0x54)?;
    world.view_store(0x54, flags & !0x20)?;
    let prior = world.view_word(0x68)?;
    let current = prior.wrapping_add(delta as u32);
    world.view_store(0x68, current)?;
    if current != 0 {
        world.residual_warning(current as i32)?;
        cleanup(world)?;
    }
    Ok(())
}

struct Bound<'a, V, F> {
    state: &'a mut State<V>,
    view: &'a mut F,
}
impl<V: Copy + Eq> State<V> {
    ///42FD10 calls42F860 over this SAME initialized cell/node allocation.
    pub fn occupy(&mut self, view: &mut impl ViewFields<V>, pc: Precision) -> Result<()> {
        occupy(&mut Bound { state: self, view }, pc)
    }
    ///430E20/430410. Diagnostic failure retains completed local removals,
    ///flag clear and count store; it never fabricates whole-grid cleanup.
    pub fn unoccupy(&mut self, view: &mut impl ViewFields<V>, pc: Precision) -> Result<()> {
        unoccupy(&mut Bound { state: self, view }, pc)
    }
}
impl<V, F> Bound<'_, V, F> {
    fn cell(&self, index: usize) -> Result<&Cell<V>> {
        self.state
            .cells
            .as_deref()
            .and_then(|c| c.get(index))
            .ok_or_else(|| "VGrid cell is outside the actual initialized allocation".into())
    }
    fn cell_mut(&mut self, index: usize) -> Result<&mut Cell<V>> {
        self.state
            .cells
            .as_deref_mut()
            .and_then(|c| c.get_mut(index))
            .ok_or_else(|| "VGrid cell is outside the actual initialized allocation".into())
    }
    fn node(&self, id: NodeId) -> Result<&Node<V>> {
        self.state
            .nodes
            .get(id.0)
            .and_then(Option::as_ref)
            .ok_or_else(|| "VGrid node is not allocated in this owner".into())
    }
    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node<V>> {
        self.state
            .nodes
            .get_mut(id.0)
            .and_then(Option::as_mut)
            .ok_or_else(|| "VGrid node is not allocated in this owner".into())
    }
}
impl<V: Copy + Eq, F: ViewFields<V>> World for Bound<'_, V, F> {
    type View = V;
    type Node = NodeId;
    fn view(&self) -> V {
        self.view.handle()
    }
    fn view_word(&mut self, offset: u32) -> Result<u32> {
        self.view.word(offset)
    }
    fn view_store(&mut self, offset: u32, value: u32) -> Result<()> {
        self.view.store(offset, value)
    }
    fn grid_word(&mut self, offset: u32) -> Result<u32> {
        self.state.required(offset)
    }
    fn head(&mut self, cell: usize) -> Result<Option<NodeId>> {
        Ok(self.cell(cell)?.head)
    }
    fn tail(&mut self, cell: usize) -> Result<Option<NodeId>> {
        Ok(self.cell(cell)?.tail)
    }
    fn head_store(&mut self, cell: usize, node: Option<NodeId>) -> Result<()> {
        self.cell_mut(cell)?.head = node;
        Ok(())
    }
    fn tail_store(&mut self, cell: usize, node: Option<NodeId>) -> Result<()> {
        self.cell_mut(cell)?.tail = node;
        Ok(())
    }
    fn allocate(&mut self) -> Result<NodeId> {
        if let Some(id) = self.state.free_nodes.pop() {
            let slot = self
                .state
                .nodes
                .get_mut(id.0)
                .ok_or("VGrid free-node slot absent")?;
            if slot.is_some() {
                return Err("VGrid free-node slot is already live".into());
            }
            *slot = Some(Node {
                view: None,
                next: None,
            });
            return Ok(id);
        }
        self.state.nodes.try_reserve(1).map_err(|e| e.to_string())?;
        let id = NodeId(self.state.nodes.len());
        self.state.nodes.push(Some(Node {
            view: None,
            next: None,
        }));
        Ok(id)
    }
    fn node_view(&mut self, node: NodeId) -> Result<V> {
        self.node(node)?
            .view
            .ok_or_else(|| "VGrid node payload is unwritten".into())
    }
    fn node_next(&mut self, node: NodeId) -> Result<Option<NodeId>> {
        self.node(node)?
            .next
            .ok_or_else(|| "VGrid node next field is unwritten".into())
    }
    fn node_view_store(&mut self, node: NodeId, view: V) -> Result<()> {
        self.node_mut(node)?.view = Some(view);
        Ok(())
    }
    fn node_next_store(&mut self, node: NodeId, next: Option<NodeId>) -> Result<()> {
        self.node_mut(node)?.next = Some(next);
        Ok(())
    }
    fn free(&mut self, node: NodeId) -> Result<()> {
        self.state
            .free_nodes
            .try_reserve(1)
            .map_err(|e| e.to_string())?;
        let slot = self
            .state
            .nodes
            .get_mut(node.0)
            .ok_or("VGrid free outside its node owner")?;
        if slot.take().is_none() {
            return Err("VGrid node already freed".into());
        }
        self.state.free_nodes.push(node);
        Ok(())
    }
    fn residual_warning(&mut self, count: i32) -> Result<()> {
        self.view.residual_occupancy_warning(count)
    }
}

#[cfg(test)]
#[path="view_grid_occupancy_tests.rs"]
mod tests;
