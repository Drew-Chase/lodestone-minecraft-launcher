//! Tabs — the style guide's `.tab` pill row (§07), or the underline row the
//! instance page uses ([`Tabs::underline`]).
//!
//! Inactive tabs are quiet (`--ink-2` on transparent); the active tab is a
//! Lode Green pill with near-black ink. The pill *glides* between tabs on a
//! 300ms ease rather than teleporting, and a faint white highlight slides
//! under the hovered tab — the guide's lift-family motion (§08). The hover
//! tint is 8% white rather than the spec's 4%, which is imperceptible on
//! this panel.
//!
//! The underline style drops the pill. The active label turns Lode Green and
//! a 2px bar glides under it, on the row's bottom two pixels. A 1px rule runs
//! across the full width on the bottom pixel, and the bar paints over it.
//!
//! Labels are real [`Text`] children: iced measures and anchors them, so the
//! pill geometry comes straight from the child layout nodes and can never
//! drift from the glyphs. Hover and hit regions are derived from the same
//! child bounds — each tab owns the span from the midpoint between it and its
//! previous label to the midpoint between it and the next — so what you see
//! highlighted is exactly what you hit.
//!
//! ```text
//! Tabs::new(vec!["Mods", "Resource packs", "Worlds"])
//!     .active(self.tab)
//!     .on_select(|i| Message::TabSelected(i))
//! ```

mod widget;

use crate::app::theme::{color, fonts};
use iced::mouse::Cursor;
use iced::widget::Text;
use iced::{Color, Element, Length, Rectangle, Renderer as BaseRenderer, Theme};
use iced_anim::Animated;

/// Horizontal padding inside one pill tab — `.tab` is `8px 14px`.
const H_PADDING: f32 = 14.0;
/// Horizontal padding inside one underline tab — the label + 32px is the slot.
const UNDERLINE_H_PADDING: f32 = 16.0;
/// Gap between adjacent tabs.
const SPACING: f32 = 4.0;
/// The `.tab` line height — 13px text plus the 8px vertical padding.
const TAB_HEIGHT: f32 = 34.0;
/// The underline row: the label centred in the box, with the 1px rule on the
/// last pixel and the 2px bar over it.
const UNDERLINE_HEIGHT: f32 = 38.0;
/// The underline bar's thickness, on the bottom two pixels of the row.
const BAR_THICKNESS: f32 = 2.0;

/// An animated tab row (`.tab`), or the underline row used by the instance page.
pub struct Tabs<Message> {
    labels: Vec<String>,
    active: usize,
    on_select: Option<Box<dyn Fn(usize) -> Message>>,
    style: Style,
    /// Left inset before the first tab, so the row can start at the page gutter.
    inset: f32,
    width: Length,
    height: Length,
    disabled: bool,
}

/// How the active tab is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    /// `.tab` — a Lode Green pill behind the active label.
    Pill,
    /// A Lode Green bar on the rule under the active label.
    Underline,
}

#[derive(Debug)]
struct State {
    /// Label bounds relative to the row, recorded during layout. These are
    /// where the glyphs actually are — the source of truth for hover and
    /// hit testing.
    text_bounds: Vec<Rectangle>,
    /// Pill slot bounds relative to the row (label + horizontal padding).
    tab_bounds: Vec<Rectangle>,
    /// Animated active-pill geometry, relative to the row. The underline bar
    /// uses the same slot, so it glides with the pill.
    pill_x: Animated<f32>,
    pill_width: Animated<f32>,
    /// Hovered tab, if any.
    hovered: Option<usize>,
    hover_x: Animated<f32>,
    hover_width: Animated<f32>,
    hover_alpha: Animated<f32>,
    is_pressed: bool,
    /// True until the first layout, so the pill settles instead of flying in.
    first_layout: bool,
}

/// The contiguous hover/hit span of the tab at `index`: bounded by the
/// midpoints between adjacent labels (the outer tabs extend to the row's
/// padding). Every point of the row belongs to exactly one tab.
fn region_span(text_bounds: &[Rectangle], index: usize, padding: f32) -> (f32, f32) {
    let text = &text_bounds[index];

    let start = if index == 0 {
        text.x - padding
    } else {
        let previous = &text_bounds[index - 1];
        (previous.x + previous.width + text.x) / 2.0
    };

    let end = if index + 1 == text_bounds.len() {
        text.x + text.width + padding
    } else {
        let next = &text_bounds[index + 1];
        (text.x + text.width + next.x) / 2.0
    };

    (start, end)
}

impl<Message> Tabs<Message> {
    /// Creates a tab row from the given labels.
    pub fn new<I: IntoIterator<Item = impl Into<String>>>(labels: I) -> Self {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            active: 0,
            on_select: None,
            style: Style::Pill,
            inset: 0.0,
            width: Length::Shrink,
            height: Length::Fixed(TAB_HEIGHT),
            disabled: false,
        }
    }

    /// Sets the selected tab index.
    pub fn active(mut self, active: usize) -> Self {
        self.active = active;
        self
    }

    /// Sets the message produced when a tab is selected.
    ///
    /// Without an `on_select` closure the tabs are not interactive.
    pub fn on_select(mut self, on_select: impl Fn(usize) -> Message + 'static) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Switches to the underline style: no pill, a Lode Green label, and a
    /// 2px bar with a full-width rule beneath the row.
    pub fn underline(mut self) -> Self {
        self.style = Style::Underline;
        self.height = Length::Fixed(UNDERLINE_HEIGHT);
        self
    }

    /// Leaves `inset` pixels before the first tab, for a row that starts
    /// inside the page gutter while its rule spans the full width.
    pub fn inset(mut self, inset: f32) -> Self {
        self.inset = inset;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Whether the tabs can be interacted with at all.
    fn interactive(&self) -> bool {
        !self.disabled && self.on_select.is_some()
    }

    /// Horizontal padding inside one tab for the current style.
    fn h_padding(&self) -> f32 {
        match self.style {
            Style::Pill => H_PADDING,
            Style::Underline => UNDERLINE_H_PADDING,
        }
    }

    /// The row's height for the current style.
    fn tab_height(&self) -> f32 {
        match self.style {
            Style::Pill => TAB_HEIGHT,
            Style::Underline => UNDERLINE_HEIGHT,
        }
    }

    /// The label widgets — rebuilt identically for `children`, `diff`,
    /// `layout` and `draw`; the per-instance state (paragraph caches) lives in
    /// the tree children, which stay diffed against these.
    fn label_widgets(&self) -> Vec<Element<'static, Message, Theme, BaseRenderer>> {
        self.labels
            .iter()
            .map(|label| {
                Text::new(label.clone())
                    .size(13.0)
                    .line_height(1.0)
                    .font(fonts::Inter::MEDIUM)
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .into()
            })
            .collect()
    }

    /// The index of the tab under the cursor, if any.
    ///
    /// Hit regions are contiguous and derived from the label bounds — the
    /// boundary between two tabs is the midpoint between their glyphs, so the
    /// region you hit is the region you see highlighted. The regions span the
    /// row's full height, so the underline bar is part of the tab.
    fn tab_at(&self, state: &State, cursor: Cursor, bounds: Rectangle) -> Option<usize> {
        // `position_in` is already relative to `bounds` — subtracting the
        // origin again shifts the whole hit test by the widget's position
        // (hovering a tab's left half registered the previous tab).
        let position = cursor.position_in(bounds)?;
        let x = position.x;
        let padding = self.h_padding();

        (0..state.text_bounds.len()).find(|&index| {
            let (start, end) = region_span(&state.text_bounds, index, padding);
            x >= start && x < end
        })
    }

    /// Ink color for a tab — active reads on the pill or in the underline's
    /// green, hover brightens.
    fn ink(&self, index: usize, hovered: Option<usize>) -> Color {
        if self.disabled {
            return color::Text::DISABLED;
        }
        if index == self.active {
            match self.style {
                Style::Pill => color::Primary::ON,
                Style::Underline => color::Primary::DEFAULT,
            }
        } else if hovered == Some(index) {
            color::Text::HEADLINE
        } else {
            color::Text::MUTED
        }
    }

    /// Syncs the animated pill/hover targets against the recorded bounds.
    /// Returns whether anything changed. Must run after the bounds exist.
    fn sync_targets(&self, state: &mut State) -> bool {
        if state.text_bounds.is_empty() {
            return false;
        }

        let mut changed = false;

        let active = state.tab_bounds.get(self.active).copied().unwrap_or(state.tab_bounds[0]);
        if state.pill_x.target() != &active.x || state.pill_width.target() != &active.width {
            if state.first_layout {
                // Settle on first layout — no fly-in from the origin.
                state.pill_x.settle_at(active.x);
                state.pill_width.settle_at(active.width);
                state.first_layout = false;
            } else {
                state.pill_x.set_target(active.x);
                state.pill_width.set_target(active.width);
            }
            changed = true;
        }

        if let Some(index) = state.hovered
            && index < state.text_bounds.len()
        {
            let (start, end) = region_span(&state.text_bounds, index, self.h_padding());
            let width = end - start;
            if state.hover_x.target() != &start || state.hover_width.target() != &width {
                state.hover_x.set_target(start);
                state.hover_width.set_target(width);
                changed = true;
            }
        }

        let hover_alpha_target = if state.hovered.is_some() { 1.0 } else { 0.0 };
        if state.hover_alpha.target() != &hover_alpha_target {
            state.hover_alpha.set_target(hover_alpha_target);
            changed = true;
        }

        changed
    }
}

impl<'a, Message: 'static + 'a> From<Tabs<Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(tabs: Tabs<Message>) -> Self {
        Element::new(tabs)
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/tabs/mod.rs"]
mod tests;
