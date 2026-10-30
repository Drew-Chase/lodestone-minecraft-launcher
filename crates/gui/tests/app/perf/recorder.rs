use super::*;

#[test]
fn a_stat_tracks_count_sum_max_and_mean() {
    let mut window = Window::new();
    window.record_timing("frame.draw", 2_000);
    window.record_timing("frame.draw", 6_000);

    let draw = window.timing("frame.draw");
    assert_eq!(
        draw,
        Stat {
            count: 2,
            sum: 8_000,
            max: 6_000
        }
    );
    assert_eq!(draw.mean(), 4_000);
    assert_eq!(Stat::default().mean(), 0, "a scope that never ran has no mean");
}

#[test]
fn timings_under_a_prefix_fold_together() {
    let mut window = Window::new();
    window.record_timing("update.Navigate", 1_000);
    window.record_timing("update.Library.Scrolled", 5_000);
    window.record_timing("view.root", 9_000);

    assert_eq!(
        window.timings_under("update."),
        Stat {
            count: 2,
            sum: 6_000,
            max: 5_000
        }
    );
}

#[test]
fn counters_are_kept_apart_from_timings() {
    let mut window = Window::new();
    window.record_count("iced.svg", 73_032);

    assert_eq!(
        window.count("iced.svg"),
        Stat {
            count: 1,
            sum: 73_032,
            max: 73_032
        }
    );
    assert_eq!(window.timing("iced.svg"), Stat::default());
    assert!(!window.is_empty());
}

#[test]
fn take_hands_over_the_window_and_starts_a_fresh_one() {
    // A private window, so the reporter thread of another test cannot take these figures first.
    let window = Mutex::new(Window::new());
    lock(&window).record_count("test.take", 1);

    let taken = take_from(&window);
    assert_eq!(taken.count("test.take").count, 1);
    assert!(take_from(&window).is_empty());
}
