use super::*;

#[test]
fn an_html_picture_becomes_a_markdown_image_with_its_alt_text() {
    assert_eq!(
        to_markdown("<img src=\"https://cdn.example/a.png\" alt=\"Preview\" width=\"200\">\n"),
        "![Preview](https://cdn.example/a.png)\n"
    );
}

#[test]
fn a_bare_attribute_value_is_read_up_to_the_next_space() {
    assert_eq!(to_markdown("<img src=a.png>"), "![](a.png)");
}

#[test]
fn fenced_code_is_left_as_it_is() {
    let readme = "```html\n<img src=\"x.png\">\n```\n";
    assert_eq!(to_markdown(readme), readme);
}

#[test]
fn comments_end_lines_and_other_tags_keep_their_text() {
    assert_eq!(to_markdown("<!-- hidden -->Hello<br><b>world</b>\n"), "Hello\nworld\n");
}

#[test]
fn a_less_than_sign_that_opens_no_tag_stays_as_text() {
    assert_eq!(to_markdown("1 < 2 and 3 > 2\n"), "1 < 2 and 3 > 2\n");
}
