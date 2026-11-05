mod common;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use common::{
    app_playing_in, boot, clocked, panel, session_with_platform, tmp_root_with_carts,
    tmp_root_with_real_carts, StubSnapshot,
};
use slot::app::Phase;
use slot::emu::Speed;
use slot::session::Session;
use slot_gfx::Draw;
use slot_input::{Action, Btn, Millis, RawEvent, POWER_HOLD_MS};
use slot_store::{read_slot_state, write_slot_state, Core, Platform, SlotState, StateRing};

use slot::link_radio::{RadioJob, RadioJobs};

const FRAME_MS: Millis = 16;
const DT: f32 = 1.0 / 60.0;

#[test]
fn stock_power_gestures_checkpoint_without_local_doze_or_shutdown() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_stock_power_owned();
    a.apply(Action::PowerPress);
    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_some()
    );
    for action in [
        Action::PowerTap,
        Action::PowerPress,
        Action::PowerHold,
        Action::PowerHold,
        Action::PowerOff,
        Action::PowerTap,
    ] {
        a.apply(action);
        a.tick_ms(a.now() + 2_000);
        assert!(matches!(a.phase(), Phase::Playing { .. }));
        assert!(!a.powering_off() && !a.restarting() && !a.relaunching());
    }
}

#[test]
fn stock_lid_pause_does_not_expire_into_system_poweroff() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_stock_power_owned();
    a.set_power(panel(d.path(), Duration::from_secs(2)).0);
    a.apply(Action::LidClose);
    a.tick_ms(a.now() + 181_000);
    a.on_doze_timeout();
    assert!(matches!(a.phase(), Phase::Doze { .. }));
    assert!(!a.powering_off() && !a.relaunching());
    a.apply(Action::LidOpen);
    assert!(matches!(a.phase(), Phase::Playing { .. }));
}

#[test]
fn stock_missing_power_release_and_idle_do_not_request_exit() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    s.app_mut()
        .set_power(panel(d.path(), Duration::from_secs(180)).0);
    s.app_mut().set_stock_power_owned();
    let mut now = 0;
    hold_power(&mut s, &mut now);
    now += 181_000;
    s.feed([], now);
    s.update(DT);
    assert!(
        matches!(s.app().phase(), Phase::Shelf),
        "{:?}",
        s.app().phase()
    );
    assert!(!s.app().powering_off() && !s.app().relaunching());
    event(&mut s, RawEvent::Up(Btn::Power), &mut now);
    assert!(matches!(s.app().phase(), Phase::Shelf));
    assert!(!s.app().powering_off() && !s.app().relaunching());
}

#[test]
fn stock_critical_battery_checkpoints_and_exits_without_a_shutdown_screen() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_stock_power_owned();
    a.on_battery(slot_power::Battery {
        percent: 1,
        charge: slot_power::Charge::Charging,
    });
    assert!(!a.relaunching());
    a.on_battery(slot_power::Battery {
        percent: 1,
        charge: slot_power::Charge::Discharging,
    });
    assert!(a.relaunching());
    assert!(!a.powering_off() && !a.restarting());
    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_some()
    );
}

#[test]
fn stock_device_restart_requests_a_device_reboot_not_a_return_to_stock() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    a.set_power(panel(d.path(), Duration::from_secs(180)).0);
    a.set_stock_power_owned();
    a.apply(Action::QuickMenu);
    for _ in 0..slot_ui::QuickRow::DeviceSettings.position(&a.hidden_rows()) {
        a.apply(Action::GbaDown(Btn::Down));
        a.apply(Action::GbaUp(Btn::Down));
    }
    a.apply(Action::GbaDown(Btn::A));
    a.apply(Action::GbaUp(Btn::A));
    assert_eq!(a.quick_menu(), Some(slot_ui::QuickRow::DeviceRestart));
    a.apply(Action::GbaDown(Btn::A));
    assert!(a.restarting());
    assert!(!a.relaunching() && !a.powering_off());
    a.set_power(slot_power::Power::new(
        Box::new(Reboot {
            inner: slot_power::SimPlatform::at(d.path().into()),
            calls: Default::default(),
            fail: true,
            resume_required: false,
        }),
        Duration::from_secs(180),
    ));
    assert!(!a.restart());
    assert_eq!(a.quick_menu(), Some(slot_ui::QuickRow::DeviceRestart));
    let faces: Vec<_> = slot_ui::Toast::all()
        .iter()
        .enumerate()
        .map(|(n, _)| slot_gfx::TexId::from_raw(n + 900))
        .collect();
    let failed = faces[slot_ui::Toast::RestartFailed.index()];
    a.set_toast_faces(faces);
    let mut draw = Vec::new();
    a.draw(&mut draw);
    assert!(
        draw.iter()
            .any(|d| matches!(d, Draw::Tex { tex, .. } if *tex == failed)),
        "Device settings must draw the reboot failure, not only record it"
    );
}

#[test]
fn reboot_request_is_sent_once_and_failure_stays_in_slot_with_a_retry() {
    for fail in [false, true] {
        let d = tmp_root_with_carts(&["Emerald"]);
        let mut a = app_playing_in(d.path(), "Emerald");
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        a.set_stock_power_owned();
        a.set_power(slot_power::Power::new(
            Box::new(Reboot {
                inner: slot_power::SimPlatform::at(d.path().into()),
                calls: calls.clone(),
                fail,
                resume_required: true,
            }),
            Duration::from_secs(180),
        ));
        assert_eq!(a.restart(), !fail);
        assert!(!a.relaunching());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(
            StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
                .read_resume()
                .unwrap()
                .is_some()
        );
        if fail {
            assert!(!a.restarting());
            assert_eq!(a.toast(), Some(slot_ui::Toast::RestartFailed));
            assert!(!a.restart());
            assert_eq!(calls.load(Ordering::Relaxed), 2);
        } else {
            assert!(a.restart());
            assert!(!a.ready_to_restart());
            assert_eq!(calls.load(Ordering::Relaxed), 1);
        }
    }
}

#[test]
fn stock_device_shutdown_saves_requests_once_and_refusal_allows_retry() {
    for fail in [false, true] {
        let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
        let mut a = boot(d.path());
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        a.set_stock_power_owned();
        a.set_power(slot_power::Power::new(
            Box::new(Reboot {
                inner: slot_power::SimPlatform::at(d.path().into()),
                calls: calls.clone(),
                fail,
                resume_required: false,
            }),
            Duration::from_secs(180),
        ));
        a.apply(Action::QuickMenu);
        for _ in 0..slot_ui::QuickRow::DeviceSettings.position(&a.hidden_rows()) {
            a.apply(Action::GbaDown(Btn::Down));
            a.apply(Action::GbaUp(Btn::Down));
        }
        a.apply(Action::GbaDown(Btn::A));
        a.apply(Action::GbaUp(Btn::A));
        a.apply(Action::GbaDown(Btn::Down));
        a.apply(Action::GbaUp(Btn::Down));
        assert_eq!(a.quick_menu(), Some(slot_ui::QuickRow::DeviceShutdown));
        a.apply(Action::GbaDown(Btn::A));
        assert!(a.powering_off());
        assert!(!a.restarting() && !a.relaunching());
        a.poweroff();
        assert_eq!(calls.load(Ordering::Relaxed), 10);
        if fail {
            assert!(!a.powering_off());
            assert_eq!(a.quick_menu(), Some(slot_ui::QuickRow::DeviceShutdown));
            assert_eq!(a.toast(), Some(slot_ui::Toast::PoweroffFailed));
            a.apply(Action::GbaDown(Btn::A));
            a.poweroff();
            assert_eq!(calls.load(Ordering::Relaxed), 20);
        } else {
            assert!(!a.ready_to_power_off());
            a.tick_ms(a.now() + 60_000);
            a.apply(Action::GbaDown(Btn::B));
            a.apply(Action::GbaDown(Btn::A));
            assert!(a.powering_off());
            assert_eq!(a.quick_menu(), Some(slot_ui::QuickRow::DeviceShutdown));
            assert!(!a.ready_to_power_off());
            a.poweroff();
            a.apply(Action::Restart);
            assert_eq!(calls.load(Ordering::Relaxed), 10);
            assert!(!a.restarting() && !a.relaunching());
        }
    }
}

#[test]
fn stock_shutdown_flushes_the_playing_game_before_requesting_init() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_stock_power_owned();
    a.set_power(slot_power::Power::new(
        Box::new(Reboot {
            inner: slot_power::SimPlatform::at(d.path().into()),
            calls: Default::default(),
            fail: true,
            resume_required: true,
        }),
        Duration::from_secs(180),
    ));
    a.poweroff();
    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_some()
    );
    assert_eq!(a.toast(), Some(slot_ui::Toast::PoweroffFailed));
}

struct Reboot {
    inner: slot_power::SimPlatform,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    fail: bool,
    resume_required: bool,
}

impl slot_power::Platform for Reboot {
    fn request_poweroff(&mut self) -> Result<(), String> {
        if self.resume_required {
            assert!(
                StateRing::new(self.root(), Platform::Gba, Core::Mgba, "Emerald")
                    .read_resume()
                    .unwrap()
                    .is_some(),
                "save must be readable before init is called"
            );
        }
        self.calls.fetch_add(10, Ordering::Relaxed);
        if self.fail {
            Err("mock poweroff refusal".into())
        } else {
            Ok(())
        }
    }

    fn request_restart(&mut self) -> Result<(), String> {
        if self.resume_required {
            assert!(
                StateRing::new(self.root(), Platform::Gba, Core::Mgba, "Emerald")
                    .read_resume()
                    .unwrap()
                    .is_some(),
                "save must be readable before init is called"
            );
        }
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.fail {
            Err("mock init refusal".into())
        } else {
            Ok(())
        }
    }
    fn root(&self) -> &std::path::Path {
        self.inner.root()
    }
    fn now(&self) -> i64 {
        self.inner.now()
    }
    fn set_backlight(&mut self, _: u8) {}
    fn set_led(&mut self, _: slot_power::LedState) {}
    fn set_rumble(&mut self, _: u16) {}
    fn set_clock(&mut self, _: i64) {}
    fn battery(&self) -> Option<slot_power::Battery> {
        None
    }
    fn charge(&self) -> slot_power::Charge {
        slot_power::Charge::Unknown
    }
    fn restart(&mut self) -> ! {
        panic!("legacy restart must not run")
    }
    fn poweroff(&mut self) -> ! {
        panic!("poweroff must not run")
    }
}

#[test]
fn lid_close_flushes_resume_before_dozing() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::LidClose);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    assert!(
        r.read_resume().unwrap().is_some(),
        "state must be durable before doze"
    );
    assert!(matches!(a.phase(), Phase::Doze { .. }));
    assert!(
        r.list().unwrap().is_empty(),
        "lid close must not create a polaroid"
    );
}

#[test]
fn lid_open_returns_to_the_game_without_a_button() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::LidClose);
    a.apply(Action::LidOpen);
    assert!(matches!(a.phase(), Phase::Playing { .. }));
}

#[test]
fn doze_timeout_powers_off_with_the_cart_still_seated() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::LidClose);
    a.on_doze_timeout();
    assert_eq!(
        read_slot_state(d.path()).cart,
        Some("Emerald".into()),
        "power off is not an eject"
    );
}

#[test]
fn lid_close_on_the_shelf_wakes_back_to_the_shelf() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.set_snapshot(StubSnapshot::boxed());
    a.apply(Action::LidClose);
    assert!(matches!(a.phase(), Phase::Doze { cart: None }));
    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_none()
    );
    a.apply(Action::LidOpen);
    assert!(matches!(a.phase(), Phase::Shelf));
}

#[test]
fn lid_close_over_the_switcher_wakes_into_the_game() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::SaveState);
    a.apply(Action::Polaroids);
    a.apply(Action::LidClose);
    a.apply(Action::LidOpen);
    assert!(matches!(a.phase(), Phase::Playing { .. }));
    assert_eq!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .list()
            .unwrap()
            .len(),
        1,
        "only the deliberate save belongs in the ring"
    );
}

#[test]
fn a_doze_that_outlasts_the_timeout_powers_off_by_itself() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_power(panel(d.path(), Duration::from_secs(2)).0);
    a.apply(Action::LidClose);
    for _ in 0..100 {
        a.update(1.0 / 60.0);
    }
    assert!(!a.powering_off(), "1.6 s is short of the 2 s timeout");
    for _ in 0..40 {
        a.update(1.0 / 60.0);
    }
    assert!(a.powering_off());
}

#[test]
fn a_stray_doze_timeout_does_not_power_off_a_running_game() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.on_doze_timeout();
    assert!(!a.powering_off());
    assert!(matches!(a.phase(), Phase::Playing { .. }));
}

#[test]
fn the_backlight_follows_brightness_from_boot() {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_slot_state(
        d.path(),
        &SlotState {
            brightness: 3,
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut a = boot(d.path());
    let (power, step) = panel(d.path(), Duration::from_secs(60));
    a.set_power(power);
    assert_eq!(step.load(Ordering::Relaxed), 4);
    a.apply(Action::BrightnessUp);
    assert_eq!(step.load(Ordering::Relaxed), 5);
    for _ in 0..12 {
        a.apply(Action::BrightnessDown);
    }
    assert_eq!(a.brightness(), 0);
    assert_eq!(
        step.load(Ordering::Relaxed),
        1,
        "the lowest brightness turned the panel off"
    );
}

#[test]
fn a_power_off_draws_a_shutdown_screen_over_everything() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::PowerHold);

    let mut out = Vec::new();
    a.draw(&mut out);

    match out.first() {
        Some(Draw::Rect { w, h, colour, .. }) => {
            assert_eq!(*colour, [0.0, 0.0, 0.0, 1.0], "the shutdown is black");
            assert!(*w > 0.0 && *h > 0.0, "and covers the panel");
        }
        other => panic!("the shutdown drew {other:?} rather than a panel of black"),
    }
    assert_eq!(
        out.len(),
        1,
        "nothing of the previous phase survives the shutdown screen"
    );
}

#[test]
fn a_hold_flushes_and_powers_off() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::PowerHold);
    assert!(a.powering_off() && !a.restarting());
    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_some(),
        "the resume was not on the card before the shutdown began"
    );
}

#[test]
fn the_shutdown_screen_is_up_before_the_machine_may_stop() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::PowerHold);

    assert!(a.powering_off(), "the hold decides immediately");
    assert!(
        !a.ready_to_power_off(),
        "but the binary may not act until the screen has been presented"
    );

    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(
        matches!(out.first(), Some(Draw::Rect { colour, .. }) if *colour == [0.0, 0.0, 0.0, 1.0]),
        "and the screen is what the loop is drawing in the meantime"
    );

    a.tick_ms(600_000);
    assert!(a.ready_to_power_off(), "then it may stop");
}

#[test]
fn a_dozing_device_powers_off_by_itself_rather_than_waiting_for_the_lid() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_power(panel(d.path(), Duration::from_secs(2)).0);
    a.apply(Action::LidClose);
    for _ in 0..180 {
        a.update(1.0 / 60.0);
    }
    assert!(
        a.powering_off(),
        "three seconds is past the two second timeout"
    );
    assert!(
        a.ready_to_power_off(),
        "the shutdown screen has had its 250 ms and the machine is still not allowed to stop"
    );
}

#[test]
fn a_committed_shutdown_holds_the_core_still() {
    let d = tmp_root_with_real_carts(&["Advance Wars", "Emerald"]);
    let (mut s, _motor) = session_with_platform(d.path());
    let mut now = 0;
    play(&mut s, &mut now);

    hold_power(&mut s, &mut now);
    assert!(s.app().powering_off(), "the hold did not power off");
    s.update(DT);
    await_paused(&mut s);
}

#[test]
fn power_pressed_while_dozing_lights_the_panel_before_the_shutdown_screen() {
    let d = tmp_root_with_carts(&["Emerald"]);
    clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    let (power, backlight) = panel(d.path(), Duration::from_secs(180));
    s.app_mut().set_power(power);
    let lit = backlight.load(Ordering::Relaxed);
    assert!(
        lit > 0,
        "the panel never came on, so going dark proves nothing"
    );

    let mut now = 0;
    event(&mut s, RawEvent::Down(Btn::Power), &mut now);
    event(&mut s, RawEvent::Up(Btn::Power), &mut now);
    assert!(
        matches!(s.app().phase(), Phase::Doze { .. }),
        "the device never dozed"
    );
    assert_eq!(
        backlight.load(Ordering::Relaxed),
        0,
        "the doze left the panel lit"
    );

    event(&mut s, RawEvent::Down(Btn::Power), &mut now);
    assert_eq!(
        backlight.load(Ordering::Relaxed),
        lit,
        "the panel is still dark under the thumb trying to wake it"
    );
    assert!(
        !matches!(s.app().phase(), Phase::Doze { .. }),
        "the panel came on over a device still dozing behind it"
    );

    let pressed = now;
    while now < pressed + POWER_HOLD_MS + FRAME_MS {
        step(&mut s, &mut now);
    }
    assert!(s.app().powering_off(), "the hold did not power off");
    assert_eq!(
        backlight.load(Ordering::Relaxed),
        lit,
        "the shutdown screen is up on a panel nobody can see"
    );
}

#[test]
fn the_press_that_woke_the_panel_does_not_doze_again_when_it_is_let_go() {
    let d = tmp_root_with_carts(&["Emerald"]);
    clocked(d.path());
    let mut s = Session::boot(d.path().to_path_buf());
    let (power, backlight) = panel(d.path(), Duration::from_secs(180));
    s.app_mut().set_power(power);
    let lit = backlight.load(Ordering::Relaxed);

    let mut now = 0;
    event(&mut s, RawEvent::Down(Btn::Power), &mut now);
    event(&mut s, RawEvent::Up(Btn::Power), &mut now);
    assert!(matches!(s.app().phase(), Phase::Doze { .. }));

    event(&mut s, RawEvent::Down(Btn::Power), &mut now);
    event(&mut s, RawEvent::Up(Btn::Power), &mut now);
    assert!(
        !matches!(s.app().phase(), Phase::Doze { .. }),
        "the tap that woke the device put it straight back to sleep"
    );
    assert_eq!(backlight.load(Ordering::Relaxed), lit);

    event(&mut s, RawEvent::Down(Btn::Power), &mut now);
    event(&mut s, RawEvent::Up(Btn::Power), &mut now);
    assert!(
        matches!(s.app().phase(), Phase::Doze { .. }),
        "POWER stopped being able to put the device out"
    );
    assert_eq!(backlight.load(Ordering::Relaxed), 0);
}

fn await_paused(s: &mut Session) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while s.observed_speed() != Some(Speed::Paused) {
        assert!(
            Instant::now() < deadline,
            "the core was still running behind the shutdown"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn hold_power(s: &mut Session, now: &mut Millis) {
    let pressed = *now;
    event(s, RawEvent::Down(Btn::Power), now);
    while *now < pressed + POWER_HOLD_MS + FRAME_MS {
        step(s, now);
    }
}

fn step(s: &mut Session, now: &mut Millis) {
    *now += FRAME_MS;
    s.feed([], *now);
    s.update(DT);
}

fn event(s: &mut Session, ev: RawEvent, now: &mut Millis) {
    *now += FRAME_MS;
    s.feed([ev], *now);
    s.update(DT);
}

fn play(s: &mut Session, now: &mut Millis) {
    event(s, RawEvent::Down(Btn::A), now);
    event(s, RawEvent::Up(Btn::A), now);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline, "the cart never seated");
        step(s, now);
        std::thread::sleep(Duration::from_millis(1));
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while s.observed_speed() != Some(Speed::Normal) {
        assert!(Instant::now() < deadline, "the core never started");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[derive(Clone, Default)]
struct RadioLog(std::sync::Arc<std::sync::Mutex<Vec<RadioJob>>>);

impl RadioLog {
    fn jobs(&self) -> Vec<RadioJob> {
        self.0.lock().expect("radio log").clone()
    }
}

impl RadioJobs for RadioLog {
    fn ask(&mut self, job: RadioJob) {
        self.0.lock().expect("radio log").push(job);
    }

    fn warmed(&self) -> bool {
        false
    }
}

#[test]
fn a_shut_lid_cools_the_radio() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let log = RadioLog::default();
    a.set_radio_jobs(Box::new(log.clone()));
    a.apply(Action::LidClose);
    assert!(
        log.jobs().contains(&RadioJob::Cool),
        "the lid closed with the radio left loaded behind it"
    );
}

#[test]
fn a_shut_lid_over_a_session_takes_its_network_down_too() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let log = RadioLog::default();
    a.set_radio_jobs(Box::new(log.clone()));
    a.begin_link(0);
    a.apply(Action::LidClose);
    let jobs = log.jobs();
    assert_eq!(
        jobs.first(),
        Some(&RadioJob::Down),
        "the session's own network must come down before the driver does: {jobs:?}"
    );
    assert!(jobs.contains(&RadioJob::Cool));
}

#[test]
fn a_shut_lid_powers_off_after_the_minutes_set_in_the_menu() {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            lid_off_min: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    let mut a = boot(d.path());
    a.set_power(panel(d.path(), Duration::from_secs(300)).0);
    a.apply(Action::LidClose);
    for _ in 0..59 * 60 {
        a.update(1.0 / 60.0);
    }
    assert!(!a.powering_off(), "59 s is short of the minute");
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert!(a.powering_off());
}
