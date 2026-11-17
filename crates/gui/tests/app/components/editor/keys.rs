use super::*;

/// A context with the caret where `|` stands in `text`, in `language`, indenting by `unit`.
fn at<'a>(text: &'a str, line: &'a mut String, language: Language, unit: &'a str) -> Context<'a> {
    let column = text.find('|').expect("the caret is marked with |");
    *line = text.replacen('|', "", 1);
    Context {
        line: line.as_str(),
        column,
        selection: false,
        language,
        unit,
    }
}

/// The plan for `typed` with the caret at `|` in `text`, in a JSON file indented by two spaces.
fn json(typed: Typed, text: &str) -> Option<Vec<Step>> {
    let mut line = String::new();
    plan(typed, &at(text, &mut line, Language::Json, "  "))
}

fn text(text: &str) -> Step {
    Step::Text(text.to_string())
}

#[test]
fn an_opening_bracket_brings_its_closer_and_leaves_the_caret_between_them() {
    assert_eq!(json(Typed::Char('{'), "|"), Some(vec![text("{}"), Step::Left]));
    assert_eq!(json(Typed::Char('['), "\"a\": |"), Some(vec![text("[]"), Step::Left]));
    assert_eq!(
        json(Typed::Char('('), "call|)"),
        Some(vec![text("()"), Step::Left]),
        "before a closer is fine"
    );
}

#[test]
fn an_opener_in_front_of_a_word_is_typed_alone() {
    assert_eq!(json(Typed::Char('('), "|word"), None);
    assert_eq!(json(Typed::Char('"'), "|word"), None);
}

#[test]
fn a_quote_pairs_except_as_an_apostrophe_or_after_a_backslash() {
    assert_eq!(json(Typed::Char('"'), "\"a\": |"), Some(vec![text("\"\""), Step::Left]));
    assert_eq!(json(Typed::Char('\''), "don|"), None, "an apostrophe");
    assert_eq!(json(Typed::Char('"'), "\"say \\|"), None, "an escaped quote");
    assert_eq!(
        json(Typed::Char('"'), "\"\"|"),
        None,
        "a third quote after an empty string is typed alone"
    );
}

#[test]
fn typing_the_closer_that_is_already_there_steps_over_it() {
    assert_eq!(json(Typed::Char(')'), "call(|)"), Some(vec![Step::Right]));
    assert_eq!(json(Typed::Char('"'), "\"text|\""), Some(vec![Step::Right]));
    assert_eq!(json(Typed::Char(')'), "call(|"), None, "nothing to step over");
    assert_eq!(json(Typed::Char('a'), "|a"), None, "only closers are stepped over");
}

#[test]
fn an_opener_typed_over_a_selection_wraps_it() {
    let mut line = String::new();
    let mut context = at("|word", &mut line, Language::Json, "  ");
    context.selection = true;
    assert_eq!(plan(Typed::Char('"'), &context), Some(vec![Step::Wrap { open: '"', close: '"' }]));
    assert_eq!(plan(Typed::Char('['), &context), Some(vec![Step::Wrap { open: '[', close: ']' }]));
    assert_eq!(plan(Typed::Char('x'), &context), None, "any other key replaces the selection as usual");
}

#[test]
fn a_log_or_a_crash_report_gets_no_pairs() {
    let mut line = String::new();
    assert_eq!(plan(Typed::Char('('), &at("|", &mut line, Language::Log, "    ")), None);
    assert_eq!(plan(Typed::Char('"'), &at("|", &mut line, Language::Crash, "    ")), None);
}

#[test]
fn backspace_between_an_empty_pair_removes_both_marks() {
    assert_eq!(json(Typed::Backspace, "{|}"), Some(vec![Step::Delete, Step::Backspace]));
    assert_eq!(json(Typed::Backspace, "\"|\""), Some(vec![Step::Delete, Step::Backspace]));
    assert_eq!(json(Typed::Backspace, "{a|}"), None);
    assert_eq!(json(Typed::Backspace, "(|]"), None, "a mismatched pair is not a pair");
}

#[test]
fn backspace_in_the_indent_goes_back_one_level() {
    assert_eq!(json(Typed::Backspace, "    |\"a\": 1"), Some(vec![Step::Backspace; 2]));
    assert_eq!(
        json(Typed::Backspace, "   |\"a\": 1"),
        Some(vec![Step::Backspace; 1]),
        "an odd space goes back to the level below"
    );
    assert_eq!(json(Typed::Backspace, "  \"a\"|"), None, "past the indent it is an ordinary backspace");
    let mut line = String::new();
    assert_eq!(
        plan(Typed::Backspace, &at("\t\t|x", &mut line, Language::Toml, "\t")),
        None,
        "a tab is one character already"
    );
}

#[test]
fn enter_keeps_the_indent_of_the_line() {
    assert_eq!(json(Typed::Enter, "    \"a\": 1,|"), Some(vec![text("\n    ")]));
    assert_eq!(json(Typed::Enter, "no indent|"), None, "with no indent Enter is ordinary");
    assert_eq!(
        json(Typed::Enter, "  |    deep"),
        Some(vec![text("\n  ")]),
        "inside the indent, only what is before the caret"
    );
}

#[test]
fn enter_after_an_opener_indents_one_level_more() {
    assert_eq!(json(Typed::Enter, "  \"a\": {|"), Some(vec![text("\n    ")]));
    assert_eq!(json(Typed::Enter, "[|"), Some(vec![text("\n  ")]));
    assert_eq!(
        json(Typed::Enter, "  \"a\": { |"),
        Some(vec![text("\n    ")]),
        "a space after the opener does not matter"
    );
}

#[test]
fn enter_between_a_pair_puts_the_closer_on_its_own_line() {
    assert_eq!(
        json(Typed::Enter, "  \"a\": {|}"),
        Some(vec![text("\n    \n  "), Step::Left, Step::Left, Step::Left]),
        "the caret goes back over the closer's indent and its line break"
    );
    assert_eq!(json(Typed::Enter, "[|]"), Some(vec![text("\n  \n"), Step::Left]));
    assert_eq!(json(Typed::Enter, "\"|\""), None, "an empty string is not a block");
}

#[test]
fn a_colon_opens_a_block_only_in_yaml() {
    let mut line = String::new();
    assert_eq!(
        plan(Typed::Enter, &at("settings:|", &mut line, Language::Yaml, "  ")),
        Some(vec![text("\n  ")])
    );
    assert_eq!(json(Typed::Enter, "\"a\":|"), None);
}

#[test]
fn tab_fills_to_the_next_level_or_inserts_a_tab() {
    assert_eq!(json(Typed::Tab, "|"), Some(vec![text("  ")]));
    assert_eq!(
        json(Typed::Tab, "abc|"),
        Some(vec![text(" ")]),
        "three characters in, one space reaches the next level"
    );
    let mut line = String::new();
    assert_eq!(plan(Typed::Tab, &at("abc|", &mut line, Language::Toml, "\t")), Some(vec![text("\t")]));
    let mut context = at("|abc", &mut line, Language::Toml, "    ");
    context.selection = true;
    assert_eq!(plan(Typed::Tab, &context), Some(vec![Step::IndentLines]));
}

#[test]
fn the_indent_unit_is_read_from_the_file() {
    assert_eq!(indent_unit(["{", "    \"a\": 1", "        \"b\": 2"].into_iter(), Language::Json), "    ");
    assert_eq!(indent_unit(["a:", "  b: 1"].into_iter(), Language::Toml), "  ");
    assert_eq!(indent_unit(["general {", "\tB:x=true"].into_iter(), Language::Ini), "\t");
    assert_eq!(
        indent_unit(["no indent", "here"].into_iter(), Language::Json),
        "  ",
        "the language's own width"
    );
    assert_eq!(indent_unit(["no indent"].into_iter(), Language::Toml), "    ");
    assert_eq!(
        indent_unit(["x", "   ", " y"].into_iter(), Language::Yaml),
        "  ",
        "blank lines and a single space say nothing"
    );
}

#[test]
fn unindenting_takes_one_level_off_a_line() {
    assert_eq!(unindent_width("\t\tx", "\t"), 1);
    assert_eq!(unindent_width("      x", "    "), 4);
    assert_eq!(unindent_width("  x", "    "), 2, "a line indented less loses what it has");
    assert_eq!(unindent_width("x", "  "), 0);
    assert_eq!(unindent_width("\tx", "  "), 1, "a tab in a file of spaces is still one level");
}
