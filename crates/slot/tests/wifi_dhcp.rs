use slot::wifi::stock_dhcp::{Client, Lease, Owner, Runtime};
use slot::wifi::Failure;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Network {
    competing: bool,
    clients: usize,
    dead: bool,
    lease: Option<Lease>,
    events: VecDeque<String>,
}
struct Fake(Arc<Mutex<Network>>);
impl Runtime for Fake {
    fn competitor(&self) -> bool {
        self.0.lock().unwrap().competing
    }
    fn start(&mut self) -> Result<(), Failure> {
        self.0.lock().unwrap().clients += 1;
        Ok(())
    }
    fn alive(&mut self) -> bool {
        !self.0.lock().unwrap().dead
    }
    fn event(&mut self) -> Result<Option<String>, Failure> {
        Ok(self.0.lock().unwrap().events.pop_front())
    }
    fn apply(&mut self, lease: &Lease) -> Result<(), Failure> {
        self.0.lock().unwrap().lease = Some(lease.clone());
        Ok(())
    }
    fn stop(&mut self) {
        let mut n = self.0.lock().unwrap();
        n.clients = 0;
        n.lease = None;
    }
}
fn owner() -> (Owner<Fake>, Arc<Mutex<Network>>) {
    let net = Arc::new(Mutex::new(Network::default()));
    (Owner::new(Fake(net.clone())), net)
}
fn bound(seconds: u32) -> String {
    format!("bound\nwlan0\n192.0.2.37\n255.255.255.0\n192.0.2.1\n192.0.2.53\n{seconds}\n\n\n\n")
}

#[test]
fn missing_client_acquires_once_renews_and_disconnect_allows_a_fresh_connect() {
    let (mut client, net) = owner();
    let now = Instant::now();
    assert_eq!(client.poll(now), Ok(None));
    assert_eq!(client.poll(now + Duration::from_secs(1)), Ok(None));
    assert_eq!(net.lock().unwrap().clients, 1);
    net.lock().unwrap().events.push_back(bound(5));
    assert_eq!(
        client.poll(now + Duration::from_secs(2)),
        Ok(Some("192.0.2.37".into()))
    );
    net.lock()
        .unwrap()
        .events
        .push_back(bound(12).replacen("bound", "renew", 1));
    assert!(client.poll(now + Duration::from_secs(6)).unwrap().is_some());
    assert!(client
        .poll(now + Duration::from_secs(10))
        .unwrap()
        .is_some());
    assert_eq!(net.lock().unwrap().clients, 1);
    client.stop();
    assert!(net.lock().unwrap().lease.is_none());
    assert_eq!(client.poll(now + Duration::from_secs(11)), Ok(None));
    net.lock().unwrap().events.push_back(bound(60));
    assert!(client
        .poll(now + Duration::from_secs(12))
        .unwrap()
        .is_some());
    drop(client);
    assert_eq!(net.lock().unwrap().clients, 0);
}

#[test]
fn competitors_timeout_exit_expiry_and_deconfig_fail_closed_and_are_retryable() {
    for failure in ["competitor", "timeout", "exit", "expiry", "deconfig"] {
        let (mut client, net) = owner();
        let now = Instant::now();
        if failure == "competitor" {
            net.lock().unwrap().competing = true;
        }
        if failure == "competitor" {
            assert_eq!(client.poll(now), Err(Failure::DhcpBusy));
        } else {
            assert_eq!(client.poll(now), Ok(None));
            if ["expiry", "deconfig"].contains(&failure) {
                net.lock().unwrap().events.push_back(bound(2));
                assert!(client.poll(now).unwrap().is_some());
            }
            if failure == "exit" {
                net.lock().unwrap().dead = true;
            }
            if failure == "deconfig" {
                net.lock()
                    .unwrap()
                    .events
                    .push_back("deconfig\nwlan0\n\n\n\n\n\n\n\n\n".into());
            }
            assert_eq!(
                client.poll(now + Duration::from_secs(12)),
                Err(Failure::NoIp)
            );
        }
        assert_eq!(net.lock().unwrap().clients, 0);
        assert!(net.lock().unwrap().lease.is_none());
        client.stop();
        {
            let mut n = net.lock().unwrap();
            n.competing = false;
            n.dead = false;
        }
        assert_eq!(client.poll(now + Duration::from_secs(13)), Ok(None));
    }
}

#[test]
fn a_stock_client_appearing_after_acquisition_stops_only_the_owned_lifecycle() {
    let (mut client, net) = owner();
    let now = Instant::now();
    client.poll(now).unwrap();
    net.lock().unwrap().events.push_back(bound(60));
    assert!(client.poll(now).unwrap().is_some());
    net.lock().unwrap().competing = true;
    assert_eq!(client.poll(now), Err(Failure::DhcpBusy));
    assert!(net.lock().unwrap().competing);
    assert_eq!(net.lock().unwrap().clients, 0);
}

#[test]
fn invalid_or_other_interface_leases_never_configure_the_network() {
    for event in [
        bound(0),
        bound(30).replace("wlan0", "eth0"),
        bound(30).replace("255.255.255.0", "255.0.255.0"),
        bound(30).replace("192.0.2.1\n", "198.51.100.1\n"),
        bound(30).replace("192.0.2.53", "1.1.1.1; touch /tmp/not-a-command"),
        bound(30).replace("\n\n\n\n", "\n\n0.0.0.0/0 192.0.2.1\n\n"),
    ] {
        let (mut client, net) = owner();
        let now = Instant::now();
        client.poll(now).unwrap();
        net.lock().unwrap().events.push_back(event);
        assert_eq!(client.poll(now), Err(Failure::NoIp));
        assert!(net.lock().unwrap().lease.is_none());
    }
}
