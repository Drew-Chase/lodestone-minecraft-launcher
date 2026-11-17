use super::*;

fn loaded(path: &str, text: &str) -> Loaded {
    Loaded {
        path: path.to_string(),
        text: text.to_string(),
        encoding: Encoding::Utf8,
        stamp: Stamp::default(),
    }
}

fn editor(path: &str, text: &str) -> Editor {
    Editor::new(loaded(path, text), false)
}

/// Puts the caret at `line` and `column`, with no selection. Moving the caret does not drop a selection by itself, so
/// one that is there is dropped first, as a press of an arrow key drops it.
fn caret(editor: &mut Editor, line: usize, column: usize) {
    if editor.content.cursor().selection.is_some() {
        editor.content.perform(Action::Move(Motion::Left));
    }
    editor.content.move_to(Cursor {
        position: Position { line, column },
        selection: None,
    });
}

/// Selects from the start of line `from` to `column` of line `to`.
fn select(editor: &mut Editor, from: usize, to: usize, column: usize) {
    editor.content.move_to(Cursor {
        position: Position { line: to, column },
        selection: Some(Position { line: from, column: 0 }),
    });
}

fn typed(editor: &mut Editor, text: &str) {
    for character in text.chars() {
        let _ = editor.update(Message::Action(Action::Edit(Edit::Insert(character))));
    }
}

#[test]
fn a_file_opens_clean_with_its_language_and_its_own_indent() {
    let editor = editor("config/sodium.json", "{\n    \"a\": 1\n}\n");
    assert!(!editor.is_dirty());
    assert_eq!(editor.language, Language::Json);
    assert_eq!(editor.unit, "    ", "the file indents by four, whatever JSON usually takes");
    assert_eq!(editor.longest, 10);
    assert_eq!(editor.path(), "config/sodium.json");
}

#[test]
fn an_unchanged_file_is_written_back_byte_for_byte() {
    for text in ["a = 1\nb = 2\n", "a = 1\nb = 2", "", "\n\n", "a\r\nb\r\n", "tab\there\n"] {
        let editor = editor("x.toml", text);
        assert_eq!(editor.bytes(), (text.as_bytes().to_vec(), Encoding::Utf8), "{text:?}");
    }
    let utf16 = Editor::new(
        Loaded {
            encoding: Encoding::Utf16Le,
            ..loaded("x.txt", "hi\r\n")
        },
        false,
    );
    assert_eq!(utf16.bytes(), (vec![0xFF, 0xFE, b'h', 0, b'i', 0, b'\r', 0, b'\n', 0], Encoding::Utf16Le));
}

#[test]
fn typing_makes_the_file_dirty_and_saving_makes_it_clean() {
    let mut editor = editor("x.toml", "a = 1");
    caret(&mut editor, 0, 5);
    typed(&mut editor, "0");
    assert!(editor.is_dirty());
    assert_eq!(editor.bytes().0, b"a = 10");
    editor.saved(Encoding::Utf8, Stamp::default());
    assert!(!editor.is_dirty());
}

#[test]
fn a_line_typed_into_a_windows_file_ends_the_windows_way() {
    let mut editor = editor("x.cfg", "a=1\r\nb=2\r\n");
    caret(&mut editor, 0, 3);
    let _ = editor.update(Message::Action(Action::Edit(Edit::Enter)));
    typed(&mut editor, "c=3");
    assert_eq!(editor.bytes().0, b"a=1\r\nc=3\r\nb=2\r\n");
}

#[test]
fn undo_takes_back_a_burst_of_typing_and_redo_brings_it_back() {
    let mut editor = editor("x.toml", "a = 1");
    caret(&mut editor, 0, 5);
    typed(&mut editor, "23");
    let _ = editor.update(Message::Command(Command::Undo));
    assert_eq!(editor.content.text(), "a = 1");
    assert!(!editor.is_dirty(), "the text is the saved text again");
    let _ = editor.update(Message::Command(Command::Redo));
    assert_eq!(editor.content.text(), "a = 123");
    assert!(editor.is_dirty());
    let _ = editor.update(Message::Command(Command::Redo));
    assert_eq!(editor.content.text(), "a = 123", "nothing more to redo");
}

#[test]
fn undo_after_a_save_makes_the_file_dirty_again() {
    let mut editor = editor("x.toml", "a");
    caret(&mut editor, 0, 1);
    typed(&mut editor, "b");
    editor.saved(Encoding::Utf8, Stamp::default());
    let _ = editor.update(Message::Command(Command::Undo));
    assert_eq!(editor.content.text(), "a");
    assert!(editor.is_dirty(), "the file holds `ab`, and the text is `a`");
}

#[test]
fn a_typing_helpers_edits_are_made_in_order() {
    let mut editor = editor("x.json", "");
    let _ = editor.update(Message::Steps(vec![Step::Text("{}".to_string()), Step::Left]));
    assert_eq!(editor.content.text(), "{}");
    assert_eq!(editor.content.cursor().position.column, 1, "the caret sits between the pair");
    let _ = editor.update(Message::Steps(vec![Step::Text("\n  \n".to_string()), Step::Left]));
    assert_eq!(editor.content.text(), "{\n  \n}");
    assert_eq!(editor.content.cursor().position, Position { line: 1, column: 2 });
    let _ = editor.update(Message::Command(Command::Undo));
    assert_eq!(editor.content.text(), "{}", "one helper is one step to undo");
}

#[test]
fn wrapping_puts_the_pair_around_the_selection() {
    let mut editor = editor("x.json", "name");
    select(&mut editor, 0, 0, 4);
    let _ = editor.update(Message::Steps(vec![Step::Wrap { open: '"', close: '"' }]));
    assert_eq!(editor.content.text(), "\"name\"");
}

#[test]
fn tab_over_a_selection_indents_its_lines_and_shift_tab_takes_the_indent_off() {
    let mut editor = editor("x.json", "{\n\"a\": 1,\n\n\"b\": 2\n}");
    select(&mut editor, 1, 3, 4);
    let _ = editor.update(Message::Steps(vec![Step::IndentLines]));
    assert_eq!(editor.content.text(), "{\n  \"a\": 1,\n\n  \"b\": 2\n}", "an empty line gets no indent");
    let cursor = editor.content.cursor();
    assert_eq!(cursor.position, Position { line: 3, column: 6 }, "the caret moved with its line");
    assert_eq!(cursor.selection, Some(Position { line: 1, column: 2 }));

    let _ = editor.update(Message::Command(Command::Unindent));
    assert_eq!(editor.content.text(), "{\n\"a\": 1,\n\n\"b\": 2\n}");
    let _ = editor.update(Message::Command(Command::Unindent));
    assert_eq!(editor.content.text(), "{\n\"a\": 1,\n\n\"b\": 2\n}", "nothing is left to take off");
}

#[test]
fn a_selection_that_ends_at_the_start_of_a_line_leaves_that_line_out() {
    let mut editor = editor("x.toml", "a\nb\nc");
    select(&mut editor, 0, 2, 0);
    assert_eq!(editor.selected_lines(), 0..=1);
    select(&mut editor, 0, 2, 1);
    assert_eq!(editor.selected_lines(), 0..=2);
    caret(&mut editor, 1, 0);
    assert_eq!(editor.selected_lines(), 1..=1);
}

#[test]
fn toggling_a_comment_marks_the_lines_at_their_shallowest_indent_and_takes_the_marks_off_again() {
    let mut editor = editor("x.toml", "[a]\n  x = 1\n    y = 2\n");
    select(&mut editor, 1, 2, 5);
    let _ = editor.update(Message::Command(Command::ToggleComment));
    assert_eq!(editor.content.text(), "[a]\n  # x = 1\n  #   y = 2\n");
    let _ = editor.update(Message::Command(Command::ToggleComment));
    assert_eq!(editor.content.text(), "[a]\n  x = 1\n    y = 2\n");
}

#[test]
fn a_language_with_no_line_comment_ignores_the_toggle() {
    let mut editor = editor("logs/latest.log", "[12:00:00] [main/INFO]: hi");
    let _ = editor.update(Message::Command(Command::ToggleComment));
    assert_eq!(editor.content.text(), "[12:00:00] [main/INFO]: hi");
    assert!(!editor.is_dirty());
}

#[test]
fn a_read_only_editor_takes_no_edit() {
    let mut editor = Editor::new(loaded("x.toml", "  a = 1"), true);
    caret(&mut editor, 0, 7);
    typed(&mut editor, "0");
    let _ = editor.update(Message::Steps(vec![Step::Text("x".to_string())]));
    let _ = editor.update(Message::Command(Command::Unindent));
    let _ = editor.update(Message::Command(Command::ToggleComment));
    assert_eq!(editor.content.text(), "  a = 1");
    assert!(!editor.is_dirty());
    assert!(editor.read_only());
}

#[test]
fn reloading_replaces_the_text_and_keeps_the_scroll() {
    let mut editor = editor("x.toml", "a = 1");
    let _ = editor.update(Message::Resized(Size::new(500.0, 300.0)));
    caret(&mut editor, 0, 5);
    typed(&mut editor, "0");
    // The reader scrolls away from the caret after typing.
    let _ = editor.update(Message::Scrolled { x: 0.0, y: 120.0 });
    editor.mark_changed_on_disk();
    editor.reload(loaded("x.toml", "a = 2\n"));
    assert_eq!(editor.content.text(), "a = 2\n");
    assert!(!editor.is_dirty() && !editor.changed_on_disk);
    assert_eq!(editor.scroll, (0.0, 120.0));
    assert_eq!(editor.viewport, Size::new(500.0, 300.0));
    let _ = editor.update(Message::Command(Command::Undo));
    assert_eq!(editor.content.text(), "a = 2\n", "the history of the old text is gone");
}

#[test]
fn text_the_files_encoding_cannot_hold_is_saved_as_utf8_and_the_editor_says_so() {
    let mut editor = Editor::new(
        Loaded {
            encoding: Encoding::Windows1252,
            ..loaded("x.txt", "caf\u{E9}")
        },
        false,
    );
    caret(&mut editor, 0, 5);
    typed(&mut editor, "\u{2603}");
    let (bytes, encoding) = editor.bytes();
    assert_eq!(encoding, Encoding::Utf8);
    assert_eq!(bytes, "caf\u{E9}\u{2603}".as_bytes());
    editor.saved(encoding, Stamp::default());
    assert!(editor.reencoded);
    assert_eq!(editor.encoding, Encoding::Utf8);
}

#[test]
fn columns_count_tabs_to_the_next_stop_and_wide_characters_twice() {
    assert_eq!(columns("abc"), 3);
    assert_eq!(columns("\tx"), 9);
    assert_eq!(columns("ab\tx"), 9, "a tab runs to the next multiple of eight");
    assert_eq!(columns("名前"), 4);
    assert_eq!(columns(""), 0);
}

#[test]
fn a_caret_on_screen_moves_nothing_and_one_off_screen_is_brought_just_inside() {
    // A 100px view showing 200..300, a 20px span, and a 10px margin.
    assert_eq!(reveal(200.0, 100.0, 240.0, 20.0, 10.0), None);
    assert_eq!(
        reveal(200.0, 100.0, 150.0, 20.0, 10.0),
        Some(140.0),
        "above: the span's start, less the margin"
    );
    assert_eq!(
        reveal(200.0, 100.0, 300.0, 20.0, 10.0),
        Some(230.0),
        "below: the span's end and the margin at the bottom edge"
    );
    assert_eq!(reveal(5.0, 100.0, 0.0, 20.0, 10.0), Some(0.0), "never past the start");
}

#[test]
fn a_caret_keeps_its_place_in_the_text_when_its_line_is_shifted() {
    assert_eq!(shift_column(6, 0, 2), 8, "text inserted before the caret pushes it along");
    assert_eq!(shift_column(1, 4, 2), 1, "text inserted after the caret leaves it");
    assert_eq!(shift_column(6, 0, -4), 2);
    assert_eq!(shift_column(2, 0, -4), 0, "a caret inside removed text lands where the text was");
    assert_eq!(shift_column(5, 4, -2), 4);
}

#[test]
fn typing_below_the_screen_scrolls_the_caret_back_into_view() {
    let text = "line\n".repeat(100);
    let mut editor = editor("notes.txt", &text);
    let _ = editor.update(Message::Resized(Size::new(500.0, 200.0)));
    caret(&mut editor, 60, 4);
    typed(&mut editor, "!");
    let (_, y) = editor.scroll;
    let caret_top = PADDING + 60.0 * LINE_HEIGHT;
    assert!(
        y <= caret_top && caret_top + LINE_HEIGHT <= y + 200.0,
        "the caret's line is inside the view at scroll {y}"
    );
    let _ = editor.update(Message::Command(Command::Page(-1)));
    assert_eq!(editor.content.cursor().position.line, 51, "a page up in a view of ten lines moves nine");
}
