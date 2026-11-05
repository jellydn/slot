//! Scan-only client of the existing stock supplicant. Stock retains all ownership.
use std::time::{Duration, Instant};

use super::{Backend, Saved, ScanError, Scanned, Status};

pub const ENDPOINT: &str = "/etc/wifi/sockets/wlan0";
const TIMEOUT: Duration = Duration::from_secs(8);

pub trait Control: Send {
    fn open(&mut self) -> Result<(), ScanError>;
    fn send(&mut self, command: &str, until: Instant) -> Result<(), ScanError>;
    fn recv(&mut self, until: Instant) -> Result<String, ScanError>;
    fn close(&mut self);
}

#[derive(Default)]
pub struct System {
    #[cfg(target_os = "linux")]
    socket: Option<std::os::unix::net::UnixDatagram>,
}

impl Control for System {
    fn open(&mut self) -> Result<(), ScanError> {
        #[cfg(target_os = "linux")]
        {
            use std::os::linux::net::SocketAddrExt;
            use std::os::unix::net::{SocketAddr, UnixDatagram};
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT: AtomicU64 = AtomicU64::new(0);
            // An abstract address needs no /tmp file, and works with a FAT32 card.
            let name = format!(
                "slot-scan-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            let addr = SocketAddr::from_abstract_name(name).map_err(|_| ScanError::Unavailable)?;
            let socket = UnixDatagram::bind_addr(&addr).map_err(|_| ScanError::Unavailable)?;
            socket
                .connect(ENDPOINT)
                .map_err(|_| ScanError::Unavailable)?;
            self.socket = Some(socket);
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        Err(ScanError::Unavailable)
    }

    fn send(&mut self, command: &str, until: Instant) -> Result<(), ScanError> {
        #[cfg(target_os = "linux")]
        {
            let left = remaining(until)?;
            let socket = self.socket.as_ref().ok_or(ScanError::Unavailable)?;
            socket.set_write_timeout(Some(left)).map_err(io_error)?;
            socket.send(command.as_bytes()).map_err(io_error)?;
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (command, until);
            Err(ScanError::Unavailable)
        }
    }

    fn recv(&mut self, until: Instant) -> Result<String, ScanError> {
        #[cfg(target_os = "linux")]
        {
            let left = remaining(until)?;
            let socket = self.socket.as_ref().ok_or(ScanError::Unavailable)?;
            socket.set_read_timeout(Some(left)).map_err(io_error)?;
            let mut bytes = vec![0; 65_537];
            let n = socket.recv(&mut bytes).map_err(io_error)?;
            if n > 65_536 {
                return Err(ScanError::Failed);
            }
            bytes.truncate(n);
            String::from_utf8(bytes).map_err(|_| ScanError::Failed)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = until;
            Err(ScanError::Unavailable)
        }
    }

    fn close(&mut self) {
        #[cfg(target_os = "linux")]
        {
            self.socket = None;
        }
    }
}

#[cfg(target_os = "linux")]
fn remaining(until: Instant) -> Result<Duration, ScanError> {
    until
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or(ScanError::TimedOut)
}

#[cfg(target_os = "linux")]
fn io_error(error: std::io::Error) -> ScanError {
    match error.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => ScanError::TimedOut,
        _ => ScanError::Unavailable,
    }
}

pub struct Scanner<C: Control> {
    pub(super) control: C,
    error: Option<ScanError>,
    pub(super) unsupported: Vec<String>,
}

impl<C: Control> Scanner<C> {
    pub fn new(control: C) -> Self {
        Self {
            control,
            error: None,
            unsupported: Vec::new(),
        }
    }

    fn scan_request(&mut self) -> Result<Vec<Scanned>, ScanError> {
        let until = Instant::now() + TIMEOUT;
        self.control.open()?;
        let result = self.scan_attached(until);
        // The same deadline covers cleanup. Never extend a stalled scan.
        let _ = self.control.send("DETACH", until);
        self.control.close();
        result
    }

    fn reply(&mut self, until: Instant) -> Result<String, ScanError> {
        loop {
            if Instant::now() >= until {
                return Err(ScanError::TimedOut);
            }
            let text = self.control.recv(until)?;
            if !text.starts_with('<') {
                return Ok(text);
            }
        }
    }

    fn scan_attached(&mut self, until: Instant) -> Result<Vec<Scanned>, ScanError> {
        for command in ["ATTACH", "SCAN"] {
            self.control.send(command, until)?;
            if self.reply(until)?.trim() != "OK" {
                return Err(ScanError::Failed);
            }
        }
        // Do not label cached results as a completed scan after an arbitrary sleep.
        loop {
            if Instant::now() >= until {
                return Err(ScanError::TimedOut);
            }
            let event = self.control.recv(until)?;
            let event = event
                .split_once('>')
                .map_or(event.as_str(), |(_, text)| text);
            if event.trim() == "CTRL-EVENT-SCAN-RESULTS" {
                break;
            }
            if event.starts_with("CTRL-EVENT-SCAN-FAILED") {
                return Err(ScanError::Failed);
            }
        }
        self.control.send("SCAN_RESULTS", until)?;
        let text = self.reply(until)?;
        if text.lines().next() != Some("bssid / frequency / signal level / flags / ssid")
            || text.lines().skip(1).filter(|l| !l.is_empty()).any(|line| {
                let columns: Vec<_> = line.split('\t').collect();
                columns.len() != 5 || columns[2].parse::<i32>().is_err()
            })
        {
            return Err(ScanError::Failed);
        }
        self.unsupported = text
            .lines()
            .skip(1)
            .filter_map(|line| {
                let columns: Vec<_> = line.split('\t').collect();
                let flags = columns.get(3)?;
                (flags.contains("WEP")
                    || flags.contains("EAP")
                    || flags.contains("OWE")
                    || (flags.contains("SAE") && !flags.contains("PSK")))
                .then(|| super::wpa::unescape(columns[4]))
            })
            .collect();
        Ok(super::wpa::parse_scan(&text))
    }
}

impl<C: Control> Backend for Scanner<C> {
    fn scan_only(&self) -> bool {
        true
    }
    fn scan_error(&self) -> Option<ScanError> {
        self.error
    }
    fn start(&mut self) -> bool {
        true
    }
    fn stop(&mut self) {}
    fn block(&mut self) {}
    fn scan(&mut self) -> Vec<Scanned> {
        match self.scan_request() {
            Ok(rows) => {
                self.error = None;
                rows
            }
            Err(error) => {
                self.error = Some(error);
                Vec::new()
            }
        }
    }
    fn saved(&mut self) -> Vec<Saved> {
        Vec::new()
    }
    fn status(&mut self) -> Status {
        Status::default()
    }
    fn temp_disabled(&mut self, _id: u32) -> bool {
        false
    }
    fn add(&mut self, _ssid: &str, _password: &str, _hidden: bool) -> Option<u32> {
        None
    }
    fn connect(&mut self, _id: u32) {}
    fn finish_connect(&mut self) {}
    fn disconnect(&mut self) {}
    fn set_auto_join(&mut self, _id: u32, _on: bool) {}
    fn forget(&mut self, _id: u32) {}
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::net::UnixDatagram;

    #[test]
    fn real_datagrams_are_bounded_and_close_without_a_socket_file() {
        let (client, server) = UnixDatagram::pair().unwrap();
        let mut control = System {
            socket: Some(client),
        };
        let until = Instant::now() + Duration::from_secs(1);
        control.send("SCAN", until).unwrap();
        let mut bytes = [0; 4];
        assert_eq!(server.recv(&mut bytes).unwrap(), 4);
        assert_eq!(&bytes, b"SCAN");
        server.send(b"OK\n").unwrap();
        assert_eq!(control.recv(until).unwrap(), "OK\n");
        assert_eq!(
            control.recv(Instant::now() + Duration::from_millis(20)),
            Err(ScanError::TimedOut)
        );
        assert_eq!(
            control.send("SCAN", Instant::now()),
            Err(ScanError::TimedOut)
        );
        server.send(&vec![b'x'; 65_537]).unwrap();
        assert_eq!(control.recv(until), Err(ScanError::Failed));
        control.close();
        assert_eq!(control.send("SCAN", until), Err(ScanError::Unavailable));
    }
}
