use aa_actors::economy_values::{self, Error};

#[test]
fn layout_primary_is_read_only_for_with_primary_organization() {
    for class in [0, 1, 2, 3, u32::MAX] {
        for flags in [0, 1 << 7] {
            let mut reads = 0;
            let result = economy_values::recruitable_fields(class, flags, || {
                reads += 1;
                Err(Error::World("reached actual Layout+18 getter".into()))
            });
            if class == 2 && flags != 0 {
                assert!(result.unwrap_err().to_string().contains("Layout+18"));
                assert_eq!(reads, 1);
            } else {
                assert_eq!(result.unwrap(), class == 1);
                assert_eq!(reads, 0);
            }
        }
    }
    for primary in [i32::MIN, -1, 0, 1, i32::MAX] {
        assert_eq!(
            economy_values::recruitable_fields(2, 1 << 7, || Ok(primary)).unwrap(),
            primary >= 0
        );
    }
}
