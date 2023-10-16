use shared::{Battery, Vehicle};

#[test]
fn test_a_default_battery() {
    let battery = Battery::new(
        0.05,
        0.95,
        0.5,
        0.5,
        250.0,
        50.0,
        40.0,
        [
            5.19073616313752e-7,
            -0.00018489381604332319,
            0.02337032885290201,
            0.029930977382156422,
        ],
        [
            86.58225544071823,
            -74.74020460962441,
            72.10950306705334,
            -3.9429566515545322,
        ],
    );

    let vehicle = Vehicle {
        original_id: 1,
        id: 1,
        index: 1,
        tour: vec![],
        battery,
    };

    let new_soc = vehicle.get_new_soc_after_charging(0.6, 15);
    dbg!(new_soc);
    assert!(new_soc > 0.75 && new_soc < 0.81);
}
