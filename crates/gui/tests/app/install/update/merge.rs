use super::*;

#[test]
fn identical_files_merge_to_themselves_without_a_conflict() {
    let text = "a\nb\nc\n";
    let pieces = merge(text, text, text);
    assert_eq!(conflicts(&pieces), 0);
    assert_eq!(render(&pieces, |_| Side::Yours), text);
}

#[test]
fn a_change_of_the_user_alone_is_kept() {
    let base = "a\nb\nc\n";
    let yours = "a\nB\nc\n";
    let pieces = merge(base, yours, base);
    assert_eq!(conflicts(&pieces), 0);
    assert_eq!(render(&pieces, |_| Side::Theirs), yours);
}

#[test]
fn a_change_of_the_release_alone_is_taken() {
    let base = "a\nb\nc\n";
    let theirs = "a\nb\nC\n";
    let pieces = merge(base, base, theirs);
    assert_eq!(conflicts(&pieces), 0);
    assert_eq!(render(&pieces, |_| Side::Yours), theirs);
}

#[test]
fn changes_to_different_lines_both_survive() {
    let base = "one\ntwo\nthree\nfour\nfive\n";
    let yours = "ONE\ntwo\nthree\nfour\nfive\n";
    let theirs = "one\ntwo\nthree\nfour\nFIVE\n";
    let pieces = merge(base, yours, theirs);
    assert_eq!(conflicts(&pieces), 0);
    assert_eq!(render(&pieces, |_| Side::Yours), "ONE\ntwo\nthree\nfour\nFIVE\n");
}

#[test]
fn the_same_line_changed_differently_is_a_conflict_the_user_picks() {
    let base = "key = 1\n";
    let yours = "key = 2\n";
    let theirs = "key = 3\n";
    let pieces = merge(base, yours, theirs);
    assert_eq!(conflicts(&pieces), 1);
    assert_eq!(render(&pieces, |_| Side::Yours), "key = 2\n");
    assert_eq!(render(&pieces, |_| Side::Theirs), "key = 3\n");
}

#[test]
fn a_change_made_the_same_way_on_both_sides_is_not_a_conflict() {
    let base = "key = 1\n";
    let changed = "key = 2\n";
    let pieces = merge(base, changed, changed);
    assert_eq!(conflicts(&pieces), 0);
    assert_eq!(render(&pieces, |_| Side::Theirs), changed);
}

#[test]
fn each_conflict_is_resolved_on_its_own() {
    let base = "a\nb\nc\nd\ne\n";
    let yours = "A\nb\nc\nD\ne\n";
    let theirs = "x\nb\nc\ny\ne\n";
    let pieces = merge(base, yours, theirs);
    assert_eq!(conflicts(&pieces), 2);
    let text = render(&pieces, |index| if index == 0 { Side::Yours } else { Side::Theirs });
    assert_eq!(text, "A\nb\nc\ny\ne\n");
}

#[test]
fn adjacent_changes_from_both_sides_are_one_conflict() {
    // With no unchanged line between them, the two changes touch, and git reports that as a conflict too.
    let base = "a\nb\nc\n";
    let yours = "A\nb\nc\n";
    let theirs = "a\nB\nc\n";
    let pieces = merge(base, yours, theirs);
    assert_eq!(conflicts(&pieces), 1);
}

#[test]
fn a_change_beside_an_appended_line_merges_when_an_unchanged_line_separates_them() {
    let base = "a\nb\nd\n";
    let yours = "a\nB\nd\n";
    let theirs = "a\nb\nd\nz\n";
    let pieces = merge(base, yours, theirs);
    assert_eq!(conflicts(&pieces), 0);
    assert_eq!(render(&pieces, |_| Side::Yours), "a\nB\nd\nz\n");
}

#[test]
fn two_way_every_difference_is_a_conflict() {
    let yours = "a\nB\nc\n";
    let theirs = "a\nb\nc\n";
    let pieces = compare(yours, theirs);
    assert_eq!(conflicts(&pieces), 1);
    assert_eq!(render(&pieces, |_| Side::Yours), yours);
    assert_eq!(render(&pieces, |_| Side::Theirs), theirs);
}

#[test]
fn two_way_identical_files_have_no_conflict() {
    let text = "a\nb\n";
    assert_eq!(conflicts(&compare(text, text)), 0);
}

#[test]
fn line_endings_are_kept_byte_for_byte() {
    let base = "a=1\r\nb=2\r\n";
    let yours = "a=1\r\nb=9\r\n";
    let pieces = merge(base, yours, base);
    assert_eq!(render(&pieces, |_| Side::Yours), yours);
}
