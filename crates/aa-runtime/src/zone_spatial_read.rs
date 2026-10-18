//! Native row-major tile word+20 from the already owned spatial index.
//! No terrain input or cached flag vector can substitute for this live read.
fn tile_at(index:&aa_spatial::index::SpatialIndex,tile:usize)->Result<&aa_spatial::index::Tile,String> {
    let [width,height]=index.grid().dimensions;
    if width<=0 || height<=0 {return Err("zone actual spatial-index dimensions invalid".into())}
    let count=(width as usize).checked_mul(height as usize).ok_or("zone spatial-index dimensions overflow")?;
    if tile>=count {return Err(format!("zone spatial-index tile{tile} is outside{count}"))}
    index.tile([(tile%width as usize)as i32,(tile/width as usize)as i32]).map_err(|e|e.to_string())
}
pub(crate) fn flags_at(index:&aa_spatial::index::SpatialIndex,tile:usize)->Result<u32,String> {
    Ok(tile_at(index,tile)?.flags20)
}
pub(crate) fn primary_at(index:&aa_spatial::index::SpatialIndex,tile:usize)->Result<u32,String> {
    Ok(tile_at(index,tile)?.primary_reference)
}
