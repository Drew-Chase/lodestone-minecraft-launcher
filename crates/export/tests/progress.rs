use super::*;

#[test]
fn advance_reports_each_finished_file_and_its_bytes() {
    let cancel = AtomicBool::new(false);
    let mut seen = Vec::new();
    let mut report = |progress: Progress| seen.push(progress);
    let mut control = Control::new(&cancel, 2, 9, &mut report);
    control.advance(4).expect("advances");
    control.advance(5).expect("advances");
    assert_eq!(seen.len(), 2);
    assert_eq!(
        seen[0],
        Progress {
            files: 1,
            total_files: 2,
            bytes: 4,
            total_bytes: 9
        }
    );
    assert_eq!(
        seen[1],
        Progress {
            files: 2,
            total_files: 2,
            bytes: 9,
            total_bytes: 9
        }
    );
}

#[test]
fn a_cancelled_write_fails_at_the_next_file() {
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, 2, 9, &mut report);
    control.advance(4).expect("advances");
    cancel.store(true, Ordering::SeqCst);
    assert!(matches!(control.check(), Err(Error::Cancelled)));
    assert!(matches!(control.advance(5), Err(Error::Cancelled)));
}
