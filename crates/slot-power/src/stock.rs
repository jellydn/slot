//! TG5040 controls backed by the retained stock Tina system.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Battery, Charge, LedState, Platform};

pub struct StockPlatform {
    root: PathBuf,
    battery: PathBuf,
    #[cfg(target_os = "linux")]
    display: Option<std::fs::File>,
}

impl StockPlatform {
    pub fn new(root: PathBuf) -> Self {
        let platform = Self::probe(Path::new("/sys"), root);
        #[cfg(target_os = "linux")]
        let platform = Self {
            display: std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/disp")
                .ok(),
            ..platform
        };
        platform
    }

    pub fn probe(sysfs: &Path, root: PathBuf) -> Self {
        Self {
            root,
            battery: sysfs.join("class/power_supply/axp2202-battery"),
            #[cfg(target_os = "linux")]
            display: None,
        }
    }

    #[cfg(target_os = "linux")]
    fn display_ioctl(&self, command: std::ffi::c_ulong, value: u32) -> Option<u32> {
        use std::os::fd::AsRawFd;

        extern "C" {
            fn ioctl(fd: std::ffi::c_int, request: std::ffi::c_ulong, ...) -> std::ffi::c_int;
        }

        // sunxi disp2 takes four native unsigned longs; GET returns the value
        // as the ioctl result, not in the argument buffer. Screen 0 is the LCD.
        let args = [0, std::ffi::c_ulong::from(value), 0, 0];
        let result = unsafe { ioctl(self.display.as_ref()?.as_raw_fd(), command, args.as_ptr()) };
        u32::try_from(result).ok()
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

    fn backlight_step(&self) -> Option<u8> {
        #[cfg(target_os = "linux")]
        {
            let raw = self.display_ioctl(0x103, 0)?;
            if raw > 255 {
                return None;
            }
            (0..=10).min_by_key(|&step| crate::device::backlight_value(step, 255).abs_diff(raw))
        }
        #[cfg(not(target_os = "linux"))]
        None
    }

    fn set_backlight(&mut self, step: u8) {
        #[cfg(target_os = "linux")]
        if self.backlight_step().is_some() {
            let value = crate::device::backlight_value(step, 255);
            if self.display_ioctl(0x102, value).is_none() {
                eprintln!(
                    "slot: stock backlight write failed: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = step;
    }

    // Stock helpers retain ownership of GPIO, RTC and USB.
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
