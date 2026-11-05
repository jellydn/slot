mod common;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use slot::wifi::stock::{Control, Scanner};
use slot::wifi::{Backend, Cmd, ScanError, Wifi};

const HEADER: &str = "bssid / frequency / signal level / flags / ssid\n";

struct Mock {
    replies: VecDeque<Result<String, ScanError>>,
    calls: Arc<Mutex<Vec<String>>>,
    open: Result<(), ScanError>,
}

impl Control for Mock {
    fn open(&mut self) -> Result<(), ScanError> {
        self.open
    }
    fn send(&mut self, cmd: &str, until: Instant) -> Result<(), ScanError> {
        assert!(until <= Instant::now() + Duration::from_secs(8));
        self.calls.lock().unwrap().push(cmd.into());
        Ok(())
    }
    fn recv(&mut self, _until: Instant) -> Result<String, ScanError> {
        self.replies.pop_front().unwrap_or(Err(ScanError::TimedOut))
    }
    fn close(&mut self) {
        self.calls.lock().unwrap().push("close".into());
    }
}

fn scanner(replies: Vec<Result<String, ScanError>>) -> (Scanner<Mock>, Arc<Mutex<Vec<String>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    (
        Scanner::new(Mock {
            replies: replies.into(),
            calls: calls.clone(),
            open: Ok(()),
        }),
        calls,
    )
}

fn results(text: &str) -> Vec<Result<String, ScanError>> {
    vec![
        Ok("OK\n".into()),
        Ok("OK\n".into()),
        Ok("<3>CTRL-EVENT-SCAN-RESULTS\n".into()),
        Ok(text.into()),
    ]
}

#[test]
fn stock_scan_uses_completed_results_and_never_owns_network_settings() {
    let (mut b, calls) = scanner(results(&format!(
        "{HEADER}00:11:22:33:44:55\t2412\t-73\t[WPA2-PSK-CCMP][ESS]\tWeak\n00:11:22:33:44:66\t2462\t-31\t[ESS]\tCafe\\x20Guest\n"
    )));
    let rows = b.scan();
    assert_eq!(b.scan_error(), None);
    assert_eq!(
        rows.iter()
            .map(|r| (r.ssid.as_str(), r.signal, r.secured))
            .collect::<Vec<_>>(),
        vec![("Cafe Guest", -31, false), ("Weak", -73, true)]
    );
    assert!(b.start());
    b.stop();
    b.block();
    assert_eq!(b.add("Ignored", "not-a-real-secret", false), None);
    b.connect(7);
    b.disconnect();
    b.set_auto_join(7, false);
    b.forget(7);
    b.finish_connect();
    assert!(b.saved().is_empty());
    assert_eq!(b.status(), Default::default());
    assert_eq!(
        *calls.lock().unwrap(),
        ["ATTACH", "SCAN", "SCAN_RESULTS", "DETACH", "close"]
    );
}

#[test]
fn missing_endpoint_fail_timeout_malformed_and_empty_are_distinct() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut b = Scanner::new(Mock {
        replies: VecDeque::new(),
        calls: calls.clone(),
        open: Err(ScanError::Unavailable),
    });
    assert!(b.scan().is_empty());
    assert_eq!(b.scan_error(), Some(ScanError::Unavailable));
    assert!(calls.lock().unwrap().is_empty());
    for (replies, expected) in [
        (vec![Ok("FAIL\n".into())], Some(ScanError::Failed)),
        (
            vec![Ok("OK\n".into()), Ok("FAIL-BUSY\n".into())],
            Some(ScanError::Failed),
        ),
        (
            vec![Ok("OK\n".into()), Ok("OK\n".into())],
            Some(ScanError::TimedOut),
        ),
        (results("garbage\n"), Some(ScanError::Failed)),
        (
            results(&format!("{HEADER}bad row\n")),
            Some(ScanError::Failed),
        ),
        (
            vec![
                Ok("OK\n".into()),
                Ok("OK\n".into()),
                Ok("<3>CTRL-EVENT-SCAN-FAILED ret=-1\n".into()),
            ],
            Some(ScanError::Failed),
        ),
        (results(HEADER), None),
    ] {
        let (mut b, _) = scanner(replies);
        assert!(b.scan().is_empty());
        assert_eq!(b.scan_error(), expected);
    }
}

#[test]
fn worker_rejects_mutations_and_recovers_after_a_scan_error() {
    let mut replies = vec![Ok("OK\n".into()), Ok("FAIL-BUSY\n".into())];
    replies.extend(results(HEADER));
    let (b, calls) = scanner(replies);
    let mut w = Wifi::inline(Box::new(b));
    let now = Instant::now();
    w.want_scans(true);
    w.send(Cmd::Hold);
    w.poll(now);
    assert_eq!(w.snapshot().scan_error, Some(ScanError::Failed));
    for cmd in [
        Cmd::Join {
            ssid: "Ignored".into(),
            password: String::new(),
            hidden: false,
        },
        Cmd::Connect(7),
        Cmd::Disconnect,
        Cmd::AutoJoin { id: 7, on: true },
        Cmd::Forget(7),
        Cmd::PowerOff,
    ] {
        w.send(cmd);
    }
    w.poll(now + Duration::from_secs(1));
    assert_eq!(calls.lock().unwrap().len(), 4);
    w.poll(now + Duration::from_secs(11));
    assert_eq!(w.snapshot().scan_error, None);
    assert_eq!(w.snapshot().scan, Some(Vec::new()));
    assert!(w.snapshot().saved.is_empty());
    assert!(w.snapshot().attempt.is_none());
    w.send(Cmd::Release);
    w.poll(now + Duration::from_secs(12));
    assert!(!w.snapshot().held);
}

#[test]
fn stock_ui_scans_without_join_or_radio_controls_and_preserves_local_setting() {
    use slot::app::Phase;
    use slot_input::{Action, Btn};
    use slot_store::{read_slot_state, write_slot_state, SlotState};
    use slot_ui::QuickRow;
    let d = common::tmp_root_with_carts(&[]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            wifi_off: true,
            ..Default::default()
        },
    )
    .unwrap();
    let (mut app, _) = common::app_booting_at(d.path(), common::CLOCK_IS_SET);
    let (b, calls) = scanner(results(&format!(
        "{HEADER}00:11:22:33:44:55\t2412\t-42\t[ESS]\tDemo\n"
    )));
    app.set_wifi(Wifi::inline(Box::new(b)));
    assert!(app.hidden_rows().contains(&QuickRow::WifiPower));
    assert!(!app.hidden_rows().contains(&QuickRow::Networks));
    app.set_wifi_off(false);
    assert!(read_slot_state(d.path()).wifi_off);
    let press = |app: &mut slot::app::App, btn| {
        app.apply(Action::GbaDown(btn));
        app.apply(Action::GbaUp(btn));
    };
    app.apply(Action::QuickMenu);
    for row in [QuickRow::General, QuickRow::Wifi, QuickRow::Networks] {
        for _ in 0..24 {
            if app.quick_menu() == Some(row) {
                break;
            }
            press(&mut app, Btn::Down);
        }
        assert_eq!(app.quick_menu(), Some(row));
        press(&mut app, Btn::A);
    }
    assert!(matches!(app.phase(), Phase::Networks { .. }));
    assert_eq!(
        app.network_screen().unwrap().message,
        Some(slot::networks::STOCK_SCAN_MESSAGE)
    );
    press(&mut app, Btn::A);
    assert_eq!(
        app.network_screen().unwrap().message,
        Some(slot::networks::SCANNING_MESSAGE)
    );
    for _ in 0..4 {
        app.tick_ms(app.now() + 1000);
    }
    let screen = app.network_screen().unwrap();
    assert_eq!(
        screen
            .lines
            .iter()
            .map(|l| l.label.as_str())
            .collect::<Vec<_>>(),
        ["Demo"]
    );
    assert_eq!(screen.message, Some(slot::networks::STOCK_SCAN_MESSAGE));
    assert_eq!(app.network_action(), None);
    press(&mut app, Btn::A);
    assert!(app.keyboard().is_none());
    assert!(matches!(app.phase(), Phase::NetworkScan { .. }));
    assert_eq!(
        *calls.lock().unwrap(),
        ["ATTACH", "SCAN", "SCAN_RESULTS", "DETACH", "close"]
    );
    for (error, message) in [
        (
            ScanError::Unavailable,
            "Wi-Fi unavailable; use stock Settings",
        ),
        (ScanError::TimedOut, "Scan timed out; use stock Settings"),
        (ScanError::Failed, "Scan failed; use stock Settings"),
    ] {
        let (b, _) = scanner(vec![Err(error)]);
        app.set_wifi(Wifi::inline(Box::new(b)));
        for _ in 0..4 {
            app.tick_ms(app.now() + 1000);
        }
        assert_eq!(app.network_screen().unwrap().message, Some(message));
        assert!(app.network_screen().unwrap().lines.is_empty());
        assert_eq!(app.network_back(), "Back");
    }
}

#[derive(Clone)]
struct Profile {
    id: u32,
    ssid: String,
    enabled: bool,
}

struct StationState {
    profiles: Vec<Profile>,
    current: u32,
    lease: bool,
    disconnected: bool,
    saved: Vec<Profile>,
    commands: Vec<String>,
    refuse: Option<String>,
}

impl Default for StationState {
    fn default() -> Self {
        Self {
            profiles: vec![
                Profile {
                    id: 2,
                    ssid: "Old".into(),
                    enabled: true,
                },
                Profile {
                    id: 9,
                    ssid: "Manual".into(),
                    enabled: false,
                },
            ],
            current: 2,
            lease: true,
            disconnected: false,
            saved: Vec::new(),
            commands: Vec::new(),
            refuse: None,
        }
    }
}

struct StationMock {
    state: Arc<Mutex<StationState>>,
    replies: VecDeque<String>,
}

impl Control for StationMock {
    fn open(&mut self) -> Result<(), ScanError> {
        Ok(())
    }
    fn close(&mut self) {}
    fn send(&mut self, command: &str, _until: Instant) -> Result<(), ScanError> {
        let mut state = self.state.lock().unwrap();
        state.commands.push(command.into());
        let fields: Vec<_> = command.split(' ').collect();
        let id = fields
            .get(1)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(42);
        let text = if state
            .refuse
            .as_deref()
            .is_some_and(|prefix| command.starts_with(prefix))
        {
            "FAIL\n".into()
        } else {
            match fields[0] {
                "PING" => "PONG\n".into(),
                "LIST_NETWORKS" => {
                    let mut text = "network id / ssid / bssid / flags\n".to_string();
                    for row in &state.profiles {
                        text += &format!(
                            "{}\t{}\tany\t{}\n",
                            row.id,
                            row.ssid,
                            if row.enabled { "" } else { "[DISABLED]" }
                        );
                    }
                    text
                }
                "STATUS" => {
                    if state.disconnected {
                        // The interface may retain a lease after disassociation.
                        self.replies
                            .push_back("wpa_state=DISCONNECTED\nip_address=192.168.5.87\n".into());
                        return Ok(());
                    }
                    let ssid = state
                        .profiles
                        .iter()
                        .find(|row| row.id == state.current)
                        .map(|row| row.ssid.as_str())
                        .unwrap_or("");
                    format!(
                        "wpa_state=COMPLETED\nid={}\nssid={ssid}\n{}",
                        state.current,
                        if state.lease {
                            "ip_address=192.168.5.87\n"
                        } else {
                            ""
                        }
                    )
                }
                "ADD_NETWORK" => {
                    state.profiles.push(Profile {
                        id: 42,
                        ssid: "New".into(),
                        enabled: false,
                    });
                    "42\n".into()
                }
                "SELECT_NETWORK" => {
                    state.disconnected = false;
                    state.current = id;
                    for row in &mut state.profiles {
                        row.enabled = row.id == id;
                    }
                    "OK\n".into()
                }
                "DISCONNECT" => {
                    state.disconnected = true;
                    "OK\n".into()
                }
                "ENABLE_NETWORK" | "DISABLE_NETWORK" => {
                    for row in &mut state.profiles {
                        if row.id == id {
                            row.enabled = fields[0] == "ENABLE_NETWORK";
                        }
                    }
                    "OK\n".into()
                }
                "REMOVE_NETWORK" => {
                    state.profiles.retain(|row| row.id != id);
                    "OK\n".into()
                }
                "SAVE_CONFIG" => {
                    state.saved = state.profiles.clone();
                    "OK\n".into()
                }
                "SET_NETWORK" | "ATTACH" | "DETACH" => "OK\n".into(),
                "SCAN" => {
                    self.replies.push_back("OK\n".into());
                    "<3>CTRL-EVENT-SCAN-RESULTS\n".into()
                }
                "SCAN_RESULTS" => {
                    format!("{HEADER}00:11:22:33:44:55\t2412\t-45\t[WPA2-EAP-CCMP][ESS]\tOffice\n")
                }
                _ => panic!("unexpected control verb"),
            }
        };
        self.replies.push_back(text);
        Ok(())
    }
    fn recv(&mut self, _until: Instant) -> Result<String, ScanError> {
        self.replies.pop_front().ok_or(ScanError::TimedOut)
    }
}

fn station() -> (
    slot::wifi::stock_station::Station<StationMock>,
    Arc<Mutex<StationState>>,
) {
    let state = Arc::new(Mutex::new(StationState::default()));
    (
        slot::wifi::stock_station::Station::new(StationMock {
            state: state.clone(),
            replies: VecDeque::new(),
        }),
        state,
    )
}

struct LeaseClient(Arc<Mutex<Result<Option<String>, slot::wifi::Failure>>>);
impl slot::wifi::stock_dhcp::Client for LeaseClient {
    fn poll(&mut self, _now: Instant) -> Result<Option<String>, slot::wifi::Failure> {
        self.0.lock().unwrap().clone()
    }
    fn stop(&mut self) {
        *self.0.lock().unwrap() = Ok(None);
    }
}

#[test]
fn stock_wake_handoff_observes_the_stock_address_without_reconnecting_or_adopting_dhcp() {
    use slot::networks::{page_rows, status_text, PageRow};
    use slot::wifi::Failure;
    for acquired in [false, true] {
        for failure in [Failure::DhcpBusy, Failure::NoIp] {
            let state = Arc::new(Mutex::new(StationState {
                lease: false,
                ..Default::default()
            }));
            let lease = Arc::new(Mutex::new(Ok(None)));
            let backend = slot::wifi::stock_station::Station::with_dhcp(
                StationMock {
                    state: state.clone(),
                    replies: VecDeque::new(),
                },
                Box::new(LeaseClient(lease.clone())),
            );
            let mut wifi = Wifi::inline(Box::new(backend));
            let now = Instant::now();
            wifi.send(Cmd::Hold);
            wifi.send(Cmd::Connect(2));
            wifi.poll(now);
            if acquired {
                *lease.lock().unwrap() = Ok(Some("192.0.2.37".into()));
            }
            wifi.poll(now + Duration::from_secs(1));
            *lease.lock().unwrap() = Err(failure);
            wifi.poll(now + Duration::from_secs(4));
            assert_eq!(wifi.snapshot().failure.as_ref().unwrap().1, failure);
            assert!(wifi.snapshot().status.ip.is_none());
            wifi.poll(now + Duration::from_secs(7));
            assert!(wifi.snapshot().status.ip.is_none());
            assert_eq!(status_text(wifi.snapshot(), "Old", false), failure.label());

            // Stock wake policy now reports its own address. This is not an
            // owned lease event and must not cause any connection mutation.
            state.lock().unwrap().lease = true;
            state.lock().unwrap().commands.clear();
            wifi.poll(now + Duration::from_secs(10));
            if failure == Failure::DhcpBusy {
                assert_eq!(status_text(wifi.snapshot(), "Old", false), "Connected");
                assert_eq!(wifi.snapshot().status.ip.as_deref(), Some("192.168.5.87"));
                assert!(wifi.snapshot().failure.is_none());
                assert_eq!(
                    page_rows(wifi.snapshot(), "Old", false),
                    [PageRow::Status, PageRow::Ip, PageRow::Disconnect]
                );
                wifi.poll(now + Duration::from_secs(13));
                assert!(wifi.snapshot().status.ip.is_some());
                state.lock().unwrap().disconnected = true;
                wifi.poll(now + Duration::from_secs(16));
                assert!(wifi.snapshot().status.ip.is_none());
            } else {
                assert_eq!(status_text(wifi.snapshot(), "Old", false), failure.label());
                assert!(wifi.snapshot().status.ip.is_none());
            }
            assert!(state
                .lock()
                .unwrap()
                .commands
                .iter()
                .all(|command| { command == "STATUS" || command == "LIST_NETWORKS" }));
            assert!(state.lock().unwrap().saved.is_empty());
        }
    }
}

#[test]
fn stock_join_without_a_stock_dhcp_client_waits_for_owned_lease_before_saving() {
    use slot::wifi::{Failure, Phase};
    let state = Arc::new(Mutex::new(StationState {
        lease: false,
        ..Default::default()
    }));
    let lease = Arc::new(Mutex::new(Ok(None)));
    let backend = slot::wifi::stock_station::Station::with_dhcp(
        StationMock {
            state: state.clone(),
            replies: VecDeque::new(),
        },
        Box::new(LeaseClient(lease.clone())),
    );
    let mut wifi = Wifi::inline(Box::new(backend));
    let now = Instant::now();
    wifi.send(Cmd::Hold);
    wifi.send(Cmd::Join {
        ssid: "New".into(),
        password: "example-only".into(),
        hidden: false,
    });
    wifi.poll(now);
    assert_eq!(
        wifi.snapshot().attempt.as_ref().unwrap().1,
        Phase::GettingIp
    );
    assert!(wifi.snapshot().status.ip.is_none());
    assert!(state.lock().unwrap().saved.is_empty());
    *lease.lock().unwrap() = Ok(Some("192.0.2.37".into()));
    wifi.poll(now + Duration::from_secs(1));
    assert_eq!(wifi.snapshot().status.ip.as_deref(), Some("192.0.2.37"));
    assert!(state.lock().unwrap().saved.iter().any(|p| p.ssid == "New"));
    wifi.send(Cmd::Release);
    wifi.poll(now + Duration::from_secs(2));
    assert!(
        wifi.snapshot().held,
        "renewal must survive closing the Wi-Fi page"
    );
    assert!(wifi.snapshot().status.ip.is_some());
    wifi.send(Cmd::Disconnect);
    wifi.poll(now + Duration::from_secs(3));
    assert!(wifi.snapshot().status.ip.is_none());
    wifi.send(Cmd::Connect(42));
    wifi.poll(now + Duration::from_secs(4));
    assert!(
        wifi.snapshot().status.ip.is_none(),
        "old address must not complete reconnect"
    );
    *lease.lock().unwrap() = Err(Failure::DhcpBusy);
    wifi.poll(now + Duration::from_secs(5));
    assert_eq!(
        wifi.snapshot().failure.as_ref().unwrap().1,
        Failure::DhcpBusy
    );
    wifi.send(Cmd::Connect(42));
    wifi.poll(now + Duration::from_secs(6));
    *lease.lock().unwrap() = Ok(Some("192.0.2.38".into()));
    wifi.poll(now + Duration::from_secs(7));
    assert_eq!(wifi.snapshot().status.ip.as_deref(), Some("192.0.2.38"));
    assert!(wifi.snapshot().failure.is_none());
}

#[test]
fn stock_join_saves_only_confirmed_profile_and_preserves_existing_profiles_and_policy() {
    let (mut backend, state) = station();
    assert!(backend.start());
    let id = backend.add("New", "mock-\"secret\\", false).unwrap();
    assert!(state.lock().unwrap().saved.is_empty());
    backend.connect(id);
    assert_eq!(backend.status().ip.as_deref(), Some("192.168.5.87"));
    backend.finish_connect();
    assert_eq!(backend.connection_error(), None);
    let mut s = state.lock().unwrap();
    assert_eq!(
        s.saved
            .iter()
            .map(|p| (p.id, p.enabled))
            .collect::<Vec<_>>(),
        [(2, true), (9, false), (42, true)]
    );
    assert!(s.commands.contains(&"SET_NETWORK 42 ssid 4e6577".into()));
    assert!(s
        .commands
        .contains(&"SET_NETWORK 42 psk \"mock-\\\"secret\\\\\"".into()));
    s.commands.clear();
    drop(s);
    backend.stop();
    backend.block();
    backend.forget(2);
    backend.set_auto_join(2, false);
    assert!(state.lock().unwrap().commands.is_empty());
    assert_eq!(
        backend.saved().iter().map(|p| p.id).collect::<Vec<_>>(),
        [42, 2, 9]
    );
}

#[test]
fn stock_association_without_ip_times_out_without_dhcp_takeover_or_saved_bad_profile() {
    use slot::wifi::{Failure, Phase};
    let (backend, state) = station();
    state.lock().unwrap().lease = false;
    let mut wifi = Wifi::inline(Box::new(backend));
    let now = Instant::now();
    wifi.send(Cmd::Hold);
    wifi.send(Cmd::Join {
        ssid: "New".into(),
        password: "mock-password".into(),
        hidden: false,
    });
    wifi.poll(now);
    assert_eq!(
        wifi.snapshot().attempt,
        Some(("New".into(), Phase::GettingIp))
    );
    assert_eq!(
        slot::networks::status_text(wifi.snapshot(), "New", false),
        "Getting IP"
    );
    wifi.poll(now + Duration::from_secs(21));
    assert_eq!(wifi.snapshot().failure, Some(("New".into(), Failure::NoIp)));
    let s = state.lock().unwrap();
    assert_eq!(
        s.profiles
            .iter()
            .map(|p| (p.id, p.enabled))
            .collect::<Vec<_>>(),
        [(2, true), (9, false)]
    );
    assert_eq!(s.current, 2);
    assert!(s.saved.is_empty());
    assert!(!s.commands.iter().any(|s| s == "SAVE_CONFIG"));
}

#[test]
fn stock_rejected_configuration_rolls_back_only_its_new_profile() {
    let (mut backend, state) = station();
    state.lock().unwrap().refuse = Some("SET_NETWORK 42 psk".into());
    assert_eq!(backend.add("New", "mock-password", true), None);
    let s = state.lock().unwrap();
    assert_eq!(
        s.profiles
            .iter()
            .map(|p| (p.id, p.enabled))
            .collect::<Vec<_>>(),
        [(2, true), (9, false)]
    );
    assert!(s.commands.contains(&"REMOVE_NETWORK 42".into()));
    assert!(!s
        .commands
        .iter()
        .any(|s| s == "SAVE_CONFIG" || s == "REMOVE_NETWORK 2" || s == "REMOVE_NETWORK 9"));
}

#[test]
fn stock_save_failure_is_visible_without_disconnecting_or_claiming_persistence() {
    use slot::wifi::Failure;
    let (backend, state) = station();
    state.lock().unwrap().refuse = Some("SAVE_CONFIG".into());
    let mut wifi = Wifi::inline(Box::new(backend));
    let now = Instant::now();
    wifi.send(Cmd::Hold);
    wifi.send(Cmd::Join {
        ssid: "New".into(),
        password: "mock-password".into(),
        hidden: false,
    });
    wifi.poll(now);
    assert_eq!(
        wifi.snapshot().failure,
        Some(("New".into(), Failure::SaveUnconfirmed))
    );
    assert_eq!(
        slot::networks::status_text(wifi.snapshot(), "New", false),
        "Connected; Save Unconfirmed"
    );
    assert_eq!(wifi.snapshot().status.ip.as_deref(), Some("192.168.5.87"));
    assert!(state.lock().unwrap().saved.is_empty());
    assert_eq!(state.lock().unwrap().current, 42);
}

#[test]
fn stock_unsupported_security_and_invalid_credentials_cannot_mutate_networks() {
    use slot::wifi::Failure;
    let (mut backend, state) = station();
    assert_eq!(backend.scan()[0].ssid, "Office");
    state.lock().unwrap().commands.clear();
    assert_eq!(backend.add("Office", "mock-password", false), None);
    assert_eq!(backend.connection_error(), Some(Failure::Unsupported));
    for (ssid, password) in [
        ("New", "bad\npassword"),
        ("New", "short"),
        ("", "mock-password"),
        ("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456", "mock-password"),
    ] {
        assert_eq!(backend.add(ssid, password, false), None);
    }
    assert!(state.lock().unwrap().commands.is_empty());
}

#[test]
fn stock_station_ui_exposes_join_and_disconnect_but_not_radio_autojoin_or_forget() {
    use slot::networks::{page_rows, PageRow};
    let (backend, state) = station();
    let mut wifi = Wifi::inline(Box::new(backend));
    let now = Instant::now();
    wifi.send(Cmd::Hold);
    wifi.poll(now);
    assert_eq!(
        page_rows(wifi.snapshot(), "Old", false),
        [PageRow::Status, PageRow::Ip, PageRow::Disconnect]
    );
    assert_eq!(
        page_rows(wifi.snapshot(), "Manual", false),
        [PageRow::Status, PageRow::Ip, PageRow::Connect]
    );
    let before = state.lock().unwrap().commands.len();
    for command in [
        Cmd::PowerOff,
        Cmd::Forget(2),
        Cmd::AutoJoin { id: 2, on: false },
    ] {
        wifi.send(command);
    }
    wifi.poll(now);
    assert_eq!(state.lock().unwrap().commands.len(), before);
    assert!(wifi.snapshot().held);
    let d = common::tmp_root_with_carts(&[]);
    slot_store::write_slot_state(
        d.path(),
        &slot_store::SlotState {
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    let (mut app, _) = common::app_booting_at(d.path(), common::CLOCK_IS_SET);
    app.set_wifi(wifi);
    assert!(app.hidden_rows().contains(&slot_ui::QuickRow::WifiPower));
    assert!(!app.hidden_rows().contains(&slot_ui::QuickRow::Networks));
    let press = |app: &mut slot::app::App, btn| {
        app.apply(slot_input::Action::GbaDown(btn));
        app.apply(slot_input::Action::GbaUp(btn));
    };
    use slot_input::{Action, Btn};
    use slot_ui::QuickRow;
    app.apply(Action::QuickMenu);
    for row in [QuickRow::General, QuickRow::Wifi, QuickRow::Networks] {
        for _ in 0..24 {
            if app.quick_menu() == Some(row) {
                break;
            }
            press(&mut app, Btn::Down);
        }
        assert_eq!(app.quick_menu(), Some(row));
        press(&mut app, Btn::A);
    }
    press(&mut app, Btn::A);
    press(&mut app, Btn::Down);
    press(&mut app, Btn::Down);
    assert_eq!(app.network_action(), Some("Disconnect"));
    press(&mut app, Btn::A);
    app.tick_ms(app.now() + 4000);
    assert_eq!(app.network_action(), Some("Connect"));
    assert!(app.wifi().status.ip.is_none());
    assert_eq!(app.wifi().saved.len(), 2);
}

#[test]
fn stock_disconnect_preserves_profiles_and_can_reconnect_without_a_password() {
    use slot::networks::{page_rows, status_text, PageRow};
    let (backend, state) = station();
    let mut wifi = Wifi::inline(Box::new(backend));
    let now = Instant::now();
    wifi.send(Cmd::Hold);
    wifi.poll(now);
    wifi.send(Cmd::Disconnect);
    wifi.poll(now + Duration::from_secs(1));
    assert_eq!(status_text(wifi.snapshot(), "Old", false), "Not Connected");
    assert!(wifi.snapshot().status.ip.is_none());
    assert_eq!(
        page_rows(wifi.snapshot(), "Old", false),
        [PageRow::Status, PageRow::Ip, PageRow::Connect]
    );
    {
        let s = state.lock().unwrap();
        assert_eq!(
            s.profiles
                .iter()
                .map(|p| (p.id, p.enabled))
                .collect::<Vec<_>>(),
            [(2, true), (9, false)]
        );
        assert!(s.saved.is_empty());
        assert!(s.commands.iter().any(|c| c == "DISCONNECT"));
        assert!(!s
            .commands
            .iter()
            .any(|c| c.starts_with("REMOVE_NETWORK") || c == "SAVE_CONFIG"));
    }
    wifi.send(Cmd::Connect(2));
    wifi.poll(now + Duration::from_secs(2));
    assert_eq!(status_text(wifi.snapshot(), "Old", false), "Connected");
    assert!(wifi.snapshot().failure.is_none());
}

#[test]
fn refused_stock_disconnect_is_visible_and_retry_does_not_forget_the_network() {
    use slot::networks::status_text;
    let (backend, state) = station();
    let mut wifi = Wifi::inline(Box::new(backend));
    let now = Instant::now();
    wifi.send(Cmd::Hold);
    wifi.poll(now);
    state.lock().unwrap().refuse = Some("DISCONNECT".into());
    wifi.send(Cmd::Disconnect);
    wifi.poll(now + Duration::from_secs(1));
    assert_eq!(
        status_text(wifi.snapshot(), "Old", false),
        "Could Not Disconnect"
    );
    assert!(wifi.snapshot().status.ip.is_some());
    state.lock().unwrap().refuse = None;
    wifi.send(Cmd::Disconnect);
    wifi.poll(now + Duration::from_secs(2));
    assert_eq!(status_text(wifi.snapshot(), "Old", false), "Not Connected");
    assert_eq!(wifi.snapshot().saved.len(), 2);
    assert!(wifi.snapshot().failure.is_none());
}

#[test]
fn stock_unconfirmed_add_blocks_later_mutations_and_unfinished_join_is_cleaned_up() {
    let (mut backend, state) = station();
    state.lock().unwrap().refuse = Some("ADD_NETWORK".into());
    assert_eq!(backend.add("New", "mock-password", false), None);
    state.lock().unwrap().refuse = None;
    state.lock().unwrap().commands.clear();
    assert_eq!(backend.add("New", "mock-password", false), None);
    backend.connect(2);
    assert!(state.lock().unwrap().commands.is_empty());

    let (mut backend, state) = station();
    let id = backend.add("New", "mock-password", false).unwrap();
    backend.connect(id);
    drop(backend);
    let s = state.lock().unwrap();
    assert_eq!(s.current, 2);
    assert_eq!(
        s.profiles
            .iter()
            .map(|p| (p.id, p.enabled))
            .collect::<Vec<_>>(),
        [(2, true), (9, false)]
    );
    assert!(s.saved.is_empty());
}

#[test]
fn stock_duplicate_ssid_on_another_profile_does_not_confirm_the_join() {
    let (mut backend, state) = station();
    let id = backend.add("New", "mock-password", false).unwrap();
    backend.connect(id);
    {
        let mut s = state.lock().unwrap();
        s.current = 2;
        s.profiles[0].ssid = "New".into();
    }
    backend.status();
    assert_eq!(backend.connection_error(), Some(slot::wifi::Failure::Other));
    backend.finish_connect();
    assert!(state.lock().unwrap().saved.is_empty());
}

#[test]
fn stock_startup_before_supplicant_ready_allows_first_explicit_connect_without_disconnect() {
    use slot::networks::{page_rows, PageRow};
    use slot::wifi::Phase;
    let state = Arc::new(Mutex::new(StationState {
        profiles: vec![Profile {
            id: 0,
            ssid: "Synthetic".into(),
            enabled: true,
        }],
        current: 0,
        lease: false,
        refuse: Some("LIST_NETWORKS".into()),
        ..Default::default()
    }));
    let lease = Arc::new(Mutex::new(Ok(None)));
    let station = slot::wifi::stock_station::Station::with_dhcp(
        StationMock {
            state: state.clone(),
            replies: VecDeque::new(),
        },
        Box::new(LeaseClient(lease.clone())),
    );
    // Slot starts before the stock control endpoint, which becomes available
    // before the owner opens Wi-Fi. Association already exists but no lease.
    let mut wifi = Wifi::inline(Box::new(station));
    state.lock().unwrap().refuse = None;
    let now = Instant::now();
    wifi.send(Cmd::Hold);
    wifi.poll(now);
    wifi.poll(now + Duration::from_secs(1));
    assert!(
        wifi.snapshot().failure.is_none(),
        "startup error must not be latched"
    );
    assert!(wifi.snapshot().status.ip.is_none());
    assert_eq!(
        page_rows(wifi.snapshot(), "Synthetic", false),
        [PageRow::Status, PageRow::Ip, PageRow::Connect]
    );
    // Only explicit Connect requests DHCP; opening the page does not.
    wifi.send(Cmd::Connect(0));
    wifi.poll(now + Duration::from_secs(2));
    assert_eq!(
        wifi.snapshot().attempt.as_ref().unwrap().1,
        Phase::GettingIp
    );
    *lease.lock().unwrap() = Ok(Some("192.0.2.37".into()));
    wifi.poll(now + Duration::from_secs(3));
    assert_eq!(wifi.snapshot().status.ip.as_deref(), Some("192.0.2.37"));
    assert!(wifi.snapshot().failure.is_none());
    assert_eq!(
        page_rows(wifi.snapshot(), "Synthetic", false),
        [PageRow::Status, PageRow::Ip, PageRow::Disconnect]
    );
}
