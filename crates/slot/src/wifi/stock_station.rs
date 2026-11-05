//! User-requested connections through the existing stock supplicant only.
//! Stock retains supplicant/radio policy. Explicit connections may use one
//! scoped DHCP owner; profile persistence waits for an associated valid lease.
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use super::stock::{Control, Scanner};
use super::{wpa, Backend, Failure, Saved, ScanError, Scanned, State, Status};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const LIST_HEADER: &str = "network id / ssid / bssid / flags";

struct Pending {
    id: u32,
    ssid: String,
    new: bool,
    enabled: Vec<u32>,
    previous: Option<u32>,
}

pub struct Station<C: Control> {
    scanner: Scanner<C>,
    pending: Option<Pending>,
    known: Vec<Saved>,
    last: Status,
    last_id: Option<u32>,
    error: Option<Failure>,
    persistence_safe: bool,
    dhcp: Option<Box<dyn super::stock_dhcp::Client>>,
    dhcp_profile: Option<(u32, String)>,
    managed: bool,
}

impl<C: Control> Station<C> {
    pub fn new(control: C) -> Self {
        Self {
            scanner: Scanner::new(control),
            pending: None,
            known: Vec::new(),
            last: Status::default(),
            last_id: None,
            error: None,
            persistence_safe: true,
            dhcp: None,
            dhcp_profile: None,
            managed: false,
        }
    }

    pub fn with_dhcp(control: C, client: Box<dyn super::stock_dhcp::Client>) -> Self {
        let mut station = Self::new(control);
        station.dhcp = Some(client);
        station
    }

    fn stop_dhcp(&mut self) {
        if let Some(client) = &mut self.dhcp {
            client.stop();
        }
        self.dhcp_profile = None;
    }

    fn request(&mut self, command: &str, until: Instant) -> Result<String, Failure> {
        // Command data (including PSK) never enters argv, a shell, a file or a log.
        let control = &mut self.scanner.control;
        let result = (|| {
            if Instant::now() >= until {
                return Err(Failure::Other);
            }
            control.open().map_err(|_| Failure::Other)?;
            control.send(command, until).map_err(|_| Failure::Other)?;
            for _ in 0..64 {
                if Instant::now() >= until {
                    break;
                }
                let text = control.recv(until).map_err(|_| Failure::Other)?;
                if !text.starts_with('<') {
                    return Ok(text);
                }
            }
            Err(Failure::Other)
        })();
        control.close();
        result
    }

    fn ok(&mut self, command: &str, until: Instant) -> bool {
        self.request(command, until)
            .is_ok_and(|text| text.trim() == "OK")
    }

    fn list(&mut self, until: Instant) -> Result<Vec<Saved>, Failure> {
        let text = self.request("LIST_NETWORKS", until)?;
        if text.lines().next() != Some(LIST_HEADER) {
            return Err(Failure::Other);
        }
        let mut rows: Vec<Saved> = Vec::new();
        for line in text.lines().skip(1).filter(|line| !line.is_empty()) {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 4 {
                return Err(Failure::Other);
            }
            let id = fields[0].parse().map_err(|_| Failure::Other)?;
            if rows.iter().any(|row| row.id == id) {
                return Err(Failure::Other);
            }
            rows.push(Saved {
                id,
                ssid: wpa::unescape(fields[1]),
                auto_join: !fields[3].contains("[DISABLED]"),
            });
        }
        // The currently selected profile wins UI deduplication, without deleting
        // an older user profile with the same SSID or reading its credentials.
        rows.sort_by_key(|row| Some(row.id) != self.last_id);
        Ok(rows)
    }

    fn restore(&mut self, pending: &Pending, connected: bool, until: Instant) -> bool {
        let mut ok = true;
        if !connected {
            if pending.new {
                ok &= self.ok(&format!("REMOVE_NETWORK {}", pending.id), until);
            }
            if let Some(id) = pending.previous.filter(|id| *id != pending.id) {
                ok &= self.ok(&format!("SELECT_NETWORK {id}"), until);
            }
            if !pending.new && !pending.enabled.contains(&pending.id) {
                ok &= self.ok(&format!("DISABLE_NETWORK {}", pending.id), until);
            }
        }
        for id in &pending.enabled {
            if *id != pending.id {
                ok &= self.ok(&format!("ENABLE_NETWORK {id}"), until);
            }
        }
        ok
    }

    fn cancel(&mut self, until: Instant) {
        if let Some(pending) = self.pending.take() {
            if !self.restore(&pending, false, until) {
                // A lost reply can leave a transient profile behind. Never let a
                // later SAVE_CONFIG accidentally persist that unconfirmed state.
                self.persistence_safe = false;
                self.error = Some(Failure::Other);
            }
        }
    }
}

impl<C: Control> Backend for Station<C> {
    fn keep_alive(&self) -> bool {
        self.dhcp_profile.is_some() && self.pending.is_none() && self.last.ip.is_some()
    }
    fn stock_owned(&self) -> bool {
        true
    }
    fn connection_error(&self) -> Option<Failure> {
        self.error
    }
    fn scan_error(&self) -> Option<ScanError> {
        self.scanner.scan_error()
    }
    fn start(&mut self) -> bool {
        self.request("PING", Instant::now() + REQUEST_TIMEOUT)
            .is_ok_and(|text| text.trim() == "PONG")
    }
    fn stop(&mut self) {
        self.cancel(Instant::now() + REQUEST_TIMEOUT);
        self.stop_dhcp();
    }
    fn block(&mut self) {}
    fn scan(&mut self) -> Vec<Scanned> {
        self.scanner.scan()
    }
    fn saved(&mut self) -> Vec<Saved> {
        match self.list(Instant::now() + REQUEST_TIMEOUT) {
            Ok(rows) => self.known = rows,
            Err(error) => self.error = Some(error),
        }
        self.known.clone()
    }
    fn status(&mut self) -> Status {
        match self.request("STATUS", Instant::now() + REQUEST_TIMEOUT) {
            Ok(text) if text.lines().any(|line| line.starts_with("wpa_state=")) => {
                self.last = wpa::parse_status(&text);
                self.last_id = text
                    .lines()
                    .find_map(|line| line.strip_prefix("id=")?.parse().ok());
                if self.pending.as_ref().is_some_and(|pending| {
                    self.last.state == State::Completed
                        && self.last.ssid.as_deref() == Some(pending.ssid.as_str())
                        && self.last_id != Some(pending.id)
                }) {
                    self.error = Some(Failure::Other);
                }
                // This filters unusable addresses, not stale leases. STATUS
                // reports an address; it does not prove DHCP or Internet access.
                self.last.ip = self.last.ip.take().filter(|value| {
                    self.last.state == State::Completed
                        && value.parse::<Ipv4Addr>().is_ok_and(|ip| {
                            !ip.is_unspecified()
                                && !ip.is_loopback()
                                && !ip.is_link_local()
                                && !ip.is_multicast()
                                && !ip.is_broadcast()
                        })
                });
            }
            _ => {
                self.last = Status::default();
                self.last_id = None;
                self.error = Some(Failure::Other);
            }
        }
        if self.managed {
            self.last.ip = None;
            if let Some((id, ssid)) = &self.dhcp_profile {
                if self.last.state == State::Completed
                    && self.last_id == Some(*id)
                    && self.last.ssid.as_deref() == Some(ssid.as_str())
                {
                    match self.dhcp.as_mut().unwrap().poll(Instant::now()) {
                        Ok(ip) => self.last.ip = ip,
                        Err(error) => {
                            self.error = Some(error);
                            self.stop_dhcp();
                        }
                    }
                } else if self.pending.is_none() {
                    self.stop_dhcp();
                }
            }
        }
        self.last.clone()
    }
    fn temp_disabled(&mut self, id: u32) -> bool {
        self.request("LIST_NETWORKS", Instant::now() + REQUEST_TIMEOUT)
            .is_ok_and(|text| wpa::temp_disabled(&text, id))
    }
    fn add(&mut self, ssid: &str, password: &str, hidden: bool) -> Option<u32> {
        self.error = None;
        if self.scanner.unsupported.iter().any(|name| name == ssid) {
            self.error = Some(Failure::Unsupported);
            return None;
        }
        if !self.persistence_safe
            || ssid.is_empty()
            || ssid.len() > 32
            || (!password.is_empty()
                && (!(8..=63).contains(&password.len())
                    || !password.bytes().all(|b| (32..=126).contains(&b))))
        {
            self.error = Some(Failure::Other);
            return None;
        }
        let until = Instant::now() + REQUEST_TIMEOUT;
        self.cancel(until);
        if !self.persistence_safe {
            return None;
        }
        let result = (|| {
            let rows = self.list(until)?;
            let status = self.request("STATUS", until)?;
            let previous = status
                .lines()
                .find_map(|line| line.strip_prefix("id=")?.parse().ok());
            let reply = self.request("ADD_NETWORK", until).inspect_err(|_| {
                // The request may have succeeded before its reply was lost.
                self.persistence_safe = false;
            })?;
            let id = reply.trim().parse::<u32>().map_err(|_| {
                self.persistence_safe = false;
                Failure::Other
            })?;
            if rows.iter().any(|row| row.id == id) {
                self.persistence_safe = false;
                return Err(Failure::Other);
            }
            self.pending = Some(Pending {
                id,
                ssid: ssid.into(),
                new: true,
                enabled: rows
                    .iter()
                    .filter(|row| row.auto_join)
                    .map(|row| row.id)
                    .collect(),
                previous,
            });
            let psk = password.replace('\\', "\\\\").replace('"', "\\\"");
            let mut commands = vec![
                format!("SET_NETWORK {id} ssid {}", wpa::hex(ssid)),
                if password.is_empty() {
                    format!("SET_NETWORK {id} key_mgmt NONE")
                } else {
                    format!("SET_NETWORK {id} psk \"{psk}\"")
                },
            ];
            if hidden {
                commands.push(format!("SET_NETWORK {id} scan_ssid 1"));
            }
            for command in commands {
                if !self.ok(&command, until) {
                    return Err(Failure::Other);
                }
            }
            Ok(id)
        })();
        match result {
            Ok(id) => Some(id),
            Err(error) => {
                self.error = Some(error);
                self.cancel(Instant::now() + REQUEST_TIMEOUT);
                None
            }
        }
    }
    fn connect(&mut self, id: u32) {
        self.error = None;
        self.stop_dhcp();
        if !self.persistence_safe {
            self.error = Some(Failure::Other);
            return;
        }
        let until = Instant::now() + REQUEST_TIMEOUT;
        if self.pending.as_ref().is_none_or(|pending| pending.id != id) {
            self.cancel(until);
            if !self.persistence_safe {
                self.error = Some(Failure::Other);
                return;
            }
            let Ok(rows) = self.list(until) else {
                self.error = Some(Failure::Other);
                return;
            };
            let Some(row) = rows.iter().find(|row| row.id == id) else {
                self.error = Some(Failure::Other);
                return;
            };
            let Ok(status) = self.request("STATUS", until) else {
                self.error = Some(Failure::Other);
                return;
            };
            self.pending = Some(Pending {
                id,
                ssid: row.ssid.clone(),
                new: false,
                enabled: rows
                    .iter()
                    .filter(|row| row.auto_join)
                    .map(|row| row.id)
                    .collect(),
                previous: status
                    .lines()
                    .find_map(|line| line.strip_prefix("id=")?.parse().ok()),
            });
        }
        self.last = Status::default();
        self.last_id = None;
        if !self.ok(&format!("SELECT_NETWORK {id}"), until) {
            self.error = Some(Failure::Other);
            self.cancel(Instant::now() + REQUEST_TIMEOUT);
        } else if self.dhcp.is_some() {
            self.managed = true;
            self.dhcp_profile = self.pending.as_ref().map(|p| (p.id, p.ssid.clone()));
        }
    }
    fn finish_connect(&mut self) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        let connected = self.last_id == Some(pending.id)
            && self.last.ssid.as_deref() == Some(&pending.ssid)
            && self.last.state == State::Completed
            && self.last.ip.is_some();
        if !connected {
            self.stop_dhcp();
        }
        let until = Instant::now() + REQUEST_TIMEOUT;
        if !self.restore(&pending, connected, until) {
            self.persistence_safe = false;
            self.error = Some(Failure::Other);
        } else if connected && pending.new && !self.ok("SAVE_CONFIG", until) {
            self.error = Some(Failure::SaveUnconfirmed);
            self.persistence_safe = false;
        }
    }
    // Existing profiles and automatic network/radio/DHCP policy are not Slot's
    // property. In particular, worker duplicate cleanup must not remove them.
    fn forget(&mut self, _id: u32) {}
    fn disconnect(&mut self) {
        self.error = None;
        self.cancel(Instant::now() + REQUEST_TIMEOUT);
        if !self.ok("DISCONNECT", Instant::now() + REQUEST_TIMEOUT) {
            self.error = Some(Failure::DisconnectFailed);
        } else {
            self.stop_dhcp();
        }
    }
    fn set_auto_join(&mut self, _id: u32, _on: bool) {}
}

impl<C: Control> Drop for Station<C> {
    fn drop(&mut self) {
        // Roll back an unfinished profile and release only Slot's DHCP state.
        // The stock supplicant/radio and external clients are left alone.
        self.cancel(Instant::now() + REQUEST_TIMEOUT);
        self.stop_dhcp();
    }
}
