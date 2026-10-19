//!Read-only walk of one VGrid cell's node list (42FAD0's `[cell]` head then
//!node+4), for the view streaming sweep. It never allocates or relinks.
use super::*;

impl<V: Copy> State<V> {
    ///The views of cell `index`, head first. A node whose view word was never
    ///stored is an error, as is an index outside the initialized cells.
    pub fn cell_views(&self, index: usize) -> Result<Vec<V>> {
        let cell = self
            .cells()?
            .get(index)
            .ok_or("VGrid cell is outside the actual initialized allocation")?;
        let mut out = Vec::new();
        let mut cursor = cell.head;
        while let Some(id) = cursor {
            let node = self
                .nodes
                .get(id.0)
                .and_then(Option::as_ref)
                .ok_or("VGrid cell node is not allocated in this owner")?;
            out.push(node.view.ok_or("VGrid cell node view was never stored")?);
            cursor = node.next.ok_or("VGrid cell node link was never stored")?;
            if out.len() > self.nodes.len() {
                return Err("VGrid cell node list is cyclic".into());
            }
        }
        Ok(out)
    }
}
