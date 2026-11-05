use slot_power::{Battery, Charge, LedState, Platform, StockPlatform};
use std::fs;

#[test]
fn stock_reads_the_verified_battery_but_leaves_system_controls_untouched() {
    let d = tempfile::tempdir().unwrap();
    let battery = d.path().join("class/power_supply/axp2202-battery");
    fs::create_dir_all(&battery).unwrap();
    fs::write(battery.join("capacity"), "91\n").unwrap();
    fs::write(battery.join("status"), "Charging\n").unwrap();
    let backlight = d.path().join("class/backlight/lcd0");
    fs::create_dir_all(&backlight).unwrap();
    fs::write(backlight.join("brightness"), "37\n").unwrap();
    let gadget = d.path().join("kernel/config/usb_gadget/g1");
    fs::create_dir_all(&gadget).unwrap();
    fs::write(gadget.join("UDC"), "stock-controller\n").unwrap();
    let root = d.path().join("card");
    let mut platform = StockPlatform::probe(d.path(), root.clone());
    assert_eq!(platform.root(), root);
    assert_eq!(
        platform.battery(),
        Some(Battery {
            percent: 91,
            charge: Charge::Charging
        })
    );
    platform.set_backlight(0);
    platform.set_led(LedState::Off);
    platform.set_rumble(u16::MAX);
    platform.set_clock(1);
    platform.set_network_time(true);
    platform.set_network_time(false);
    assert!(!platform.relink_adb());
    assert!(platform.card_log().is_none());
    assert!(platform.take_repair_result().is_none());
    assert_eq!(
        fs::read_to_string(backlight.join("brightness")).unwrap(),
        "37\n"
    );
    assert_eq!(
        fs::read_to_string(gadget.join("UDC")).unwrap(),
        "stock-controller\n"
    );
    assert!(!root.exists());
    fs::write(battery.join("capacity"), "invalid\n").unwrap();
    assert_eq!(platform.battery(), None);
}
