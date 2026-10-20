//! Tabs — the style guide's `.tab` pill row (§07).
//!
//! Inactive tabs are quiet (`--ink-2` on transparent); the active tab is a
//! Lode Green pill with near-black ink. The pill *glides* between tabs on a
//! 300ms ease rather than teleporting, and a faint white highlight slides
//! under the hovered tab — the guide's lift-family motion (§08).
//!
//! Labels are real [`Text`] children: iced measures and anchors them, so the
//! pill geometry comes straight from the child layout nodes and can never
//! drift from the glyphs.
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
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::widget::Text;
use iced::{Border, Color, Element, Event, Length, Point, Rectangle, Renderer as BaseRenderer, Size, Theme, mouse, window};
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
}

#[derive(Debug)]
struct State {
    /// Tab slot bounds relative to the row, recorded during layout.
    tab_bounds: Vec<Rectangle>,
    /// Animated active-pill geometry, relative to the row.
    pill_x: Animated<f32>,
    pill_width: Animated<f32>,
    /// Hovered tab, if any.
    hovered: Option<usize>,
    hover_x: Animated<f32>,
    hover_alpha: Animated<f32>,
    is_pressed: bool,
    /// True until the first layout, so the pill settles instead of flying in.
    first_layout: bool,
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
    /// The index of the tab under the cursor, if any.
    ///
    /// Hit regions are contiguous: each tab owns the row from its slot's
    /// left edge to the next slot's left edge, so the padding between the
    /// labels never dead-zones and every boundary lines up with a pill edge.
    fn tab_at(state: &State, cursor: Cursor, bounds: Rectangle) -> Option<usize> {
        let position = cursor.position_in(bounds)?;
        let x = position.x - bounds.x;

        // The last slot whose left edge is at or before the cursor wins; the
        // widget's own width ends at the final slot's trailing padding, so
        // nothing past the last tab can match.
        let mut index = None;
        for (i, tab) in state.tab_bounds.iter().enumerate() {
            if x >= tab.x {
                index = Some(i);
            } else {
                break;
            }
        }

        index
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
    /// Returns whether anything changed. Must run after `tab_bounds` exists.
    fn sync_targets(&self, state: &mut State) -> bool {
        if state.tab_bounds.is_empty() {
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

        let hover_target_x = state
            .hovered
            .and_then(|i| state.tab_bounds.get(i))
            .map(|tab| tab.x)
            .unwrap_or_else(|| *state.hover_x.target());
        let hover_alpha_target = if state.hovered.is_some() { 1.0 } else { 0.0 };
        if state.hover_x.target() != &hover_target_x {
            state.hover_x.set_target(hover_target_x);
            changed = true;
        }
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
            tab_bounds: Vec::new(),
            pill_x: Animated::new(0.0, anim::glide()),
            pill_width: Animated::new(0.0, anim::glide()),
            hovered: None,
            hover_x: Animated::new(0.0, anim::micro()),
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

            tab_bounds.push(Rectangle::new(Point::new(x, 0.0), Size::new(tab_width, TAB_HEIGHT)));
            children.push(positioned);
            x += tab_width + SPACING;
        }
        if let Some(last) = tab_bounds.last() {
            total_width = last.x + last.width;
        }

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

        // Sliding hover highlight — white 4% under the hovered tab.
        let hover_alpha = *state.hover_alpha.value();
        if hover_alpha > 0.01
            && let Some(hovered) = state.hovered
            && let Some(tab) = state.tab_bounds.get(hovered)
        {
            let hover_bounds = Rectangle::new(
                Point::new(bounds.x + *state.hover_x.value(), bounds.y),
                Size::new(tab.width, bounds.height),
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
}
