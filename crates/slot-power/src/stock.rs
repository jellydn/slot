//! TG5040 controls backed by the retained stock Tina system.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Battery, Charge, LedState, Platform};

pub struct StockPlatform {
    root: PathBuf,
    battery: PathBuf,
}

impl StockPlatform {
    pub fn new(root: PathBuf) -> Self {
        Self::probe(Path::new("/sys"), root)
    }

    pub fn probe(sysfs: &Path, root: PathBuf) -> Self {
        Self {
            root,
            battery: sysfs.join("class/power_supply/axp2202-battery"),
        }
    }
}

impl Platform for StockPlatform {
    fn root(&self) -> &Path {
        &self.root
    }

    fn battery(&self) -> Option<Battery> {
        let percent = std::fs::read_to_string(self.battery.join("capacity"))
            .ok()?
            .trim()
            .parse::<u32>()
            .ok()?;
        Some(Battery {
            percent: percent.min(100) as u8,
            charge: self.charge(),
        })
    }

    fn charge(&self) -> Charge {
        match std::fs::read_to_string(self.battery.join("status"))
            .unwrap_or_default()
            .trim()
        {
            "Charging" => Charge::Charging,
            "Discharging" => Charge::Discharging,
            "Full" => Charge::Full,
            _ => Charge::Unknown,
        }
    }

    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64)
    }

    // Stock helpers retain ownership; do not write GPIO, debugfs, RTC or USB.
    fn set_backlight(&mut self, _step: u8) {}
    fn set_led(&mut self, _state: LedState) {}
    fn set_rumble(&mut self, _strength: u16) {}
    fn set_clock(&mut self, _secs: i64) {}

    fn request_restart(&mut self) -> Result<(), String> {
        // Factory /sbin/reboot is BusyBox's normal init request. No force flag,
        // driver unload, stock menu dispatch or guessed power register writes.
        let status = Command::new("/sbin/reboot")
            .status()
            .map_err(|_| "could not request reboot from init".to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("init reboot request failed".into())
        }
    }

    fn request_poweroff(&mut self) -> Result<(), String> {
        let status = Command::new("/sbin/poweroff")
            .status()
            .map_err(|_| "could not request poweroff from init".to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("init poweroff request failed".into())
        }
    }

    // Legacy non-returning operations remain refused. Normal init requests
    // above do not force power changes or unload drivers.
    fn restart(&mut self) -> ! {
        eprintln!("slot: stock restart refused");
        std::process::exit(78)
    }

    fn poweroff(&mut self) -> ! {
        eprintln!("slot: stock poweroff refused");
        std::process::exit(78)
    }

    fn repair_card(&mut self) -> ! {
        eprintln!("slot: stock card repair refused");
        std::process::exit(78)
    }
}
