//! Popover — a trigger that opens a floating panel beside it. The panel holds any content, so its rows
//! can carry buttons. The caller owns the open state: the trigger asks for a toggle, a press outside
//! the panel or Escape asks for a dismissal, and the panel shows exactly when the caller says it is open.
//!
//! ```text
//! Popover::new(indicator, rows)
//!     .open(self.panel_open)
//!     .on_toggle(Message::TogglePanel)
//!     .on_dismiss(Message::DismissPanel)
//! ```

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, overlay};
use iced::{Element, Event, Length, Rectangle, Renderer as BaseRenderer, Size, Theme, Vector, keyboard, mouse};

/// The gap between the trigger and the panel, in pixels.
const GAP: f32 = 6.0;
/// The least room the panel keeps from the window's edges, in pixels.
const MARGIN: f32 = 8.0;

/// A trigger that opens a panel of content beside it while the caller keeps it open.
pub struct Popover<'a, Message> {
    trigger: Element<'a, Message>,
    content: Element<'a, Message>,
    open: bool,
    on_toggle: Option<Message>,
    on_dismiss: Option<Message>,
}

impl<'a, Message: Clone> Popover<'a, Message> {
    /// A popover whose panel is `content`, opened from `trigger`. It starts shut.
    pub fn new(trigger: impl Into<Element<'a, Message>>, content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            trigger: trigger.into(),
            content: content.into(),
            open: false,
            on_toggle: None,
            on_dismiss: None,
        }
    }

    /// Whether the panel is shown.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The message the trigger sends when it is pressed. Without it the trigger does nothing.
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.on_toggle = Some(message);
        self
    }

    /// The message sent when a press lands outside both the panel and the trigger, or Escape is
    /// pressed while the panel is open.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

impl<Message: Clone + 'static> Widget<Message, Theme, BaseRenderer> for Popover<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.trigger.as_widget().size()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.trigger), Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.trigger, &self.content]);
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &BaseRenderer, limits: &layout::Limits) -> layout::Node {
        self.trigger.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &BaseRenderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.trigger
            .as_widget_mut()
            .update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        if shell.is_event_captured() {
            return;
        }
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && cursor.is_over(layout.bounds())
            && let Some(message) = &self.on_toggle
        {
            shell.publish(message.clone());
            shell.capture_event();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut BaseRenderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.trigger
            .as_widget()
            .draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &BaseRenderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) && self.on_toggle.is_some() {
            return mouse::Interaction::Pointer;
        }
        self.trigger
            .as_widget()
            .mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &BaseRenderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, BaseRenderer>> {
        if !self.open {
            return None;
        }
        Some(overlay::Element::new(Box::new(PanelOverlay {
            content: &mut self.content,
            tree: &mut tree.children[1],
            trigger: layout.bounds() + translation,
            on_dismiss: self.on_dismiss.clone(),
        })))
    }
}

impl<'a, Message: Clone + 'static> From<Popover<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(popover: Popover<'a, Message>) -> Self {
        Element::new(popover)
    }
}

/// The panel an open popover shows: its content under the trigger, or above it when the window has
/// no room below.
struct PanelOverlay<'a, 'b, Message> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    /// The trigger's bounds in window coordinates.
    trigger: Rectangle,
    on_dismiss: Option<Message>,
}

impl<Message: Clone + 'static> overlay::Overlay<Message, Theme, BaseRenderer> for PanelOverlay<'_, '_, Message> {
    fn layout(&mut self, renderer: &BaseRenderer, bounds: Size) -> layout::Node {
        let window = Rectangle::with_size(bounds);
        let limits = layout::Limits::new(Size::ZERO, Size::new((bounds.width - 2.0 * MARGIN).max(0.0), bounds.height));
        let panel = self.content.as_widget_mut().layout(self.tree, renderer, &limits);
        let size = panel.size();

        let below = self.trigger.y + self.trigger.height + GAP;
        let y = if below + size.height <= window.height - MARGIN {
            below
        } else {
            (self.trigger.y - GAP - size.height).max(MARGIN)
        };
        let x = self.trigger.x.min(window.width - MARGIN - size.width).max(MARGIN);
        layout::Node::with_children(size, vec![panel]).translate(Vector::new(x, y))
    }

    fn draw(&self, renderer: &mut BaseRenderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor) {
        if let Some(panel) = layout.children().next() {
            self.content
                .as_widget()
                .draw(self.tree, renderer, theme, style, panel, cursor, &Rectangle::with_size(Size::INFINITE));
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &BaseRenderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(panel) = layout.children().next() else {
            return;
        };
        self.content.as_widget_mut().update(
            self.tree,
            event,
            panel,
            cursor,
            renderer,
            clipboard,
            shell,
            &Rectangle::with_size(Size::INFINITE),
        );
        if shell.is_event_captured() {
            return;
        }
        // A press inside the panel never reaches the page beneath it.
        if let Event::Mouse(mouse::Event::ButtonPressed(_)) = event
            && cursor.is_over(panel.bounds())
        {
            shell.capture_event();
            return;
        }
        let Some(message) = &self.on_dismiss else {
            return;
        };
        match event {
            // A press on the trigger is left to the trigger, which toggles the panel itself.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if !cursor.is_over(panel.bounds()) && !cursor.is_over(self.trigger) => {
                shell.publish(message.clone());
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                shell.publish(message.clone());
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(&self, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &BaseRenderer) -> mouse::Interaction {
        match layout.children().next() {
            Some(panel) => self
                .content
                .as_widget()
                .mouse_interaction(self.tree, panel, cursor, &Rectangle::with_size(Size::INFINITE), renderer),
            None => mouse::Interaction::None,
        }
    }

    /// The content's own overlays, such as the hint of a button in the panel, open above the panel.
    fn overlay<'c>(&'c mut self, layout: Layout<'c>, renderer: &BaseRenderer) -> Option<overlay::Element<'c, Message, Theme, BaseRenderer>> {
        let panel = layout.children().next()?;
        self.content
            .as_widget_mut()
            .overlay(self.tree, panel, renderer, &Rectangle::with_size(Size::INFINITE), Vector::ZERO)
    }
}
