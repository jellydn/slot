#![cfg(feature = "stock-tg5040")]

use std::process::Command;

#[test]
fn stock_binary_refuses_unreviewed_entrypoints_before_creating_state() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("must-not-be-created");
    for args in [
        vec![],
        vec!["--install-bootlogo"],
        vec!["--init-root", root.to_str().unwrap()],
        vec!["--launch"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_slot"))
            .args(args)
            .env("SLOT_ROOT", &root)
            .env("SLOT_BOARD", "rg35xxsp")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(78));
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("no hardware has been initialized")
        );
        assert!(!root.exists());
    }
    assert_eq!(
        slot::input::evdev::detect_pad_map(),
        slot::input::evdev::PadMap::Tsp
    );
}

#[test]
fn stock_manual_entrypoint_refuses_without_running_a_command() {
    let d = tempfile::tempdir().unwrap();
    let result = Command::new("/bin/sh")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../knulli/stock-tg5040-check.sh"
        ))
        .arg("--launch")
        .env("PATH", d.path())
        .current_dir(d.path())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(78));
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("not the normal stock Apps launcher"));
    assert!(stderr.contains("all existing stock processes remain untouched"));
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
}

#[test]
fn stock_mount_contract_refuses_fallback_readonly_and_nested_mounts() {
    let good = "/dev/mmcblk1p1 /mnt/SDCARD exfat rw,noatime 0 0\n";
    assert!(slot::stock_tg5040::check_mounts(good).is_ok());
    assert!(slot::stock_tg5040::check_mounts(&good.replace("exfat", "vfat")).is_ok());
    for bad in [
        String::new(),
        good.repeat(2),
        good.replace("rw,noatime", "ro,noatime"),
        good.replace("exfat", "ext4"),
        good.replace("/dev/mmcblk1p1", "tmpfs"),
        good.replace("/mnt/SDCARD", "/mnt/SDCARD-other"),
        format!("{good}tmpfs /mnt/SDCARD/Slot/Logs tmpfs rw 0 0\n"),
    ] {
        assert!(slot::stock_tg5040::check_mounts(&bad).is_err(), "{bad}");
    }
}

#[test]
fn tina_mount_contract_requires_the_selected_cards_data_partition() {
    let good = "/dev/mmcblk1p5 /mnt/SDCARD vfat rw,noatime 0 0\n";
    assert!(slot::stock_tg5040::check_tina_mounts(good, "mmcblk1").is_ok());
    for (mounts, card) in [
        (good.to_string(), "mmcblk0"),
        (good.replace("p5", "p4"), "mmcblk1"),
        (good.replace("rw,", "ro,"), "mmcblk1"),
        (good.to_string(), "mmcblk1/../mmcblk0"),
        (good.to_string(), "mmcblk"),
        (good.to_string(), "mmcblkx"),
    ] {
        assert!(slot::stock_tg5040::check_tina_mounts(&mounts, card).is_err());
    }
}

#[test]
fn stock_apps_launcher_refuses_before_writes_on_host() {
    let result = Command::new("/bin/sh")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../knulli/stock-app/launch.sh"
        ))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&result.stderr).contains("Slot stock safety check"));
}
