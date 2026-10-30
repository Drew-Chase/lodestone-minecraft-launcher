//! The `LODESTONE - v{VERSION}` tag, shared by the Windows titlebar and the login
//! page's top bar. Callers centre it.

use iced::Element;
use iced::widget::Text;

use crate::app::LodestoneLauncher;
use crate::app::lang::t_with;
use crate::app::theme::{color, fonts};

/// The tag in the brand's mono style.
pub fn brand_tag<'a, Message: 'a>() -> Element<'a, Message> {
    Text::new(t_with("titlebar.tag", &[("VERSION", LodestoneLauncher::VERSION)]))
        .font(fonts::JetBrainsMono::REGULAR)
        .size(11)
        .color(color::Text::PLACEHOLDER)
        .into()
}
