use iced::Color;

use super::super::parse;
use super::*;

/// A resolver for the tests: full web addresses stay, paths join onto `https://site.test`, and anything else is
/// rejected.
fn resolve(href: &str) -> Option<String> {
    if href.starts_with("https://") || href.starts_with("http://") {
        Some(href.to_owned())
    } else if href.starts_with('/') {
        Some(format!("https://site.test{href}"))
    } else {
        None
    }
}

/// The document of some HTML.
fn read(html: &str) -> Document {
    parse(html, resolve)
}

/// The blocks' shape in short: `p("text")`, `h2^("centred")`, `pics[a.png b.png]`, `ul[item | item]`, `ol3[...]`,
/// `quote[...]`, `code("...")`, `hr`, `table[cell , cell / cell]` and `spoiler0("title")[...]`. `^` marks centred
/// blocks and `>` right-aligned ones.
fn outline(blocks: &[Block]) -> String {
    blocks.iter().map(block_outline).collect::<Vec<_>>().join(" ")
}

/// Text in quotes, with its line breaks written as `\n`. Other characters, a no-break space among them, stay as they are.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\n', "\\n"))
}

fn block_outline(block: &Block) -> String {
    match block {
        Block::Text { spans, heading, align } => {
            let text: String = spans.iter().map(|span| span.text.as_str()).collect();
            let kind = heading.map_or_else(|| "p".to_owned(), |level| format!("h{level}"));
            format!("{kind}{}({})", mark(*align), quoted(&text))
        }
        Block::Pictures { pictures, align } => {
            let names: Vec<&str> = pictures
                .iter()
                .map(|picture| picture.url.rsplit('/').next().unwrap_or_default())
                .collect();
            format!("pics{}[{}]", mark(*align), names.join(" "))
        }
        Block::List { start, items } => {
            let kind = start.map_or_else(|| "ul".to_owned(), |start| format!("ol{start}"));
            let items: Vec<String> = items.iter().map(|item| outline(item)).collect();
            format!("{kind}[{}]", items.join(" | "))
        }
        Block::Quote(blocks) => format!("quote[{}]", outline(blocks)),
        Block::Code(code) => format!("code({})", quoted(code)),
        Block::Rule => "hr".to_owned(),
        Block::Table(rows) => {
            let rows: Vec<String> = rows
                .iter()
                .map(|row| row.iter().map(|cell| outline(cell)).collect::<Vec<_>>().join(" , "))
                .collect();
            format!("table[{}]", rows.join(" / "))
        }
        Block::Spoiler { index, title, blocks } => {
            let title = title.as_ref().map_or_else(String::new, |title| format!("({})", quoted(title)));
            format!("spoiler{index}{title}[{}]", outline(blocks))
        }
    }
}

fn mark(align: Align) -> &'static str {
    match align {
        Align::Left => "",
        Align::Center => "^",
        Align::Right => ">",
    }
}

/// The spans of the document's only text block.
fn spans_of(document: &Document) -> &[Span] {
    match document.blocks.as_slice() {
        [Block::Text { spans, .. }] => spans,
        other => panic!("expected one text block, got {other:?}"),
    }
}

/// The pictures of the document's only row of pictures.
fn pictures_of(document: &Document) -> &[Picture] {
    match document.blocks.as_slice() {
        [Block::Pictures { pictures, .. }] => pictures,
        other => panic!("expected one row of pictures, got {other:?}"),
    }
}

#[test]
fn whitespace_collapses_and_lines_drop_their_edges() {
    let document = read("<p>  Hello \n  <b>bold</b>   world  </p>");
    let spans = spans_of(&document);
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].text, "Hello ");
    assert_eq!(spans[1].text, "bold");
    assert!(spans[1].style.bold);
    assert_eq!(spans[2].text, " world");
}

#[test]
fn paragraphs_that_show_nothing_are_dropped() {
    assert_eq!(outline(&read("<p>&nbsp;</p>\n<p> </p><p><br></p><p>x</p>").blocks), "p(\"x\")");
}

#[test]
fn a_no_break_space_stays_inside_text() {
    assert_eq!(outline(&read("<p>1.0 &nbsp;•&nbsp; mods</p>").blocks), "p(\"1.0 \u{a0}•\u{a0} mods\")");
}

#[test]
fn neighbouring_spans_in_one_style_merge() {
    let document = read("<p>a<span>b</span><span style=\"\">c</span></p>");
    assert_eq!(spans_of(&document).len(), 1);
}

#[test]
fn breaks_end_lines_and_keep_at_most_one_blank_line() {
    assert_eq!(outline(&read("<p><br>a<br>b<br><br><br><br>c<br></p>").blocks), "p(\"a\\nb\\n\\nc\")");
}

#[test]
fn a_heading_keeps_its_level_and_the_colour_and_size_of_its_span() {
    let document = read("<h1><span style=\"color:#9d5cff;font-size:40px\">TITLE</span></h1>");
    match document.blocks.as_slice() {
        [Block::Text { spans, heading, .. }] => {
            assert_eq!(*heading, Some(1));
            assert_eq!(spans[0].style.color, Some(Color::from_rgb8(0x9d, 0x5c, 0xff)));
            assert_eq!(spans[0].style.size, Some(35.0));
        }
        other => panic!("expected a heading, got {other:?}"),
    }
}

#[test]
fn blocks_inherit_the_alignment_of_a_centred_container() {
    assert_eq!(
        outline(&read("<div style=\"text-align:center\"><h2>T</h2><p>x</p></div><p>y</p><center>z</center><p align=right>w</p>").blocks),
        "h2^(\"T\") p^(\"x\") p(\"y\") p^(\"z\") p>(\"w\")"
    );
}

#[test]
fn a_picture_in_a_link_takes_the_link() {
    let document = read("<p><a href=\"/linkout?x\"><img src=\"https://cdn.test/a.png\" alt=\" The  banner \"></a></p>");
    let pictures = pictures_of(&document);
    assert_eq!(pictures[0].url, "https://cdn.test/a.png");
    assert_eq!(pictures[0].alt, "The banner");
    assert_eq!(pictures[0].link, Some(0));
    assert_eq!(document.links, vec!["https://site.test/linkout?x".to_owned()]);
}

#[test]
fn pictures_side_by_side_share_a_row_with_their_sizes_and_links() {
    let document = read(
        "<p><a href=\"https://forge.test\"><img src=\"https://i.test/k.png\" width=\"156\" height=\"156\"></a>\n\
         <a href=\"https://neo.test\"><img src=\"https://i.test/9.png\" style=\"width: 120px\" width=\"156\"></a></p>",
    );
    let pictures = pictures_of(&document);
    assert_eq!(pictures.len(), 2);
    assert_eq!((pictures[0].width, pictures[0].height), (Some(156.0), Some(156.0)));
    assert_eq!(pictures[1].width, Some(120.0), "the style wins over the attribute");
    assert_eq!((pictures[0].link, pictures[1].link), (Some(0), Some(1)));
}

#[test]
fn text_around_a_picture_splits_into_lines_and_a_break_ends_a_row() {
    assert_eq!(
        outline(&read("<p>before <img src=\"https://i.test/a.png\"> after</p><p><img src=\"https://i.test/b.png\"><br><img src=\"https://i.test/c.png\"></p>").blocks),
        "p(\"before\") pics[a.png] p(\"after\") pics[b.png] pics[c.png]"
    );
}

#[test]
fn a_picture_inside_a_heading_splits_off() {
    assert_eq!(
        outline(&read("<h2>Top <img src=\"https://i.test/a.png\"> tail</h2>").blocks),
        "h2(\"Top\") pics[a.png] h2(\"tail\")"
    );
}

#[test]
fn lists_nest_and_numbered_lists_count_from_their_start() {
    assert_eq!(
        outline(&read("<ul><li>A<ul><li>B</li></ul></li><li>C</li></ul><ol start=\"3\"><li>x</li><li>y</li></ol><ol><li>z</li></ol>").blocks),
        "ul[p(\"A\") ul[p(\"B\")] | p(\"C\")] ol3[p(\"x\") | p(\"y\")] ol1[p(\"z\")]"
    );
}

#[test]
fn content_outside_a_list_item_joins_the_item_before_it() {
    assert_eq!(
        outline(&read("<ul>first<li>A</li><ul><li>B</li></ul><li></li></ul>").blocks),
        "ul[p(\"first\") | p(\"A\") ul[p(\"B\")]]"
    );
}

#[test]
fn spoilers_are_numbered_in_reading_order_and_details_take_their_summary_as_a_title() {
    assert_eq!(
        outline(
            &read(
                "<h3>Mods</h3><div class=\"spoiler\"><p>one</p><div class=\"Spoiler\"><p>two</p></div></div>\
                 <details><summary>More  info</summary><p>three</p></details><div class=\"spoiler\"> </div>"
            )
            .blocks
        ),
        "h3(\"Mods\") spoiler0[p(\"one\") spoiler1[p(\"two\")]] spoiler2(\"More info\")[p(\"three\")]"
    );
}

#[test]
fn a_youtube_embed_becomes_a_linked_thumbnail() {
    let document = read(
        "<p><span><iframe src=\"https://www.youtube.com/embed/rR8W-f9YhYA?wmode=transparent\" width=\"638\" height=\"358\" \
         allowfullscreen=\"allowfullscreen\"></iframe></span></p>",
    );
    let pictures = pictures_of(&document);
    assert_eq!(pictures[0].url, "https://i.ytimg.com/vi/rR8W-f9YhYA/hqdefault.jpg");
    assert_eq!(pictures[0].width, Some(638.0));
    assert!(pictures[0].video);
    assert_eq!(document.links, vec!["https://www.youtube.com/watch?v=rR8W-f9YhYA".to_owned()]);
}

#[test]
fn an_embed_with_no_width_takes_the_default_video_width() {
    let document = read("<iframe src=\"https://youtu.be/abcdefghijk\"></iframe>");
    assert_eq!(pictures_of(&document)[0].width, Some(VIDEO_WIDTH));
}

#[test]
fn youtube_ids_read_from_each_embed_address() {
    assert_eq!(
        youtube_id("https://www.youtube-nocookie.com/embed/abc_DEF-123?rel=0"),
        Some("abc_DEF-123")
    );
    assert_eq!(youtube_id("https://youtu.be/abc_DEF-123"), Some("abc_DEF-123"));
    assert_eq!(youtube_id("https://www.youtube.com/watch?v=abc_DEF-123"), None);
    assert_eq!(youtube_id("https://notyoutube.com/embed/abc_DEF-123"), None);
    assert_eq!(youtube_id("https://www.youtube.com/embed/<script>"), None);
}

#[test]
fn another_embed_becomes_a_line_linking_to_it() {
    let document = read("<iframe src=\"https://player.test/v/1\"></iframe>");
    let spans = spans_of(&document);
    assert_eq!(spans[0].text, "https://player.test/v/1");
    assert_eq!(spans[0].link, Some(0));
}

#[test]
fn preformatted_text_keeps_its_lines() {
    assert_eq!(
        outline(&read("<pre>\n  line one\n  <b>line</b> two\n\n</pre>").blocks),
        "code(\"  line one\\n  line two\")"
    );
}

#[test]
fn rules_and_quotes_are_blocks() {
    assert_eq!(
        outline(&read("<p>a</p><hr><blockquote><p>b</p><blockquote>c</blockquote></blockquote>").blocks),
        "p(\"a\") hr quote[p(\"b\") quote[p(\"c\")]]"
    );
}

#[test]
fn a_table_reads_its_rows_and_cells_with_its_caption_above() {
    let document = read(
        "<table><caption>Cap</caption><thead><tr><th>H1</th><th>H2</th></tr></thead>\
         <tbody><tr><td>a</td><td><img src=\"https://i.test/x.png\"></td></tr></tbody></table>",
    );
    assert_eq!(
        outline(&document.blocks),
        "p(\"Cap\") table[p(\"H1\") , p(\"H2\") / p(\"a\") , pics[x.png]]"
    );
    let Some(Block::Table(rows)) = document.blocks.last() else {
        panic!("no table");
    };
    let [Block::Text { spans, .. }] = rows[0][0].as_slice() else {
        panic!("no header text");
    };
    assert!(spans[0].style.bold, "a header cell is bold");
}

#[test]
fn rejected_addresses_drop_pictures_and_links() {
    let document = read("<p><a href=\"javascript:alert(1)\">x</a><img src=\"data:image/png;base64,AA\"><a href=\"#top\">y</a></p>");
    assert_eq!(outline(&document.blocks), "p(\"xy\")");
    assert!(document.links.is_empty());
}

#[test]
fn skipped_elements_take_their_content_with_them() {
    assert_eq!(
        outline(&read("<p>a<svg><text>no</text></svg><button>no</button>b</p>").blocks),
        "p(\"ab\")"
    );
}

#[test]
fn a_link_used_twice_is_kept_once() {
    let document = read("<a href=\"https://a.test\">one</a> <a href=\"https://a.test\">two</a>");
    assert_eq!(document.links.len(), 1);
}

#[test]
fn containers_nest_no_deeper_than_the_limit() {
    let html = format!("{}deep{}", "<blockquote>".repeat(12), "</blockquote>".repeat(12));
    let document = read(&html);
    let mut depth = 0;
    let mut blocks = document.blocks.as_slice();
    while let [Block::Quote(inner)] = blocks {
        depth += 1;
        blocks = inner;
    }
    assert_eq!(depth, MAX_NESTING);
    assert_eq!(outline(blocks), "p(\"deep\")");
}

#[test]
fn a_description_shaped_like_a_curseforge_modpack_page_reads_as_its_sections() {
    let html = "<p><img src=\"https://media.test/banner.png\" alt=\"Banner\"></p>\n<hr>\n\
        <div style=\"text-align:center\"><h1><span style=\"color:#9d5cff;font-size:40px\">BIG TITLE</span></h1>\
        <p><span style=\"\">Loader 1.0 &nbsp;•&nbsp; Many mods</span></p>\
        <p>Read the <a href=\"/linkout?remoteUrl=https%253a%252f%252fchat.test\" rel=\"nofollow\">chat</a>.</p></div>\n<hr>\n\
        <ul>\n<li>One <strong>thing</strong> <span style=\"\"><em>(Mod A)</em></span></li>\n<li>Two</li>\n</ul>\n\
        <h3><span style=\"color:#9d5cff\">Section</span></h3>\n\
        <div class=\"spoiler\"><h4>Inside</h4><ul><li>Hidden</li></ul></div>\n\
        <div style=\"text-align:center\"><p><a href=\"/linkout?remoteUrl=https%253a%252f%252fhost.test\" target=\"_blank\">\
        <img src=\"https://host.test/ad.webp\" alt=\"Host\"></a></p></div>";
    let document = read(html);
    assert_eq!(
        outline(&document.blocks),
        "pics[banner.png] hr h1^(\"BIG TITLE\") p^(\"Loader 1.0 \u{a0}•\u{a0} Many mods\") p^(\"Read the chat.\") hr \
         ul[p(\"One thing (Mod A)\") | p(\"Two\")] h3(\"Section\") spoiler0[h4(\"Inside\") ul[p(\"Hidden\")]] pics^[ad.webp]"
    );
    assert_eq!(document.links.len(), 2);
}
