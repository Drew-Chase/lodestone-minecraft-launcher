use super::*;

#[test]
fn a_tick_records_the_bytes_received_since_the_last_one() {
    let mut meter = Meter::default();
    meter.tick(1_000);
    meter.tick(1_500);
    assert_eq!(meter.samples.iter().copied().collect::<Vec<_>>(), vec![1_000, 500]);
}

#[test]
fn the_average_is_the_mean_of_the_samples() {
    let mut meter = Meter::default();
    meter.tick(0);
    meter.tick(100);
    meter.tick(400);
    assert!((meter.average() - 400.0 / 3.0).abs() < 1e-9);
}

#[test]
fn an_empty_meter_averages_nothing() {
    assert_eq!(Meter::default().average(), 0.0);
    assert!(Meter::default().minutes().is_empty());
}

#[test]
fn the_meter_keeps_an_hour_of_samples() {
    let mut meter = Meter::default();
    for second in 0..(HOUR as u64 + 10) {
        meter.tick(second);
    }
    assert_eq!(meter.samples.len(), HOUR);
}

#[test]
fn the_chart_has_one_point_per_minute_and_the_last_is_partial() {
    let mut meter = Meter::default();
    for second in 0..130u64 {
        meter.tick(second * 10);
    }
    assert_eq!(meter.minutes().len(), 3);
}
