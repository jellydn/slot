use slot::romm::api::{
    content_path, device_name, parse_installs, parse_pending, parse_poll, parse_rom, parse_version,
    report_body, status_error,
};
use slot::romm::{http_api, version_ok, ApiError, Outcome, Poll, RomFile, RommConf};

#[test]
fn version_table() {
    for (v, want) in [
        ("5.4.0", true),
        ("5.10.0", true),
        ("6.0.0", true),
        ("5.3.1", false),
        ("5.4.0-beta.1", true),
        ("development", true),
        ("garbage", true),
        ("5.x.0", false),
    ] {
        assert_eq!(version_ok(v), want, "{v}");
    }
}

#[test]
fn pairing_names_the_device_by_model() {
    assert_eq!(device_name(Some("rg35xxsp")), "slot on rg35xxsp");
    assert_eq!(device_name(None), "slot");
}

#[test]
fn rom_files_are_parsed() {
    let rom = parse_rom(
        r#"{"platform_slug":"gba","fs_name":"Folder","files":[{"id":7,"file_name":"Game.gba"}]}"#,
    )
    .unwrap();
    assert_eq!(
        rom.files,
        vec![RomFile {
            id: 7,
            file_name: "Game.gba".into()
        }]
    );
}

#[test]
fn poll_maps_every_pending_detail() {
    for (detail, want) in [
        ("authorization_pending", Poll::Waiting),
        ("slow_down", Poll::SlowDown),
        ("access_denied", Poll::Denied),
        ("expired_token", Poll::Expired),
    ] {
        let body = format!(r#"{{"detail":"{detail}"}}"#);
        assert_eq!(parse_poll(400, &body), Ok(want), "{detail}");
    }
}

#[test]
fn poll_success_carries_token_and_device() {
    let body = r#"{"access_token":"rmm_x","device_id":"d1","scopes":[],"expires_at":null}"#;
    assert_eq!(
        parse_poll(200, body),
        Ok(Poll::Paired {
            token: "rmm_x".into(),
            device_id: "d1".into()
        })
    );
}

#[test]
fn status_mapping() {
    assert_eq!(status_error(401), ApiError::Unauthorized);
    assert_eq!(status_error(404), ApiError::NotFound);
    assert_eq!(status_error(409), ApiError::Client(409));
    assert_eq!(status_error(503), ApiError::Server(503));
}

#[test]
fn connection_failure_is_offline() {
    let conf = RommConf {
        protocol: slot::romm::Protocol::Http,
        server: "127.0.0.1".into(),
        port: Some(1),
        device_id: Some("d".into()),
        token: Some("t".into()),
        ..RommConf::default()
    };
    assert_eq!(http_api(&conf).version(), Err(ApiError::Offline));
}

#[test]
fn streamed_downloads_report_real_bytes_and_total_through_errors_and_cancellation() {
    use slot::romm::api::DownloadProgress;
    use slot::romm::{Cancel, Protocol};
    use std::io::{BufRead, Write};
    use std::net::TcpListener;
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};

    for mode in ["known", "unknown", "truncated", "cancel"] {
        let size = 3 * 1024 * 1024;
        let first = 64 * 1024;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (release, wait) = channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(&mut socket);
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
            }
            let length = if mode == "unknown" {
                String::new()
            } else {
                format!("Content-Length: {size}\r\n")
            };
            write!(
                socket,
                "HTTP/1.1 200 OK\r\n{length}Connection: close\r\n\r\n"
            )
            .unwrap();
            socket.write_all(&vec![0x59; first]).unwrap();
            wait.recv_timeout(Duration::from_secs(10)).unwrap();
            if mode != "truncated" {
                let _ = socket.write_all(&vec![0x59; size - first]);
            }
        });
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("download.part");
        let to = path.clone();
        let cancel = Cancel::default();
        let transfer = cancel.clone();
        let worker = std::thread::spawn(move || {
            http_api(&RommConf {
                protocol: Protocol::Http,
                server: "127.0.0.1".into(),
                port: Some(port),
                ..RommConf::default()
            })
            .download(42, "Large.chd", &[], &to, &transfer)
        });
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if matches!(cancel.progress(), Some(DownloadProgress::Downloading { bytes, .. }) if bytes == first as u64)
            {
                break;
            }
            assert!(Instant::now() < deadline, "{mode}: no live progress");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            cancel.progress(),
            Some(DownloadProgress::Downloading {
                bytes: first as u64,
                total: (mode != "unknown").then_some(size as u64),
            })
        );
        if mode == "cancel" {
            cancel.set();
        }
        release.send(()).unwrap();
        let result = worker.join().unwrap();
        server.join().unwrap();
        match mode {
            "truncated" => assert_eq!(result, Err(ApiError::Truncated)),
            "cancel" => assert_eq!(result, Err(ApiError::Cancelled)),
            _ => {
                assert_eq!(result, Ok(()));
                assert_eq!(std::fs::read(path).unwrap(), vec![0x59; size]);
                assert_eq!(
                    cancel.progress(),
                    Some(DownloadProgress::Downloading {
                        bytes: size as u64,
                        total: (mode != "unknown").then_some(size as u64),
                    })
                );
            }
        }
    }
}

#[test]
fn new_core_libraries_download_exact_bytes_over_http_using_server_platform_ids() {
    use slot::romm::{Cancel, Protocol};
    use std::io::{BufRead, Write};
    use std::net::TcpListener;
    for (wanted, server_slug, name, bytes, folder) in [
        (
            "psx",
            "ps",
            "Disc.chd",
            b"MComprHDauthored".as_slice(),
            "PS",
        ),
        (
            "pico",
            "pico8",
            "Cart.p8",
            include_bytes!("../../slot-retro/tests/fixtures/persistence.p8").as_slice(),
            "PICO8",
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            for _ in 0..4 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut reader = std::io::BufReader::new(&mut socket);
                let mut request = String::new();
                reader.read_line(&mut request).unwrap();
                let path = request.split_whitespace().nth(1).unwrap();
                let mut authorized = false;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    authorized |= line
                        .to_ascii_lowercase()
                        .starts_with("authorization: bearer test-token");
                }
                assert!(authorized);
                let rom = serde_json::json!({"id":42,"name":"Authored fixture","platform_slug":server_slug,"fs_name":"Folder","files":[{"id":91,"file_name":name}]});
                let body = match path {
                    "/api/platforms" => serde_json::json!([{"id":99,"slug":"sega-pico"},{"id":73,"slug":server_slug}]).to_string().into_bytes(),
                    "/api/roms?platform_ids=73&limit=20&offset=0&order_by=name&order_dir=asc&with_files=true" => serde_json::json!({"items":[rom],"total":1}).to_string().into_bytes(),
                    "/api/roms/42" => rom.to_string().into_bytes(),
                    path => {
                        assert_eq!(path, format!("/api/roms/42/content/{name}?file_ids=91"));
                        bytes.to_vec()
                    }
                };
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                socket.write_all(&body).unwrap();
            }
        });
        let conf = RommConf {
            protocol: Protocol::Http,
            server: "127.0.0.1".into(),
            port: Some(port),
            token: Some("test-token".into()),
            ..RommConf::default()
        };
        let api = http_api(&conf);
        let page = api.library(wanted, 0).unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(page.items[0].supported());
        let d = tempfile::tempdir().unwrap();
        slot::root::ensure(d.path());
        let done = slot::romm::install::install(
            d.path(),
            api.as_ref(),
            &page.items[0].install(),
            &Cancel::default(),
            |_| u64::MAX,
        )
        .unwrap();
        assert_eq!(done.outcome, Outcome::Done);
        assert_eq!(
            std::fs::read(d.path().join("Games").join(folder).join(name)).unwrap(),
            bytes
        );
        server.join().unwrap();
    }
}

#[test]
fn parses_the_response_fixtures() {
    assert_eq!(
        parse_version(r#"{"SYSTEM":{"VERSION":"5.4.0"}}"#),
        Ok("5.4.0".into())
    );
    let pending = parse_pending(
        r#"{"device_code":"dc","user_code":"ABCD-EFGH","verification_path":"/pair/device","verification_path_complete":"/pair/device?user_code=ABCD-EFGH","expires_in":600,"interval":5}"#,
    )
    .unwrap();
    assert_eq!(pending.path, "/pair/device?user_code=ABCD-EFGH");
    assert_eq!(pending.interval, 5);
    let installs = parse_installs(
        r#"[{"id":"i1","user_id":1,"device_id":"d","rom_id":7,"file_ids":[3,4],"status":"taken"}]"#,
    )
    .unwrap();
    assert_eq!(installs[0].rom_id, 7);
    assert_eq!(installs[0].file_ids, vec![3, 4]);
    let rom = parse_rom(r#"{"platform_slug":"gba","fs_name":"Game (USA).zip"}"#).unwrap();
    assert_eq!(rom.platform_slug, "gba");
}

#[test]
fn builds_requests() {
    let pairing: serde_json::Value =
        serde_json::from_str(&slot::romm::api::pair_body(&RommConf::default())).unwrap();
    assert!(pairing["requested_scopes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s == "platforms.read"));
    assert_eq!(
        content_path(7, "Game (USA).zip", &[1, 2]),
        "/api/roms/7/content/Game%20%28USA%29.zip?file_ids=1,2"
    );
    assert_eq!(report_body(&Outcome::Done), r#"{"status":"done"}"#);
    assert_eq!(
        report_body(&Outcome::Failed("no".into())),
        r#"{"reason":"no","status":"failed"}"#
    );
}

mod loopback {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    use slot::romm::{http_api, ApiError, Cancel, Protocol, RommConf};

    fn serve(response: &'static [u8], hold: bool) -> (RommConf, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut seen = Vec::new();
            let mut buf = [0u8; 1024];
            while !seen.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = sock.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                seen.extend_from_slice(&buf[..n]);
            }
            let _ = tx.send(String::from_utf8_lossy(&seen).to_string());
            sock.write_all(response).unwrap();
            if hold {
                thread::sleep(std::time::Duration::from_millis(300));
            }
        });
        let conf = RommConf {
            protocol: Protocol::Http,
            server: "127.0.0.1".into(),
            port: Some(port),
            device_id: Some("d".into()),
            token: Some("secret-token".into()),
            ..RommConf::default()
        };
        (conf, rx)
    }

    #[test]
    fn pairing_requests_send_no_authorization() {
        let body = r#"{"device_code":"dc","user_code":"U","verification_path_complete":"/p","expires_in":1,"interval":1}"#;
        let response: &'static [u8] = Box::leak(
            format!(
                "HTTP/1.1 201 Created\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .into_boxed_str()
            .into_boxed_bytes(),
        );
        let (conf, rx) = serve(response, false);
        http_api(&conf).pair_start(&conf).unwrap();
        let request = rx.recv().unwrap().to_ascii_lowercase();
        assert!(request.contains("/api/auth/device/init"));
        assert!(!request.contains("authorization"));
    }

    #[test]
    fn authed_requests_send_the_bearer() {
        let (conf, rx) = serve(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]",
            false,
        );
        assert_eq!(http_api(&conf).claim(), Ok(vec![]));
        let request = rx.recv().unwrap().to_ascii_lowercase();
        assert!(request.contains("authorization: bearer secret-token"));
    }

    #[test]
    fn short_body_is_truncated() {
        let (conf, _rx) = serve(
            b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc",
            false,
        );
        let dir = tempfile::tempdir().unwrap();
        let result = http_api(&conf).download(
            1,
            "a.gba",
            &[],
            &dir.path().join("a.gba"),
            &Cancel::default(),
        );
        assert_eq!(result, Err(ApiError::Truncated));
    }

    #[test]
    fn cancel_before_the_request_never_connects() {
        let (conf, _rx) = serve(b"", false);
        let dir = tempfile::tempdir().unwrap();
        let cancel = Cancel::default();
        cancel.set();
        let result = http_api(&conf).download(1, "a.gba", &[], &dir.path().join("a.gba"), &cancel);
        assert_eq!(result, Err(ApiError::Cancelled));
        assert!(!dir.path().join("a.gba").exists());
    }

    #[test]
    fn unwritable_target_is_a_write_error() {
        let (conf, _rx) = serve(
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc",
            false,
        );
        let dir = tempfile::tempdir().unwrap();
        let result = http_api(&conf).download(
            1,
            "a.gba",
            &[],
            &dir.path().join("missing").join("a.gba"),
            &Cancel::default(),
        );
        assert!(matches!(result, Err(ApiError::Write(_))));
    }
}
