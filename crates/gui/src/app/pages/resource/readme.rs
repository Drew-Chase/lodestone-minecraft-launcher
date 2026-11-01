//! A project's markdown as the page draws it. The markdown parser drops raw HTML, and readmes lean on HTML for
//! their pictures, so the HTML is turned into markdown first: an `<img>` becomes a markdown image, a `<br>` a line
//! break, and any other tag goes while its text stays. Fenced code blocks are left alone.

use std::collections::HashMap;

use iced::widget::{container, image, markdown, rich_text};
use iced::{Element, Fill, Theme};

use super::Message;
use crate::app::theme::color;

/// The settings the markdown draws with: body text at 14 px, and the platform's link colour.
pub fn settings() -> markdown::Settings {
    let style = markdown::Style {
        link_color: color::Primary::DEFAULT,
        ..markdown::Style::from(&Theme::Dark)
    };
    markdown::Settings::with_text_size(14, style)
}

/// Draws a readme's pictures from the ones the page has loaded, and its links as page messages. A picture still
/// loading, or one that failed, shows its alt text.
pub struct ReadmeViewer<'a> {
    pub pictures: &'a HashMap<String, image::Handle>,
}

impl<'a> markdown::Viewer<'a, Message> for ReadmeViewer<'a> {
    fn on_link_click(url: markdown::Uri) -> Message {
        Message::Link(url)
    }

    fn image(&self, settings: markdown::Settings, url: &'a markdown::Uri, _title: &'a str, alt: &markdown::Text) -> Element<'a, Message> {
        match self.pictures.get(url) {
            Some(handle) => image(handle.clone()).width(Fill).into(),
            None => container(rich_text(alt.spans(settings.style)).on_link_click(Self::on_link_click))
                .padding(settings.spacing.0)
                .into(),
        }
    }
}

/// The markdown a readme or changelog draws, with its HTML turned into what the markdown parser keeps.
pub fn to_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fenced = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            out.push_str(line);
        } else if fenced {
            out.push_str(line);
        } else {
            out.push_str(&strip_html(line));
        }
    }
    out
}

/// One line with its tags turned into markdown or dropped. A `<` that does not open a tag stays as text.
fn strip_html(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let opens_tag = tail
            .as_bytes()
            .get(1)
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'/' || *byte == b'!');
        if !opens_tag {
            out.push('<');
            rest = &tail[1..];
            continue;
        }
        if tail.starts_with("<!--") {
            // A comment runs to its end marker. One that is not closed on this line is dropped with the rest of the line.
            rest = tail.find("-->").map_or("", |end| &tail[end + 3..]);
            continue;
        }
        let Some(end) = tail.find('>') else {
            out.push_str(tail);
            return out;
        };
        let tag = &tail[1..end];
        rest = &tail[end + 1..];
        let name = tag
            .trim_start_matches('/')
            .split(|character: char| character.is_whitespace() || character == '/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match name.as_str() {
            "img" => {
                if let Some(picture) = image_markdown(tag) {
                    out.push_str(&picture);
                }
            }
            "br" => out.push('\n'),
            _ => {}
        }
    }
    out.push_str(rest);
    out
}

/// An `<img>` tag as a markdown image, or `None` when it names no picture.
fn image_markdown(tag: &str) -> Option<String> {
    let source = attribute(tag, "src")?.replace(' ', "%20");
    let alt = attribute(tag, "alt").unwrap_or_default().replace(['[', ']'], "");
    Some(format!("![{alt}]({source})"))
}

/// The value of an attribute in a tag, quoted with either quote mark, or bare up to the next space.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let at = lower.find(&format!("{name}="))? + name.len() + 1;
    let value = &tag[at..];
    let quote = value.chars().next()?;
    if quote == '"' || quote == '\'' {
        let end = value[1..].find(quote)?;
        Some(value[1..1 + end].to_string())
    } else {
        Some(value.split_whitespace().next()?.to_string())
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/resource/readme.rs"]
mod tests;
