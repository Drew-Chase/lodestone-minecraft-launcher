use crate::app::components::anim;
use iced::advanced::Widget;
use iced::{Point, Size};

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

fn state_with(text_bounds: Vec<Rectangle>) -> State {
    State {
        text_bounds,
        tab_bounds: Vec::new(),
        pill_x: Animated::new(0.0, anim::glide()),
        pill_width: Animated::new(0.0, anim::glide()),
        hovered: None,
        hover_x: Animated::new(0.0, anim::micro()),
        hover_width: Animated::new(0.0, anim::micro()),
        hover_alpha: Animated::new(0.0, anim::micro()),
        is_pressed: false,
        first_layout: true,
    }
}

#[test]
fn ink_follows_active_and_hover() {
    let tabs = tabs();
    assert_eq!(tabs.ink(0, None), color::Primary::ON);
    assert_eq!(tabs.ink(1, None), color::Text::MUTED);
    assert_eq!(tabs.ink(1, Some(1)), color::Text::HEADLINE);
}

#[test]
fn underline_ink_is_green_on_the_active_tab_only() {
    let tabs = Tabs::<()>::new(["Overview", "Mods"]).underline().on_select(|_| ());
    assert_eq!(tabs.ink(0, None), color::Primary::DEFAULT);
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
fn hit_test_is_relative_to_the_widget_origin() {
    let state = state_with(sample_bounds());
    let tabs = tabs();

    // A widget parked at origin (44, 100) — the cursor is given in window
    // coordinates while `position_in` resolves it against these bounds.
    // Regression: subtracting the origin a second time shifted the whole
    // hit test left by 44px (the left half of every tab hit its
    // predecessor, or nothing).
    let bounds = Rectangle::new(Point::new(44.0, 100.0), Size::new(264.0, TAB_HEIGHT));
    let at = |rel_x: f32| Cursor::Available(Point::new(44.0 + rel_x, 100.0 + 16.0));

    // Deep inside the first tab's region...
    assert_eq!(tabs.tab_at(&state, at(34.0), bounds), Some(0));
    // ...its far-left padding edge (one pixel in)...
    assert_eq!(tabs.tab_at(&state, at(1.0), bounds), Some(0));
    // ...and the left half of the second tab, which used to hit the first.
    assert_eq!(tabs.tab_at(&state, at(70.0), bounds), Some(1));

    // Just past the row's right edge: nothing.
    assert_eq!(tabs.tab_at(&state, at(265.0), bounds), None);
}

#[test]
fn underline_slots_include_the_bar_row() {
    // The instance row: labels start at x = 44 (GUTTER 28 + 16) and the
    // second label at 138, as in the reference (slot = label + 32, gap 4).
    let text_bounds = vec![
        Rectangle::new(Point::new(44.0, 10.0), Size::new(58.0, 13.0)),
        Rectangle::new(Point::new(138.0, 10.0), Size::new(30.0, 13.0)),
    ];
    let state = state_with(text_bounds);
    let tabs = Tabs::<()>::new(["Overview", "Mods"]).underline().on_select(|_| ());
    let bounds = Rectangle::new(Point::new(0.0, 0.0), Size::new(400.0, UNDERLINE_HEIGHT));

    // The bar's row is the last pixel row of the box: it still hits the tab.
    let on_bar = |rel_x: f32| Cursor::Available(Point::new(rel_x, UNDERLINE_HEIGHT - 1.0));
    assert_eq!(tabs.tab_at(&state, on_bar(30.0), bounds), Some(0));
    assert_eq!(tabs.tab_at(&state, on_bar(130.0), bounds), Some(1));
}

#[test]
fn regions_are_contiguous_and_contain_their_label() {
    let bounds = sample_bounds();
    let (start, end) = region_span(&bounds, 0, H_PADDING);
    assert!(start <= bounds[0].x && end >= bounds[0].x + bounds[0].width);

    // The boundary between two regions is the midpoint between the
    // previous label's right edge and the next label's left edge.
    let (next_start, _) = region_span(&bounds, 1, H_PADDING);
    let expected = (bounds[0].x + bounds[0].width + bounds[1].x) / 2.0;
    assert_eq!(next_start, expected);

    // Contiguity: each region starts where the previous one ends.
    let (_, first_end) = region_span(&bounds, 0, H_PADDING);
    let (second_start, _) = region_span(&bounds, 1, H_PADDING);
    assert_eq!(first_end, second_start);
}

#[test]
fn outer_regions_cover_the_row_padding() {
    let bounds = sample_bounds();
    let (first_start, _) = region_span(&bounds, 0, H_PADDING);
    assert_eq!(first_start, bounds[0].x - 14.0);

    let last = bounds.len() - 1;
    let (_, last_end) = region_span(&bounds, last, H_PADDING);
    let last_text = &bounds[last];
    assert_eq!(last_end, last_text.x + last_text.width + 14.0);
}

#[test]
fn underline_padding_is_sixteen_each_side() {
    // Label at 44 with 16px padding: the first slot starts at the 28px
    // inset, and the midpoint with the second label (138) is at 120.
    let bounds = vec![
        Rectangle::new(Point::new(44.0, 10.0), Size::new(58.0, 13.0)),
        Rectangle::new(Point::new(138.0, 10.0), Size::new(30.0, 13.0)),
    ];
    assert_eq!(region_span(&bounds, 0, UNDERLINE_H_PADDING), (28.0, 120.0));
}
