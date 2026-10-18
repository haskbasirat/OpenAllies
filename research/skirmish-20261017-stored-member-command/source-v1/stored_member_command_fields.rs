//!Read-only leaves for522890. Geometry and current words come from live owners.
use aa_mind::construct::Kind;
use aa_spatial::gworld_placement::GMapView;
use aa_zones::tile::MapGeometry;
type Result<T> = std::result::Result<T, String>;

///5543F0 bounds check followed by the shared native truncation/shift/index core.
pub(crate) fn tile(geometry: &MapGeometry, point: [u32; 2]) -> Result<Option<usize>> {
    let [x, y] = point.map(f32::from_bits);
    let [width, height] = geometry.extent_bits.map(f32::from_bits);
    if !(x >= 0.0 && x < width && y >= 0.0 && y < height) {
        return Ok(None);
    }
    GMapView {
        width: geometry.width,
        height: geometry.height,
        coordinate_shift: geometry.shift,
        cell_size_bits: geometry.tile_size_bits,
        extent_bits: geometry.extent_bits,
    }
    .tile_at_point(point)
    .map(Some)
    .map_err(|e| e.to_string())
}

///Mind vf80:535EE0/524890 copy two current words;51C230 also reads+54.
///Base vf80 is the genuine96CAC2 purecall, never a zero-point fallback.
pub(crate) fn target(kind: Kind, mut word: impl FnMut(u32) -> Result<u32>) -> Result<[u32; 2]> {
    let offsets = match kind {
        Kind::Element => [0x90, 0x94],
        Kind::Regiment => [0x7c, 0x80],
        Kind::Aircraft => [0x4c, 0x50],
        Kind::Base => return Err("522890 actual Base Mind vf80 purecall96CAC2".into()),
    };
    let mut output = [word(offsets[0])?, word(offsets[1])?];
    if kind == Kind::Aircraft {
        let _ = word(0x54)?;
        //51C23E/24E copies X through x87; Y uses MOV. Unicorn does not quiet
        //signalling NaNs, so use the existing hardware-correct logical helper.
        output[0] = aa_mind::cai_thinks::x87_copy(output[0]);
    }
    Ok(output)
}

#[cfg(test)]
#[path = "stored_member_command_fields_tests.rs"]
mod tests;
