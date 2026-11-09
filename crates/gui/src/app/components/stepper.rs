//! The wizard's stepper: numbered steps joined by rules, with the active step lit, the steps before it ticked, and an
//! optional caption on the right. The steps are labels only: a wizard moves on through its own buttons.

use aurora_leaf::Icon;
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Background, Border, Element, Fill, Padding};

use crate::app::theme::{color, fonts, utility};

/// The side of a step's circle, in pixels.
const CIRCLE: f32 = 20.0;

/// The stepper over `labels`, with `active` as the step in progress. A step before `active` is ticked. `caption`, when
/// given, sits on the right.
pub fn stepper<'a, M: 'a>(labels: &[&'static str], active: usize, caption: Option<String>) -> Element<'a, M> {
    let mut items: Vec<Element<'a, M>> = Vec::with_capacity(labels.len() * 2 + 2);
    for (index, label) in labels.iter().enumerate() {
        if index > 0 {
            items.push(container(Space::new()).width(24).height(1).style(rule_style).into());
        }
        items.push(step(index, label, index.cmp(&active)));
    }
    if let Some(caption) = caption {
        items.push(Space::new().width(Fill).into());
        items.push(
            text(caption)
                .size(11.5)
                .font(fonts::JetBrainsMono::REGULAR)
                .style(|_| text::Style {
                    color: Some(color::text_muted()),
                })
                .into(),
        );
    }
    let bar = container(row(items).spacing(16).align_y(Alignment::Center))
        .padding(Padding::new(0.0).left(32.0).right(32.0).top(14.0).bottom(14.0))
        .width(Fill);
    column![bar, container(Space::new()).width(Fill).height(1).style(rule_style)].into()
}

/// One step: its circle, holding its number or a tick, and its label. `order` compares the step with the active one.
fn step<'a, M: 'a>(index: usize, label: &'static str, order: std::cmp::Ordering) -> Element<'a, M> {
    let (lit, ink, fill, border) = match order {
        std::cmp::Ordering::Less => (true, color::primary(), color::primary_tint(), color::primary_tint()),
        std::cmp::Ordering::Equal => (true, color::primary_on(), color::primary(), color::primary()),
        std::cmp::Ordering::Greater => (false, color::text_muted(), color::surface_bg1(), color::surface_line_strong()),
    };
    let number: Element<'a, M> = if order == std::cmp::Ordering::Less {
        utility::icon(Icon::lucide().check(), 11.0, Some((ink, ink)))
    } else {
        text((index + 1).to_string())
            .size(11.0)
            .font(fonts::Inter::BOLD)
            .style(move |_| text::Style { color: Some(ink) })
            .into()
    };
    let circle = container(number)
        .width(CIRCLE)
        .height(CIRCLE)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(fill)),
            border: Border {
                color: border,
                width: 1.0,
                radius: (CIRCLE / 2.0).into(),
            },
            ..Default::default()
        });
    let text_ink = if lit { color::primary() } else { color::text_muted() };
    row![
        circle,
        text(label)
            .size(12.5)
            .font(fonts::Inter::MEDIUM)
            .style(move |_| text::Style { color: Some(text_ink) }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// The hairline a rule or the stepper's bottom edge is drawn with.
fn rule_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color::surface_line())),
        ..Default::default()
    }
}
