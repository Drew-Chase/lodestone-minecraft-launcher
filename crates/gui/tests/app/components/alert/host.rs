use super::*;
use tokio::sync::oneshot::error::TryRecvError;

#[test]
fn an_answer_reaches_the_waiting_receiver() {
    let mut alerts = Alerts::default();
    let mut answer = alerts.push(Alert::danger("Delete it?"));
    assert_eq!(answer.try_recv(), Err(TryRecvError::Empty));
    alerts.respond(true);
    assert_eq!(answer.try_recv(), Ok(true));
    assert!(!alerts.is_open());
}

#[test]
fn update_forwards_the_response() {
    let mut alerts = Alerts::default();
    let mut answer = alerts.push(Alert::warn("Continue?"));
    alerts.update(Message::Respond(false));
    assert_eq!(answer.try_recv(), Ok(false));
}

#[test]
fn alerts_queue_in_order() {
    let mut alerts = Alerts::default();
    let mut first = alerts.push(Alert::info("first"));
    let mut second = alerts.push(Alert::error("second"));
    assert_eq!(alerts.current().map(Alert::message), Some("first"));
    assert!(alerts.view().is_some());

    alerts.respond(false);
    assert_eq!(first.try_recv(), Ok(false));
    assert_eq!(second.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(alerts.current().map(Alert::message), Some("second"));

    alerts.respond(true);
    assert_eq!(second.try_recv(), Ok(true));
    assert!(alerts.view().is_none());
}

#[test]
fn responding_with_nothing_open_is_ignored() {
    let mut alerts = Alerts::default();
    alerts.respond(true);
    assert!(!alerts.is_open());
}

#[test]
fn a_dropped_alert_closes_its_receiver() {
    let mut alerts = Alerts::default();
    let mut answer = alerts.push(Alert::danger("Delete it?"));
    drop(alerts);
    // `show` maps this error to `false`.
    assert_eq!(answer.try_recv(), Err(TryRecvError::Closed));
}

#[test]
fn an_abandoned_receiver_does_not_block_the_queue() {
    let mut alerts = Alerts::default();
    drop(alerts.push(Alert::info("nobody waits")));
    let mut next = alerts.push(Alert::info("next"));
    alerts.respond(true);
    assert_eq!(alerts.current().map(Alert::message), Some("next"));
    alerts.respond(true);
    assert_eq!(next.try_recv(), Ok(true));
}
