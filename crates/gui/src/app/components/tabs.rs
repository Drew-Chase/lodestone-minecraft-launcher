//! Tabs — the style guide's `.tab` pill row (§07).
//!
//! Inactive tabs are quiet (`--ink-2` on transparent); the active tab is a
//! Lode Green pill with near-black ink. The pill *glides* between tabs on a
//! 300ms ease rather than teleporting, and a faint white highlight slides
//! under the hovered tab — the guide's lift-family motion (§08).
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

use crate::app::theme::{color, fonts};
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::text;
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::widget::Text;
use iced::{Border, Color, Element, Event, Length, Point, Rectangle, Renderer as BaseRenderer, Size, Theme, alignment, mouse, window};
use iced_anim::Animated;
use iced_color_extension::rgb;

use super::anim;

/// Horizontal padding inside one tab — `.tab` is `8px 14px`.
const H_PADDING: f32 = 14.0;
/// Gap between adjacent tabs.
const SPACING: f32 = 4.0;
/// The `.tab` line height — 13px text plus the 8px vertical padding.
const TAB_HEIGHT: f32 = 34.0;

/// An animated tab row (`.tab`).
pub struct Tabs<Message> {
    labels: Vec<String>,
    active: usize,
    on_select: Option<Box<dyn Fn(usize) -> Message>>,
    width: Length,
    height: Length,
    disabled: bool,
    /// Draws the hit/hover regions — a diagnostic aid, off by default.
    debug_hits: bool,
}

#[derive(Debug)]
struct State {
    /// Label bounds relative to the row, recorded during layout. These are
    /// where the glyphs actually are — the source of truth for hover and
    /// hit testing.
    text_bounds: Vec<Rectangle>,
    /// Pill slot bounds relative to the row (label + horizontal padding).
    tab_bounds: Vec<Rectangle>,
    /// Animated active-pill geometry, relative to the row.
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
fn region_span(text_bounds: &[Rectangle], index: usize) -> (f32, f32) {
    let text = &text_bounds[index];

    let start = if index == 0 {
        text.x - H_PADDING
    } else {
        let previous = &text_bounds[index - 1];
        (previous.x + previous.width + text.x) / 2.0
    };

    let end = if index + 1 == text_bounds.len() {
        text.x + text.width + H_PADDING
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
            width: Length::Shrink,
            height: Length::Fixed(TAB_HEIGHT),
            disabled: false,
            debug_hits: false,
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

    /// Draws the hover/hit regions — a diagnostic aid for verifying alignment.
    pub fn debug_hits(mut self, debug_hits: bool) -> Self {
        self.debug_hits = debug_hits;
        self
    }

    /// Whether the tabs can be interacted with at all.
    fn interactive(&self) -> bool {
        !self.disabled && self.on_select.is_some()
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
    /// region you hit is the region you see highlighted.
    fn tab_at(state: &State, cursor: Cursor, bounds: Rectangle) -> Option<usize> {
        let position = cursor.position_in(bounds)?;
        let x = position.x - bounds.x;

        (0..state.text_bounds.len()).find(|&index| {
            let (start, end) = region_span(&state.text_bounds, index);
            x >= start && x < end
        })
    }

    /// Ink color for a tab — active reads on the green pill, hover brightens.
    fn ink(&self, index: usize, hovered: Option<usize>) -> Color {
        if self.disabled {
            return color::Text::DISABLED;
        }
        if index == self.active {
            color::Primary::ON
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
            let (start, end) = region_span(&state.text_bounds, index);
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

impl<Message: 'static> Widget<Message, Theme, BaseRenderer> for Tabs<Message> {
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            text_bounds: Vec::new(),
            tab_bounds: Vec::new(),
            pill_x: Animated::new(0.0, anim::glide()),
            pill_width: Animated::new(0.0, anim::glide()),
            hovered: None,
            hover_x: Animated::new(0.0, anim::micro()),
            hover_width: Animated::new(0.0, anim::micro()),
            hover_alpha: Animated::new(0.0, anim::micro()),
            is_pressed: false,
            first_layout: true,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.label_widgets().into_iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let widgets = self.label_widgets();
        tree.diff_children(&widgets);
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &BaseRenderer, limits: &Limits) -> Node {
        let mut widgets = self.label_widgets();
        let state = tree.state.downcast_mut::<State>();

        let mut children = Vec::with_capacity(self.labels.len());
        let mut text_bounds = Vec::with_capacity(self.labels.len());
        let mut tab_bounds = Vec::with_capacity(self.labels.len());
        let mut x = 0.0;
        let mut total_width = 0.0;

        for (index, widget) in widgets.iter_mut().enumerate() {
            // Shrink-resolve the label to its natural size.
            let node = widget.as_widget_mut().layout(&mut tree.children[index], renderer, limits);
            let text_size = node.size();
            let tab_width = text_size.width + H_PADDING * 2.0;

            // Center the label inside its tab slot.
            let positioned = node.move_to(Point::new(x + H_PADDING, (TAB_HEIGHT - text_size.height) / 2.0));

            text_bounds.push(Rectangle::new(
                Point::new(x + H_PADDING, (TAB_HEIGHT - text_size.height) / 2.0),
                text_size,
            ));
            tab_bounds.push(Rectangle::new(Point::new(x, 0.0), Size::new(tab_width, TAB_HEIGHT)));
            children.push(positioned);
            x += tab_width + SPACING;
        }
        if let Some(last) = tab_bounds.last() {
            total_width = last.x + last.width;
        }

        state.text_bounds = text_bounds;
        state.tab_bounds = tab_bounds;
        let _ = self.sync_targets(state);

        let size = limits.resolve(self.width, self.height, Size::new(total_width, TAB_HEIGHT));
        Node::with_children(size, children)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &BaseRenderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                state.pill_x.tick(*now);
                state.pill_width.tick(*now);
                state.hover_x.tick(*now);
                state.hover_width.tick(*now);
                state.hover_alpha.tick(*now);
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.interactive() && Self::tab_at(state, cursor, bounds).is_some() {
                    state.is_pressed = true;
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.is_pressed => {
                state.is_pressed = false;
                if let (Some(index), Some(on_select)) = (Self::tab_at(state, cursor, bounds), self.on_select.as_ref())
                    && !self.disabled
                {
                    shell.publish(on_select(index));
                }
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let hovered = if self.interactive() { Self::tab_at(state, cursor, bounds) } else { None };
                if state.hovered != hovered {
                    state.hovered = hovered;
                }
            }
            _ => {}
        }

        let mut needs_redraw = self.sync_targets(state);
        needs_redraw |= state.pill_x.is_animating();
        needs_redraw |= state.pill_width.is_animating();
        needs_redraw |= state.hover_x.is_animating();
        needs_redraw |= state.hover_width.is_animating();
        needs_redraw |= state.hover_alpha.is_animating();

        if needs_redraw {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut BaseRenderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let widgets = self.label_widgets();
        let bounds = layout.bounds();
        let radius = Radius::from(12.0);

        // Sliding hover highlight — white 4%, covering exactly the hit region
        // of the hovered tab (both position and width ease toward it).
        let hover_alpha = *state.hover_alpha.value();
        if hover_alpha > 0.01
            && let Some(hovered) = state.hovered
            && hovered < state.text_bounds.len()
        {
            let hover_bounds = Rectangle::new(
                Point::new(bounds.x + *state.hover_x.value(), bounds.y),
                Size::new(*state.hover_width.value(), bounds.height),
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: hover_bounds,
                    border: Border { radius, ..Border::default() },
                    ..renderer::Quad::default()
                },
                anim::fade(rgb!(255, 255, 255, 4%), hover_alpha),
            );
        }

        // Active pill — Lode Green, glowing (§07 `.tab.active`).
        let pill_bounds = Rectangle::new(
            Point::new(bounds.x + *state.pill_x.value(), bounds.y),
            Size::new(*state.pill_width.value(), bounds.height),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: pill_bounds,
                border: Border { radius, ..Border::default() },
                shadow: iced::Shadow {
                    color: rgb!(34, 255, 132, 35%),
                    offset: iced::Vector::new(0.0, 8.0),
                    blur_radius: 20.0,
                },
                ..renderer::Quad::default()
            },
            color::Primary::DEFAULT,
        );

        // Labels — real Text children, so measurement/anchoring is iced's job.
        let mut children = layout.children();
        for (index, widget) in widgets.iter().enumerate() {
            let Some(node) = children.next() else {
                break;
            };
            if node.bounds().intersection(viewport).is_none() {
                continue;
            }
            widget.as_widget().draw(
                &tree.children[index],
                renderer,
                theme,
                // The labels carry no explicit color; the ink flows in here so
                // the active/hovered states restyle without a view rebuild.
                &renderer::Style {
                    text_color: self.ink(index, state.hovered),
                },
                node,
                cursor,
                viewport,
            );
        }

        // Diagnostic outlines — the hover/hit regions in red, the label bounds
        // in cyan. Gated behind the builder flag.
        if self.debug_hits {
            for index in 0..state.text_bounds.len() {
                let (start, end) = region_span(&state.text_bounds, index);
                let region = Rectangle::new(Point::new(bounds.x + start, bounds.y), Size::new(end - start, bounds.height));
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: region,
                        border: Border {
                            radius,
                            width: 1.0,
                            color: iced::color!(0xFF3B30),
                        },
                        ..renderer::Quad::default()
                    },
                    Color::TRANSPARENT,
                );
            }
            for text in &state.text_bounds {
                let node_bounds = Rectangle::new(Point::new(bounds.x + text.x, bounds.y + text.y), text.size());
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: node_bounds,
                        border: Border {
                            radius: Radius::from(2.0),
                            width: 1.0,
                            color: iced::color!(0x30D5C8),
                        },
                        ..renderer::Quad::default()
                    },
                    Color::TRANSPARENT,
                );
            }

            // The exact values the hit test sees: the cursor x in the widget
            // frame (same call as `tab_at`), the hovered index, the widget
            // origin, and every region span — rendered above the row so a
            // screenshot pins down any coordinate mismatch exactly.
            let cursor_x = cursor.position_in(bounds).map(|p| (p.x - bounds.x) as i32);
            let regions = state
                .text_bounds
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let (start, end) = region_span(&state.text_bounds, i);
                    format!("[{start:.0}..{end:.0}]")
                })
                .collect::<String>();
            let summary = format!(
                "cursor={:?} hover={:?} origin={:.0} {regions}",
                cursor_x,
                state.hovered.map(|i| i as i32),
                bounds.x,
            );
            renderer.fill_text(
                text::Text {
                    content: summary,
                    bounds: Size::new(f32::INFINITY, 14.0),
                    size: iced::Pixels::from(11.0),
                    line_height: text::LineHeight::Relative(1.0),
                    font: fonts::JetBrainsMono::REGULAR,
                    align_x: text::Alignment::Default,
                    align_y: alignment::Vertical::Top,
                    shaping: text::Shaping::Basic,
                    wrapping: text::Wrapping::default(),
                },
                Point::new(bounds.x, bounds.y - 18.0),
                iced::color!(0xFF3B30),
                *viewport,
            );
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        let state = tree.state.downcast_ref::<State>();
        if self.interactive() && Self::tab_at(state, cursor, layout.bounds()).is_some() {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}

impl<'a, Message: 'static + 'a> From<Tabs<Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(tabs: Tabs<Message>) -> Self {
        Element::new(tabs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tabs() -> Tabs<()> {
        Tabs::new(["Mods", "Resource packs", "Worlds"]).on_select(|_| ())
    }

    fn sample_bounds() -> Vec<Rectangle> {
        // Labels at 13px Inter, laid out as the widget would: each starts
        // 14px after its slot, slots separated by a 4px gap.
        vec![
            Rectangle::new(Point::new(14.0, 10.0), Size::new(33.0, 13.0)),
            Rectangle::new(Point::new(79.0, 10.0), Size::new(95.0, 13.0)),
            Rectangle::new(Point::new(206.0, 10.0), Size::new(44.0, 13.0)),
        ]
    }

    #[test]
    fn ink_follows_active_and_hover() {
        let tabs = tabs();
        assert_eq!(tabs.ink(0, None), color::Primary::ON);
        assert_eq!(tabs.ink(1, None), color::Text::MUTED);
        assert_eq!(tabs.ink(1, Some(1)), color::Text::HEADLINE);
    }

    #[test]
    fn disabled_overrides_everything() {
        let tabs = tabs().disabled(true);
        for index in 0..3 {
            assert_eq!(tabs.ink(index, Some(index)), color::Text::DISABLED);
        }
    }

    #[test]
    fn without_on_select_is_not_interactive() {
        let no_callback: Tabs<()> = Tabs::new(["A", "B"]);
        assert!(!no_callback.interactive());
        assert!(tabs().interactive());
    }

    #[test]
    fn one_child_per_label() {
        let tabs = tabs();
        assert_eq!(tabs.label_widgets().len(), 3);
        assert_eq!(tabs.children().len(), 3);
    }

    #[test]
    fn regions_are_contiguous_and_contain_their_label() {
        let bounds = sample_bounds();
        let (start, end) = region_span(&bounds, 0);
        assert!(start <= bounds[0].x && end >= bounds[0].x + bounds[0].width);

        // The boundary between two regions is the midpoint between the
        // previous label's right edge and the next label's left edge.
        let (next_start, _) = region_span(&bounds, 1);
        let expected = (bounds[0].x + bounds[0].width + bounds[1].x) / 2.0;
        assert_eq!(next_start, expected);

        // Contiguity: each region starts where the previous one ends.
        let (_, first_end) = region_span(&bounds, 0);
        let (second_start, _) = region_span(&bounds, 1);
        assert_eq!(first_end, second_start);
    }

    #[test]
    fn outer_regions_cover_the_row_padding() {
        let bounds = sample_bounds();
        let (first_start, _) = region_span(&bounds, 0);
        assert_eq!(first_start, bounds[0].x - 14.0);

        let last = bounds.len() - 1;
        let (_, last_end) = region_span(&bounds, last);
        let last_text = &bounds[last];
        assert_eq!(last_end, last_text.x + last_text.width + 14.0);
    }
}
