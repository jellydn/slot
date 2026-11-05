//! Explicit wlan0 lease ownership. No stock client, radio or service is stopped.
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use super::Failure;

pub trait Client: Send {
    fn poll(&mut self, now: Instant) -> Result<Option<String>, Failure>;
    fn stop(&mut self);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    pub address: Ipv4Addr,
    pub prefix: u32,
    pub router: Option<Ipv4Addr>,
    pub dns: Vec<Ipv4Addr>,
    pub seconds: u32,
}

fn address(text: &str) -> Result<Ipv4Addr, Failure> {
    let ip: Ipv4Addr = text.parse().map_err(|_| Failure::NoIp)?;
    if ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_broadcast()
    {
        return Err(Failure::NoIp);
    }
    Ok(ip)
}

impl Lease {
    // Hook fields are data, never shell commands. Reject unsupported classless
    // routing instead of silently applying a different network configuration.
    pub fn parse(text: &str) -> Result<Option<Self>, Failure> {
        let lines: Vec<_> = text.lines().collect();
        if lines.len() != 10 || lines[1] != "wlan0" {
            return Err(Failure::NoIp);
        }
        match lines[0] {
            "deconfig" => return Ok(None),
            "bound" | "renew" => {}
            _ => return Err(Failure::NoIp),
        }
        if !lines[8].is_empty() || !lines[9].is_empty() {
            return Err(Failure::NoIp);
        }
        let leased_address = address(lines[2])?;
        let mask: Ipv4Addr = lines[3].parse().map_err(|_| Failure::NoIp)?;
        let mask = u32::from(mask);
        let prefix = mask.leading_ones();
        if prefix == 0 || mask != u32::MAX.checked_shl(32 - prefix).unwrap_or(0) {
            return Err(Failure::NoIp);
        }
        let host = u32::from(leased_address) & !mask;
        if prefix < 31 && (host == 0 || host == !mask) {
            return Err(Failure::NoIp);
        }
        let routers: Vec<_> = lines[4].split_whitespace().collect();
        if routers.len() > 1 {
            return Err(Failure::NoIp);
        }
        let router = routers.first().map(|v| address(v)).transpose()?;
        if router.is_some_and(|r| u32::from(r) & mask != u32::from(leased_address) & mask) {
            return Err(Failure::NoIp);
        }
        let dns = lines[5]
            .split_whitespace()
            .map(address)
            .collect::<Result<Vec<_>, _>>()?;
        let seconds = lines[6].parse().map_err(|_| Failure::NoIp)?;
        if dns.len() > 3 || !(1..=604_800).contains(&seconds) || !lines[7].is_empty() {
            return Err(Failure::NoIp);
        }
        Ok(Some(Self {
            address: leased_address,
            prefix,
            router,
            dns,
            seconds,
        }))
    }

    #[cfg(target_os = "linux")]
    fn cidr(&self) -> String {
        format!("{}/{}", self.address, self.prefix)
    }
}

/// OS seam also used by deterministic lifecycle tests; implementations must
/// stop only their own child and conditionally restore only their own writes.
pub trait Runtime: Send {
    fn competitor(&self) -> bool;
    fn start(&mut self) -> Result<(), Failure>;
    fn alive(&mut self) -> bool;
    fn event(&mut self) -> Result<Option<String>, Failure>;
    fn apply(&mut self, lease: &Lease) -> Result<(), Failure>;
    fn stop(&mut self);
}

pub struct Owner<R: Runtime> {
    runtime: R,
    started: Option<Instant>,
    expires: Option<Instant>,
    ip: Option<String>,
    failed: bool,
}

impl<R: Runtime> Owner<R> {
    pub fn new(runtime: R) -> Self {
        Self {
            runtime,
            started: None,
            expires: None,
            ip: None,
            failed: false,
        }
    }
}

impl<R: Runtime> Client for Owner<R> {
    fn poll(&mut self, now: Instant) -> Result<Option<String>, Failure> {
        let result = (|| {
            if self.failed {
                return Err(Failure::NoIp);
            }
            if self.runtime.competitor() {
                return Err(Failure::DhcpBusy);
            }
            if self.started.is_none() {
                self.runtime.start()?;
                self.started = Some(now);
            }
            if !self.runtime.alive() {
                return Err(Failure::NoIp);
            }
            if let Some(event) = self.runtime.event()? {
                match Lease::parse(&event)? {
                    Some(lease) => {
                        self.runtime.apply(&lease)?;
                        self.ip = Some(lease.address.to_string());
                        self.expires = Some(now + Duration::from_secs(lease.seconds.into()));
                    }
                    None => {
                        // A lease loss must not advertise the previous address.
                        if self.expires.is_some() {
                            return Err(Failure::NoIp);
                        }
                    }
                }
            }
            if self.expires.is_some_and(|t| now >= t)
                || (self.ip.is_none()
                    && now.duration_since(self.started.unwrap()) >= Duration::from_secs(12))
            {
                return Err(Failure::NoIp);
            }
            Ok(self.ip.clone())
        })();
        if result.is_err() {
            self.runtime.stop();
            self.ip = None;
            self.failed = true;
        }
        result
    }

    fn stop(&mut self) {
        self.runtime.stop();
        self.started = None;
        self.expires = None;
        self.ip = None;
        self.failed = false;
    }
}

impl<R: Runtime> Drop for Owner<R> {
    fn drop(&mut self) {
        self.runtime.stop();
    }
}

#[cfg(target_os = "linux")]
pub use linux::System;

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::io::{Read, Write};
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
    use std::os::unix::process::CommandExt;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Resolver {
        path: PathBuf,
        dev: u64,
        ino: u64,
        before: Vec<u8>,
        ours: Vec<u8>,
    }

    #[derive(Default)]
    pub struct System {
        child: Option<Child>,
        dir: Option<PathBuf>,
        lease: Option<Lease>,
        resolver: Option<Resolver>,
    }

    fn ip(args: &[&str]) -> Result<String, Failure> {
        let output = Command::new("/bin/busybox")
            .arg("ip")
            .args(args)
            .stderr(Stdio::null())
            .output()
            .map_err(|_| Failure::NoIp)?;
        if !output.status.success() || output.stdout.len() > 16_384 {
            return Err(Failure::NoIp);
        }
        String::from_utf8(output.stdout).map_err(|_| Failure::NoIp)
    }

    impl System {
        fn unmanaged_network() -> Result<bool, Failure> {
            Ok(!ip(&["-4", "-o", "addr", "show", "dev", "wlan0"])?
                .trim()
                .is_empty()
                || !ip(&["-4", "route", "show", "default", "dev", "wlan0"])?
                    .trim()
                    .is_empty())
        }

        fn address_present(lease: &Lease) -> bool {
            ip(&["-4", "-o", "addr", "show", "dev", "wlan0"])
                .is_ok_and(|s| s.split_whitespace().any(|v| v == lease.cidr()))
        }

        fn route_present(lease: &Lease) -> bool {
            let Some(router) = lease.router else {
                return false;
            };
            ip(&["-4", "route", "show", "default", "dev", "wlan0"]).is_ok_and(|s| {
                s.lines().any(|line| {
                    line.starts_with(&format!("default via {router} "))
                        && line
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .windows(2)
                            .any(|w| w == ["metric", "42700"])
                })
            })
        }

        fn restore(&mut self) {
            // A stock writer appearing is a handoff, not permission to undo it.
            if self.competitor() {
                self.lease = None;
                self.resolver = None;
                return;
            }
            if let Some(r) = self.resolver.take() {
                let _ = Self::write_resolver(&r.path, r.dev, r.ino, &r.ours, &r.before);
            }
            if let Some(lease) = self.lease.take() {
                if Self::route_present(&lease) {
                    let _ = ip(&[
                        "route",
                        "del",
                        "default",
                        "via",
                        &lease.router.unwrap().to_string(),
                        "dev",
                        "wlan0",
                        "metric",
                        "42700",
                    ]);
                }
                if Self::address_present(&lease) {
                    let _ = ip(&["addr", "del", &lease.cidr(), "dev", "wlan0"]);
                }
            }
        }

        fn write_resolver(
            path: &PathBuf,
            dev: u64,
            ino: u64,
            expected: &[u8],
            data: &[u8],
        ) -> Result<(), Failure> {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path)
                .map_err(|_| Failure::NoIp)?;
            let meta = file.metadata().map_err(|_| Failure::NoIp)?;
            let mut current = Vec::new();
            (&mut file)
                .take(16_385)
                .read_to_end(&mut current)
                .map_err(|_| Failure::NoIp)?;
            if meta.dev() != dev || meta.ino() != ino || current != expected {
                return Err(Failure::DhcpBusy);
            }
            use std::io::{Seek, SeekFrom};
            file.seek(SeekFrom::Start(0)).map_err(|_| Failure::NoIp)?;
            file.write_all(data)
                .and_then(|_| file.set_len(data.len() as u64))
                .and_then(|_| file.sync_all())
                .map_err(|_| Failure::NoIp)
        }
    }

    impl Runtime for System {
        fn competitor(&self) -> bool {
            let Ok(entries) = fs::read_dir("/proc") else {
                return true;
            };
            for entry in entries.flatten() {
                let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
                    continue;
                };
                if self.child.as_ref().is_some_and(|c| c.id() == pid) {
                    continue;
                }
                let bytes = match fs::read(entry.path().join("cmdline")) {
                    Ok(bytes) => bytes,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(_) => return true,
                };
                let args: Vec<_> = bytes
                    .split(|b| *b == 0)
                    .filter_map(|v| std::str::from_utf8(v).ok())
                    .collect();
                let client = args.iter().take(2).any(|v| {
                    ["udhcpc", "dhclient", "dhcpcd"].contains(&v.rsplit('/').next().unwrap_or(""))
                });
                if !client {
                    continue;
                }
                // Other-interface clients are ignored only when their interface
                // is explicit; unknown invocations fail closed.
                let iface = args
                    .windows(2)
                    .find(|a| a[0] == "-i")
                    .map(|a| a[1])
                    .or_else(|| {
                        args.iter()
                            .find_map(|a| a.strip_prefix("-i").filter(|v| !v.is_empty()))
                    });
                if iface.is_none_or(|v| v == "wlan0") {
                    return true;
                }
            }
            false
        }

        fn start(&mut self) -> Result<(), Failure> {
            if self.competitor() {
                return Err(Failure::DhcpBusy);
            }
            if Self::unmanaged_network()? {
                return Err(Failure::DhcpBusy);
            }
            let resolver = match fs::canonicalize("/etc/resolv.conf") {
                Ok(path) => path,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // The verified factory symlink may dangle before the first
                    // lease. Create only that exact tmpfs target, never /etc.
                    let path = fs::read_link("/etc/resolv.conf").map_err(|_| Failure::NoIp)?;
                    if path != PathBuf::from("/tmp/resolv.conf") {
                        return Err(Failure::DhcpBusy);
                    }
                    OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(0o644)
                        .open(&path)
                        .map_err(|_| Failure::DhcpBusy)?;
                    path
                }
                Err(_) => return Err(Failure::NoIp),
            };
            // Never write an internal persistent resolver or a firmware file.
            if !resolver.starts_with("/tmp") {
                return Err(Failure::DhcpBusy);
            }
            let meta = fs::symlink_metadata(&resolver).map_err(|_| Failure::NoIp)?;
            let before = fs::read(&resolver).map_err(|_| Failure::NoIp)?;
            if !meta.is_file() || meta.nlink() != 1 || before.len() > 16_384 {
                return Err(Failure::DhcpBusy);
            }
            self.resolver = Some(Resolver {
                path: resolver,
                dev: meta.dev(),
                ino: meta.ino(),
                before: before.clone(),
                ours: before,
            });
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let dir = PathBuf::from(format!(
                "/tmp/slot-dhcp-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&dir)
                .map_err(|_| Failure::NoIp)?;
            self.dir = Some(dir.clone());
            let hook = dir.join("hook");
            let script = format!("#!/bin/sh\n[ \"$interface\" = wlan0 ] || exit 1\ncase \"$1\" in deconfig|bound|renew) ;; *) exit 0;; esac\numask 077\nprintf '%s\\n' \"$1\" \"$interface\" \"$ip\" \"$subnet\" \"$router\" \"$dns\" \"$lease\" '' \"$staticroutes\" \"$msstaticroutes\" > '{0}/event.new' || exit 1\nmv '{0}/event.new' '{0}/event'\n", dir.display());
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o700)
                .open(&hook)
                .map_err(|_| Failure::NoIp)?;
            file.write_all(script.as_bytes())
                .map_err(|_| Failure::NoIp)?;
            fs::set_permissions(&hook, fs::Permissions::from_mode(0o700))
                .map_err(|_| Failure::NoIp)?;
            let mut command = Command::new("/sbin/udhcpc");
            command
                .args(["-f", "-n", "-i", "wlan0", "-t", "3", "-T", "3", "-s"])
                .arg(&hook)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            let parent = std::process::id();
            // Async-signal-safe syscalls only between fork and exec. A crashed
            // Slot must not leave a renewing orphan client behind.
            unsafe {
                command.pre_exec(move || {
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0
                        || libc::getppid() as u32 != parent
                    {
                        return Err(std::io::Error::other("DHCP parent unavailable"));
                    }
                    Ok(())
                });
            }
            self.child = Some(command.spawn().map_err(|_| Failure::NoIp)?);
            if self.competitor() {
                return Err(Failure::DhcpBusy);
            }
            Ok(())
        }

        fn alive(&mut self) -> bool {
            self.child
                .as_mut()
                .is_some_and(|c| matches!(c.try_wait(), Ok(None)))
        }

        fn event(&mut self) -> Result<Option<String>, Failure> {
            let Some(dir) = &self.dir else {
                return Ok(None);
            };
            let path = dir.join("processing");
            match fs::rename(dir.join("event"), &path) {
                Ok(()) => {
                    let bytes = fs::read(&path).map_err(|_| Failure::NoIp)?;
                    fs::remove_file(&path).map_err(|_| Failure::NoIp)?;
                    if bytes.len() > 4096 {
                        return Err(Failure::NoIp);
                    }
                    String::from_utf8(bytes)
                        .map(Some)
                        .map_err(|_| Failure::NoIp)
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(Failure::NoIp),
            }
        }

        fn apply(&mut self, lease: &Lease) -> Result<(), Failure> {
            if self.competitor() {
                return Err(Failure::DhcpBusy);
            }
            if self.lease.as_ref().is_some_and(|old| {
                old.address != lease.address
                    || old.prefix != lease.prefix
                    || old.router != lease.router
                    || old.dns != lease.dns
            }) {
                // An unexpected network reconfiguration needs a new explicit
                // Connect; never replace another owner's state during renewal.
                return Err(Failure::NoIp);
            }
            if self.lease.is_some() {
                if !Self::address_present(lease)
                    || (lease.router.is_some() && !Self::route_present(lease))
                {
                    return Err(Failure::DhcpBusy);
                }
                let r = self.resolver.as_ref().ok_or(Failure::NoIp)?;
                let meta = fs::symlink_metadata(&r.path).map_err(|_| Failure::DhcpBusy)?;
                if meta.dev() != r.dev
                    || meta.ino() != r.ino
                    || fs::read(&r.path).map_err(|_| Failure::DhcpBusy)? != r.ours
                {
                    return Err(Failure::DhcpBusy);
                }
                return Ok(());
            }
            if Self::unmanaged_network()? {
                return Err(Failure::DhcpBusy);
            }
            let r = self.resolver.as_mut().ok_or(Failure::NoIp)?;
            let dns = if lease.dns.is_empty() {
                r.ours.clone()
            } else {
                lease
                    .dns
                    .iter()
                    .map(|ip| format!("nameserver {ip}\n"))
                    .collect::<String>()
                    .into_bytes()
            };
            Self::write_resolver(&r.path, r.dev, r.ino, &r.ours, &dns)?;
            r.ours = dns;
            ip(&["addr", "add", &lease.cidr(), "dev", "wlan0"])?;
            self.lease = Some(lease.clone());
            if let Some(router) = lease.router {
                ip(&[
                    "route",
                    "add",
                    "default",
                    "via",
                    &router.to_string(),
                    "dev",
                    "wlan0",
                    "metric",
                    "42700",
                ])?;
            }
            Ok(())
        }

        fn stop(&mut self) {
            if let Some(mut child) = self.child.take() {
                // Child handle only: no PID-file adoption or process-name kill.
                if matches!(child.try_wait(), Ok(None)) {
                    let _ = child.kill();
                }
                let _ = child.wait();
            }
            self.restore();
            if let Some(dir) = self.dir.take() {
                let _ = fs::remove_dir_all(dir);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn resolver_rollback_preserves_external_writes_and_replacements() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("resolver");
            let original = b"nameserver 192.0.2.53\n";
            let ours = b"nameserver 198.51.100.53\n";
            fs::write(&path, original).unwrap();
            let meta = fs::metadata(&path).unwrap();
            System::write_resolver(&path, meta.dev(), meta.ino(), original, ours).unwrap();
            System::write_resolver(&path, meta.dev(), meta.ino(), ours, original).unwrap();
            assert_eq!(fs::read(&path).unwrap(), original);

            fs::write(&path, b"external writer\n").unwrap();
            assert_eq!(
                System::write_resolver(&path, meta.dev(), meta.ino(), ours, original),
                Err(Failure::DhcpBusy)
            );
            assert_eq!(fs::read(&path).unwrap(), b"external writer\n");
            fs::rename(&path, dir.path().join("old")).unwrap();
            fs::write(&path, ours).unwrap();
            assert_eq!(
                System::write_resolver(&path, meta.dev(), meta.ino(), ours, original),
                Err(Failure::DhcpBusy)
            );
            assert_eq!(fs::read(&path).unwrap(), ours);
        }

        #[test]
        fn stop_restores_only_an_unchanged_owned_resolver() {
            for external in [false, true] {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("resolver");
                fs::write(&path, b"ours\n").unwrap();
                let meta = fs::metadata(&path).unwrap();
                let mut system = System {
                    resolver: Some(Resolver {
                        path: path.clone(),
                        dev: meta.dev(),
                        ino: meta.ino(),
                        before: b"before\n".to_vec(),
                        ours: b"ours\n".to_vec(),
                    }),
                    ..System::default()
                };
                if external {
                    fs::write(&path, b"external\n").unwrap();
                }
                system.stop();
                assert_eq!(
                    fs::read(&path).unwrap(),
                    if external {
                        b"external\n".as_slice()
                    } else {
                        b"before\n".as_slice()
                    }
                );
                system.stop();
            }
        }

        #[test]
        fn resolver_symlink_is_never_followed() {
            let dir = tempfile::tempdir().unwrap();
            let target = dir.path().join("target");
            let path = dir.path().join("resolver");
            fs::write(&target, b"unchanged").unwrap();
            std::os::unix::fs::symlink(&target, &path).unwrap();
            let meta = fs::metadata(&target).unwrap();
            assert!(
                System::write_resolver(&path, meta.dev(), meta.ino(), b"unchanged", b"new")
                    .is_err()
            );
            assert_eq!(fs::read(target).unwrap(), b"unchanged");
        }
    }
}
