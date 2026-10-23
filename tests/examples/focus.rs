use super::*;

mod support;

#[test]
fn pause_resume_and_finish_use_monotonic_elapsed_time() {
    let start = Instant::now();
    let mut session = Session::default();
    session.start(start);
    session.pause(start + Duration::from_secs(10));
    assert_eq!(session.elapsed, Duration::from_secs(10));
    assert_eq!(
        session.elapsed_at(start + Duration::from_secs(100)),
        Duration::from_secs(10)
    );
    session.start(start + Duration::from_secs(100));
    assert_eq!(
        session.elapsed_at(start + Duration::from_secs(105)),
        Duration::from_secs(15)
    );
    session.finish(Duration::from_secs(1500));
    assert!(session.finished);
    assert!(session.started.is_none());
    session.start(start + Duration::from_secs(200));
    assert!(session.started.is_none());
}

#[test]
fn timer_actions_start_pause_and_reset_the_same_session() {
    let mut runtime = support::mount(app);
    support::click(&mut runtime, "Start");
    assert!(support::text(runtime.retained().unwrap().root().unwrap())
        .iter()
        .any(|text| text == "Focus in progress"));
    support::click(&mut runtime, "Pause");
    assert!(support::text(runtime.retained().unwrap().root().unwrap())
        .iter()
        .any(|text| text == "Paused"));
    support::click(&mut runtime, "Reset");
    assert!(support::text(runtime.retained().unwrap().root().unwrap())
        .iter()
        .any(|text| text == "Ready when you are"));
}
