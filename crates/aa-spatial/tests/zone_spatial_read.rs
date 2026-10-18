//! Exact production read helper compiled against the real index owner.
#[path="../../aa-runtime/src/zone_spatial_read.rs"]
mod production;
use aa_spatial::{index::SpatialIndex,collision::Grid};
#[test]
fn rectangular_native_flat_indices_read_the_actual_index_words() {
    for pc in [aa_numeric::Precision::Single24,aa_numeric::Precision::Double53,aa_numeric::Precision::Extended64] {
        let flags=vec![0x00000001,0x00000100,0x20000400,0xffffffff,0x80000000,0];
        let index=SpatialIndex::new(Grid {dimensions:[3,2],coordinate_shift:1,cell_size_bits:2f32.to_bits()},flags.clone(),pc).unwrap();
        for (tile,expected) in flags.into_iter().enumerate() {
            assert_eq!(production::flags_at(&index,tile).unwrap(),expected);
            assert_eq!(production::primary_at(&index,tile).unwrap(),0);
        }
        assert!(production::flags_at(&index,6).is_err());
        assert!(production::flags_at(&index,usize::MAX).is_err());
        assert!(production::primary_at(&index,6).is_err());
        let other=SpatialIndex::new(Grid {dimensions:[3,2],coordinate_shift:1,cell_size_bits:2f32.to_bits()},vec![0x12345678;6],pc).unwrap();
        assert_eq!(production::flags_at(&other,4).unwrap(),0x12345678);
        assert_eq!(production::flags_at(&index,4).unwrap(),0x80000000);
    }
}
