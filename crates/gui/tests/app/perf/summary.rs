use super::*;

#[test]
fn figures_turn_counts_into_rates_per_second() {
    let mut window = Window::new();
    for _ in 0..60 {
        window.record_timing("frame.draw", 2_000_000);
    }
    window.record_count("iced.svg", 73_032);
    window.record_count("iced.svg", 73_032);

    let figures = Figures::new(&window, 2.0, 3, 0);
    assert_eq!(figures.frames_per_s, 30.0);
    assert_eq!(figures.svg_per_s, 1.0);
    assert_eq!(figures.svg_pixels_per_s, 73_032.0);
    assert_eq!(figures.dropped_lines, 3);
}

#[test]
fn the_report_lists_the_slowest_scopes_first_and_stops_at_the_top() {
    let mut window = Window::new();
    window.record_timing("view.library", 10);
    window.record_timing("io.scan", 500);
    window.record_timing("frame.draw", 300);
    window.record_timing("art.build", 40);
    window.record_timing("update.Library.Scrolled", 200);
    window.record_timing("view.instance", 5);

    let figures = Figures::new(&window, 1.0, 0, 0);
    let names: Vec<&str> = figures.slowest.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["io.scan", "frame.draw", "update.Library.Scrolled", "art.build", "view.library"]);
}

#[test]
fn the_headline_carries_the_frame_and_view_figures() {
    let mut window = Window::new();
    window.record_timing("frame.draw", 4_000_000);
    window.record_timing("view.root", 1_000_000);

    let figures = Figures::new(&window, 1.0, 0, 0);
    let headline = &lines(&figures)[0];
    assert!(headline.starts_with("perf 1.0s  frames 1/s  draw 4.00 ms (max 4.00)"), "{headline}");
    assert!(headline.contains("view 1.00 ms (max 1.00)"), "{headline}");
}
