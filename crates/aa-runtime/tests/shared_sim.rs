use aa_numeric::Precision;
use aa_runtime::{
    live_sim::Sim,
    simulation::{
        borrowed::{SimFields, TimeWord},
        shared_sim::SharedSim,
    },
};
use aa_simulation::FloatBits;
use std::cell::RefCell;

#[test]
fn driver_and_callback_field_loans_reach_same_sim_without_a_second_timeline() {
    let mut actual = Sim::from_native_constructor_fields();
    let identity = &actual as *const Sim;
    {
        let cell = RefCell::new(&mut actual);
        let mut driver = SharedSim::new(&cell);
        assert!(driver.frozen());
        assert!(driver.time(TimeWord::BeginGame).is_err());
        assert!(driver.game_per_admin_tick().is_err());
        // A synchronous callback writes through the original owner, between
        // driver field loans. Existing native reset fixtures prove arithmetic.
        {
            let mut callback = cell.borrow_mut();
            assert_eq!(&**callback as *const Sim, identity);
            callback
                .reset_admin_time_76dce0(FloatBits(0x3f000000), Precision::Double53, || {
                    Ok::<_, String>(FloatBits(0x40000000))
                })
                .unwrap();
            callback
                .store_frozen_after_sync(0, || Ok::<_, String>(()))
                .unwrap();
        }
        assert!(!driver.frozen());
        assert_eq!(driver.time(TimeWord::BeginAdmin).unwrap().0, 0xbf000000);
        driver
            .store_time(TimeWord::EndAdmin, FloatBits(0x3f800000))
            .unwrap();
        driver.store_avi_frames(7);
        {
            let callback = cell.borrow();
            assert_eq!(callback.time_words().end_admin.unwrap().0, 0x3f800000);
            assert_eq!(SimFields::avi_frames(&**callback), 7);
        }
        driver.store_frozen(true);
        assert!(cell.borrow().frozen());
        cell.borrow_mut()
            .store_frozen_after_sync(128, || Ok::<_, String>(()))
            .unwrap();
        assert!(driver.frozen());
        assert!(driver.frozen_for_log().is_err());
    }
    assert_eq!(&actual as *const Sim, identity);
    assert_eq!(actual.frozen_byte(), 128);
}
