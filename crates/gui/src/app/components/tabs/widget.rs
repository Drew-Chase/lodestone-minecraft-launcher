//! The [`Widget`] implementation for [`Tabs`]: layout, input handling, and
//! drawing. The builder API and hit testing live in the parent module.

use crate::app::components::anim;
use crate::app::theme::color;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{Border, Event, Length, Point, Rectangle, Renderer as BaseRenderer, Size, Theme, mouse, window};
use iced_anim::Animated;
use iced_color_extension::rgb;

use super::{BAR_THICKNESS, SPACING, State, Style, Tabs};

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
        let padding = self.h_padding();
        let height = self.tab_height();
        let state = tree.state.downcast_mut::<State>();

        let mut children = Vec::with_capacity(self.labels.len());
        let mut text_bounds = Vec::with_capacity(self.labels.len());
        let mut tab_bounds = Vec::with_capacity(self.labels.len());
        let mut x = self.inset;
        let mut total_width = 0.0;

        for (index, widget) in widgets.iter_mut().enumerate() {
            // Shrink-resolve the label to its natural size.
            let node = widget.as_widget_mut().layout(&mut tree.children[index], renderer, limits);
            let text_size = node.size();
            let tab_width = text_size.width + padding * 2.0;

            // Center the label inside its tab slot.
            let positioned = node.move_to(Point::new(x + padding, (height - text_size.height) / 2.0));

            text_bounds.push(Rectangle::new(Point::new(x + padding, (height - text_size.height) / 2.0), text_size));
            tab_bounds.push(Rectangle::new(Point::new(x, 0.0), Size::new(tab_width, height)));
            children.push(positioned);
            x += tab_width + SPACING;
        }
        if let Some(last) = tab_bounds.last() {
            total_width = last.x + last.width;
        }

        state.text_bounds = text_bounds;
        state.tab_bounds = tab_bounds;
        let _ = self.sync_targets(state);

        let size = limits.resolve(self.width, self.height, Size::new(total_width, height));
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

                // Redraw frames receive the freshest cursor but dispatch no
                // CursorMoved events, so the hover must be re-derived here —
                // otherwise it trails fast mouse motion by a frame while the
                // row is animating.
                let hovered = if self.interactive() { self.tab_at(state, cursor, bounds) } else { None };
                if state.hovered != hovered {
                    state.hovered = hovered;
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.interactive() && self.tab_at(state, cursor, bounds).is_some() {
                    state.is_pressed = true;
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.is_pressed => {
                state.is_pressed = false;
                if let (Some(index), Some(on_select)) = (self.tab_at(state, cursor, bounds), self.on_select.as_ref())
                    && !self.disabled
                {
                    shell.publish(on_select(index));
                }
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let hovered = if self.interactive() { self.tab_at(state, cursor, bounds) } else { None };
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

        match self.style {
            Style::Pill => {
                let radius = Radius::from(12.0);

                // Sliding hover highlight — white 8% (the guide's ghost-hover level;
                // the spec's 4% is imperceptible on this panel), covering exactly the
                // hit region of the hovered tab (both position and width ease toward
                // it).
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
                        anim::fade(rgb!(255, 255, 255, 8%), hover_alpha),
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
            }
            Style::Underline => {
                // The 1px rule runs the full width on the row's last pixel…
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(Point::new(bounds.x, bounds.y + bounds.height - 1.0), Size::new(bounds.width, 1.0)),
                        ..renderer::Quad::default()
                    },
                    color::Surface::LINE,
                );

                // …and the active bar sits on the two bottom pixels, over the
                // rule, spanning the same slot as the pill would.
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(bounds.x + *state.pill_x.value(), bounds.y + bounds.height - BAR_THICKNESS),
                            Size::new(*state.pill_width.value(), BAR_THICKNESS),
                        ),
                        ..renderer::Quad::default()
                    },
                    color::Primary::DEFAULT,
                );
            }
        }

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
        if self.interactive() && self.tab_at(state, cursor, layout.bounds()).is_some() {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}
