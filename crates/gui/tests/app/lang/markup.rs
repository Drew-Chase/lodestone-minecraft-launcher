use super::*;

/// The `(text, primary)` runs of `raw`, as the renderer sees them.
fn runs(raw: &str) -> Vec<(&str, bool)> {
    segments(raw).into_iter().map(|segment| (segment.text, segment.primary)).collect()
}

/// The text of `raw` with its markup resolved.
fn plain(raw: &str) -> String {
    segments(raw).iter().map(|segment| segment.text).collect()
}

#[test]
fn text_without_markup_is_one_base_run() {
    assert_eq!(runs("Lodestone"), vec![("Lodestone", false)]);
}

#[test]
fn p_turns_primary_ink_on_and_c_clears_it() {
    assert_eq!(runs("Welcome back, %Pcrafter%C"), vec![("Welcome back, ", false), ("crafter", true)]);
    assert_eq!(runs("%Pa%Cb%Pc"), vec![("a", true), ("b", false), ("c", true)]);
}

#[test]
fn an_empty_run_is_dropped() {
    assert!(runs("%P%C").is_empty());
    assert_eq!(runs("%Pcrafter"), vec![("crafter", true)]);
}

#[test]
fn a_double_percent_is_one_literal_percent() {
    assert_eq!(plain("100%% done"), "100% done");
    assert_eq!(plain("%%P is literal"), "%P is literal");
}

#[test]
fn other_percent_signs_are_kept_as_written() {
    assert_eq!(runs("50% off"), vec![("50% off", false)]);
    assert_eq!(runs("%X and a trailing %"), vec![("%X and a trailing %", false)]);
}
