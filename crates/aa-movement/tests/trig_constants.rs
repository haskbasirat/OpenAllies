//! Cross-check: the native table values hard-coded by
//! aa_spatial::placement (ROTATION_SIN_BITS / ROTATION_COS_BITS, recorded from
//! 0x85DAA0 / 0x85DA80 under Unicorn) equal aa-trig's port of the same lookups.
use aa_numeric::Precision;

#[test]
fn minus_quarter_pi_table_lookups_match_aa_trig() {
    for p in [Precision::Single24, Precision::Double53, Precision::Extended64] {
        assert_eq!(aa_trig::lookup_sin(0xbf49_0fdb, p).unwrap(), 0xbf35_04ef);
        assert_eq!(aa_trig::lookup_cos(0xbf49_0fdb, p).unwrap(), 0x3f35_04f7);
    }
}
