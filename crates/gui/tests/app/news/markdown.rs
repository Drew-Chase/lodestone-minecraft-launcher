use super::*;

#[test]
fn paragraphs_keep_their_emphasis_and_decode_entities() {
    assert_eq!(to_markdown("<p>Hello <strong>world</strong> &#x26; more</p>"), "Hello **world** & more");
}

#[test]
fn headings_become_markdown_headings_on_lines_of_their_own() {
    assert_eq!(
        to_markdown("<h1>Experimental Features</h1>\n<h2>Breeze</h2>"),
        "# Experimental Features\n\n## Breeze"
    );
}

#[test]
fn lists_and_links_become_markdown_bullets_with_their_targets() {
    let html = "<ul>\n<li>Updated the model</li>\n<li><a href=\"https://bugs.mojang.com/browse/MC-1\" target=\"_blank\">MC-1</a> Fixed</li>\n</ul>";
    assert_eq!(
        to_markdown(html),
        "- Updated the model\n- [MC-1](https://bugs.mojang.com/browse/MC-1) Fixed"
    );
}

#[test]
fn nested_lists_indent_under_their_parent_and_numbered_lists_count_up() {
    assert_eq!(to_markdown("<ul><li>A<ul><li>B</li></ul></li></ul>"), "- A\n  - B");
    assert_eq!(to_markdown("<ol><li>x</li><li>y</li></ol>"), "1. x\n2. y");
}

#[test]
fn code_keeps_its_symbols_and_text_outside_it_is_escaped() {
    assert_eq!(
        to_markdown("<code>block_id</code> and <code>a &#x3C; b</code> &#x3C; snake_case"),
        "`block_id` and `a < b` \\< snake\\_case"
    );
}

#[test]
fn preformatted_blocks_keep_their_lines() {
    assert_eq!(to_markdown("<pre><code>one\ntwo</code></pre>"), "```\none\ntwo\n```");
}

#[test]
fn unknown_tags_go_and_their_text_stays_while_comments_go_with_theirs() {
    assert_eq!(to_markdown("<span class=\"x\">text</span> <!-- hidden -->here"), "text here");
}

#[test]
fn pictures_become_markdown_images_with_absolute_addresses() {
    assert_eq!(
        to_markdown("<img src=\"/images/a.png\" alt=\"A diagram\">"),
        "![A diagram](https://launchercontent.mojang.com/images/a.png)"
    );
}

#[test]
fn a_less_than_sign_that_opens_no_tag_is_text() {
    assert_eq!(to_markdown("1 < 2"), "1 \\< 2");
}

#[test]
fn a_real_patch_note_body_reads_as_its_sections() {
    let html = "<p>We are releasing Snapshot 24w03b to fix a crash, along with some bug fixes and minor changes.</p>\n\
                <p>Happy Mining!</p>\n\
                <h1>Experimental Features</h1>\n\
                <h2>Breeze</h2>\n\
                <ul>\n\
                <li>Updated the model, texture, and animation of the Wind Charge projectile to give it a more dynamic look and feel</li>\n\
                </ul>";
    assert_eq!(
        to_markdown(html),
        "We are releasing Snapshot 24w03b to fix a crash, along with some bug fixes and minor changes.\n\n\
         Happy Mining!\n\n\
         # Experimental Features\n\n\
         ## Breeze\n\n\
         - Updated the model, texture, and animation of the Wind Charge projectile to give it a more dynamic look and feel"
    );
}
