//! Stock Tina runtime policy. This is not MinUI's boot takeover path.

use std::io;
use std::path::Path;

pub const ROOT: &str = "/mnt/SDCARD/Slot";
pub const BOARD: &str = "tg5040";
pub const BLOCKED_EXIT: i32 = 78;

pub fn check_mounts(text: &str) -> io::Result<()> {
    let mut found = 0;
    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 4 {
            return Err(io::Error::other("malformed mount table"));
        }
        if fields[1].starts_with(&format!("{ROOT}/")) || fields[1] == ROOT {
            return Err(io::Error::other("nested Slot mount refused"));
        }
        if fields[1] == "/mnt/SDCARD" {
            found += 1;
            if !fields[0].starts_with("/dev/")
                || !matches!(fields[2], "exfat" | "vfat")
                || !fields[3].split(',').any(|option| option == "rw")
                || fields[3].split(',').any(|option| option == "ro")
            {
                return Err(io::Error::other(
                    "SD must be a read-write FAT/exFAT device mount",
                ));
            }
        }
    }
    if found != 1 {
        return Err(io::Error::other("exactly one /mnt/SDCARD mount required"));
    }
    Ok(())
}

/// The independent image accepts only the SD selected by its PID1 supervisor.
pub fn check_tina_mounts(text: &str, card: &str) -> io::Result<()> {
    check_mounts(text)?;
    let number = card.strip_prefix("mmcblk").unwrap_or("");
    if number.len() != 1 || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(io::Error::other("invalid Tina SD device"));
    }
    let expected = format!("/dev/{card}p5");
    if !text.lines().any(|line| {
        let mut fields = line.split_whitespace();
        fields.next() == Some(expected.as_str()) && fields.next() == Some("/mnt/SDCARD")
    }) {
        return Err(io::Error::other(
            "Tina data must be selected SD partition 5",
        ));
    }
    Ok(())
}

/// No hardware/library initialization occurs until every precondition passes.
pub fn prepare() -> io::Result<()> {
    let root = Path::new(ROOT);
    let entry_ok = if cfg!(feature = "tina-sd") {
        std::env::var("SLOT_TINA_SD").as_deref() == Ok("1")
            && std::env::var_os("SLOT_STOCK_APP").is_none()
    } else {
        std::env::var("SLOT_STOCK_APP").as_deref() == Ok("1")
    };
    if !entry_ok
        || std::env::var("SLOT_ROOT").as_deref() != Ok(ROOT)
        || std::env::var("SLOT_BOARD").as_deref() != Ok(BOARD)
        || std::env::current_dir()? != root
        || std::env::current_exe()?.canonicalize()? != root.join("System/slot")
    {
        return Err(io::Error::other(
            "use only the prepared entrypoint and fixed root/board",
        ));
    }
    for name in ["SLOT_CORE", "LD_LIBRARY_PATH", "LD_PRELOAD", "LD_AUDIT"] {
        if std::env::var_os(name).is_some_and(|value| !value.is_empty()) {
            return Err(io::Error::other(format!("{name} override refused")));
        }
    }
    let mounts = std::fs::read_to_string("/proc/mounts")?;
    check_mounts(&mounts)?;
    if cfg!(feature = "tina-sd") {
        let card = std::fs::read_to_string("/run/slot-tina-sd")?;
        let card = card.trim();
        check_tina_mounts(&mounts, card)?;
        let sys = Path::new("/sys/class/block").join(card);
        if std::fs::read_to_string(sys.join("removable"))?.trim() != "1"
            || std::fs::read_to_string(sys.join("device/type"))?.trim() != "SD"
        {
            return Err(io::Error::other("Tina device is not removable SD"));
        }
        #[cfg(target_os = "linux")]
        if unsafe { libc::getppid() } != 1 {
            return Err(io::Error::other("Tina Slot must be a direct child of PID1"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        let device = std::fs::metadata(root)?.dev();
        for fd in [1, 2] {
            let target = std::fs::read_link(format!("/proc/self/fd/{fd}"))?;
            if !target.starts_with(root.join("Logs"))
                || !target.is_file()
                || std::fs::metadata(target)?.dev() != device
            {
                return Err(io::Error::other(
                    "stdout/stderr must be regular SD Logs files",
                ));
            }
        }
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    slot_store::fs::confine_writes(root)?;
    let runtime = root.join("Cache/Runtime");
    slot_store::fs::create_dir_all(&runtime)?;
    std::env::set_var("HOME", &runtime);
    std::env::set_var("TMPDIR", &runtime);
    std::env::set_var("XDG_CACHE_HOME", &runtime);
    std::env::set_var("XDG_CONFIG_HOME", root.join("Config"));
    std::env::set_var("SLOT_RUN", root.join("Logs"));
    eprintln!(
        "slot: TG5040 candidate; NULL EGL creation verified; full Slot/presentation untested"
    );
    Ok(())
}
