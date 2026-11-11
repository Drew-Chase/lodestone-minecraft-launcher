use super::super::tokens::tokenize;
use super::*;

/// The tree of a document.
fn tree(html: &str) -> Element {
    build(tokenize(html))
}

/// The tree's shape as nested names, with text quoted: `p("a" b("c"))`.
fn shape(element: &Element) -> String {
    let children: Vec<String> = element
        .children
        .iter()
        .map(|child| match child {
            Node::Text(text) => format!("{text:?}"),
            Node::Element(element) => shape(element),
        })
        .collect();
    if element.name.is_empty() {
        children.join(" ")
    } else {
        format!("{}({})", element.name, children.join(" "))
    }
}

#[test]
fn elements_nest_and_text_joins_its_parent() {
    assert_eq!(shape(&tree("<p>a <b>c</b></p>")), "p(\"a \" b(\"c\"))");
}

#[test]
fn void_elements_take_no_children() {
    assert_eq!(shape(&tree("<p>a<br>b<img src=x>c</p>")), "p(\"a\" br() \"b\" img() \"c\")");
}

#[test]
fn a_block_closes_an_open_paragraph() {
    assert_eq!(shape(&tree("<p>one<div>two</div>")), "p(\"one\") div(\"two\")");
    assert_eq!(shape(&tree("<p>one<p>two")), "p(\"one\") p(\"two\")");
}

#[test]
fn a_block_inside_a_cell_does_not_close_a_paragraph_outside_it() {
    assert_eq!(shape(&tree("<p>a<td>b<div>c</div>")), "p(\"a\" td(\"b\" div(\"c\")))");
}

#[test]
fn a_table_closes_an_open_paragraph_and_end_tags_stay_inside_their_cell() {
    assert_eq!(
        shape(&tree("<p>a<span><table><tr><td><div>b</span>c</div></td></tr></table></span></p>")),
        "p(\"a\" span()) table(tr(td(div(\"bc\"))))"
    );
}

#[test]
fn a_list_item_closes_the_one_before_it_but_not_its_parent_list_item() {
    assert_eq!(shape(&tree("<ul><li>a<li>b</ul>")), "ul(li(\"a\") li(\"b\"))");
    assert_eq!(shape(&tree("<ul><li>a<ul><li>b</ul></ul>")), "ul(li(\"a\" ul(li(\"b\"))))");
}

#[test]
fn rows_and_cells_close_the_ones_before_them() {
    assert_eq!(
        shape(&tree("<table><tr><td>a<td>b<tr><th>c</table>")),
        "table(tr(td(\"a\") td(\"b\")) tr(th(\"c\")))"
    );
}

#[test]
fn a_heading_inside_a_heading_closes_the_first() {
    assert_eq!(shape(&tree("<h1>a<h2>b</h2>")), "h1(\"a\") h2(\"b\")");
}

#[test]
fn a_link_inside_a_link_closes_the_first() {
    assert_eq!(shape(&tree("<a href=1>a<a href=2>b</a>")), "a(\"a\") a(\"b\")");
}

#[test]
fn an_end_tag_with_nothing_to_close_is_ignored() {
    assert_eq!(shape(&tree("a</span></p>b")), "\"ab\"");
}

#[test]
fn an_end_tag_closes_the_elements_open_inside_it() {
    assert_eq!(shape(&tree("<div><b><i>a</div>b")), "div(b(i(\"a\"))) \"b\"");
}

#[test]
fn an_end_br_is_a_line_break() {
    assert_eq!(shape(&tree("a</br>b")), "\"a\" br() \"b\"");
}

#[test]
fn deep_nesting_stops_at_the_limit_and_keeps_its_text() {
    let html = format!("{}deep{}after", "<div>".repeat(10_000), "</div>".repeat(10_000));
    let root = tree(&html);
    let mut depth = 0;
    let mut element = &root;
    while let Some(Node::Element(child)) = element.children.first() {
        depth += 1;
        element = child;
    }
    assert_eq!(depth, MAX_DEPTH);
    assert_eq!(element.children, vec![Node::Text("deep".to_owned())]);
    assert_eq!(root.children.last(), Some(&Node::Text("after".to_owned())));
}

#[test]
fn classes_and_text_content_read() {
    let root = tree("<div class=\"Spoiler wide\"><pre>a<br>b <i>c</i></pre></div>");
    let Some(Node::Element(div)) = root.children.first() else {
        panic!("no div");
    };
    assert!(div.has_class("spoiler"));
    assert!(!div.has_class("spoil"));
    assert_eq!(div.text_content(), "a\nb c");
}
