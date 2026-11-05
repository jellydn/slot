pub mod attempt;
mod fake;
pub mod sp;
pub mod stock;
pub mod stock_station;
mod worker;
pub mod wpa;

pub use fake::Fake;
pub use worker::{online, wait_online, Wifi};

pub use attempt::{Failure, Phase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scanned {
    pub ssid: String,
    pub signal: i32,
    pub secured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub id: u32,
    pub ssid: String,
    pub auto_join: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Off,
    Idle,
    Scanning,
    Associating,
    Handshake,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Status {
    pub state: State,
    pub ssid: Option<String>,
    pub ip: Option<String>,
}

pub trait Backend: Send {
    fn stock_owned(&self) -> bool {
        false
    }
    fn connection_error(&self) -> Option<Failure> {
        None
    }
    fn scan_only(&self) -> bool {
        false
    }
    fn scan_error(&self) -> Option<ScanError> {
        None
    }
    fn start(&mut self) -> bool;
    fn stop(&mut self);
    fn block(&mut self);
    fn scan(&mut self) -> Vec<Scanned>;
    fn saved(&mut self) -> Vec<Saved>;
    fn status(&mut self) -> Status;
    fn temp_disabled(&mut self, id: u32) -> bool;
    fn add(&mut self, ssid: &str, password: &str, hidden: bool) -> Option<u32>;
    fn connect(&mut self, id: u32);
    fn finish_connect(&mut self);
    fn disconnect(&mut self);
    fn set_auto_join(&mut self, id: u32, on: bool);
    fn forget(&mut self, id: u32);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Hold,
    Release,
    PowerOff,
    Scan,
    ClearScan,
    Join {
        ssid: String,
        password: String,
        hidden: bool,
    },
    Connect(u32),
    Disconnect,
    AutoJoin {
        id: u32,
        on: bool,
    },
    Forget(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub stock_owned: bool,
    pub scan_only: bool,
    pub scan_error: Option<ScanError>,
    pub held: bool,
    pub saved: Vec<Saved>,
    pub scan: Option<Vec<Scanned>>,
    pub status: Status,
    pub attempt: Option<(String, Phase)>,
    pub failure: Option<(String, Failure)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanError {
    Unavailable,
    Failed,
    TimedOut,
}

impl ScanError {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unavailable => "Wi-Fi unavailable; use stock Settings",
            Self::Failed => "Scan failed; use stock Settings",
            Self::TimedOut => "Scan timed out; use stock Settings",
        }
    }
}
