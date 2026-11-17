use super::*;

#[test]
fn nothing_is_reported_until_an_event_is_seen() {
    assert_eq!(Burst::default().deadline(), None);
}

#[test]
fn a_burst_is_reported_once_the_folder_has_been_quiet() {
    let start = Instant::now();
    let mut burst = Burst::default();
    burst.push(start);
    assert_eq!(burst.deadline(), Some(start + QUIET));
    burst.push(start + Duration::from_millis(100));
    assert_eq!(
        burst.deadline(),
        Some(start + Duration::from_millis(100) + QUIET),
        "a new event pushes the report back"
    );
}

#[test]
fn a_folder_that_never_goes_quiet_is_still_reported() {
    let start = Instant::now();
    let mut burst = Burst::default();
    burst.push(start);
    burst.push(start + LONGEST);
    assert_eq!(burst.deadline(), Some(start + LONGEST), "the report waits no longer than the cap");
}
