//! The Library — the launcher's landing page: the most-recent instance
//! hero, the quick actions, and the instance list over a virtualized
//! scroll region (Grid / Compact / Table views, filter tabs, search).

pub mod instance;

use instance::{Instance, InstanceState, Loader};

use crate::app::components::instance_card::{CARD_HEIGHT, COMPACT_ROW_HEIGHT, TABLE_ROW_HEIGHT, compact_row, instance_card, table_header, table_row};
use crate::app::components::scene_art::scene_handle;
use crate::app::components::{IconButton, PulseDot, TextButton};
use crate::app::theme::{color, fonts};
use aurora_leaf::Icon;
use chrono::{DateTime, Local};
use iced::widget::{Space, Text, column, container, responsive, row, scrollable, stack, svg, text_input};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, alignment};
use iced_color_extension::rgb;
use tracing::info;

/// Messages the Library page handles.
#[derive(Debug, Clone)]
pub enum Message {
    /// The search query changed — filters the instance list by name.
    SearchChanged(String),
    /// A filter tab was selected.
    FilterChanged(LibraryFilter),
    /// The view mode changed (Grid / Compact / Table).
    ViewChanged(LibraryView),
    /// The scroll region moved — drives the virtualized windowing.
    Scrolled(f32),
    /// Play was pressed on an instance (dummy until the game launcher).
    Play(String),
    /// A per-instance menu button (folder/settings/kebab) — dummy.
    InstanceMenu { instance: String, action: &'static str },
    /// A header/quick action (New Instance, Import, ...) — dummy.
    QuickAction(&'static str),
    /// The refresh button — the root re-queries the database.
    Reload,
}

/// The Library's view modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryView {
    /// Full cards with scene art — the reference layout.
    #[default]
    Grid,
    /// The grid card's info without the art, one tight row.
    Compact,
    /// A traditional table.
    Table,
}

impl LibraryView {
    /// The virtualized row pitch (px): one item's height plus the gap.
    fn row_pitch(self) -> f32 {
        Self::item_height(self) + Self::gap(self)
    }

    /// One item's fixed height in this view.
    fn item_height(self) -> f32 {
        match self {
            LibraryView::Grid => CARD_HEIGHT,
            LibraryView::Compact => COMPACT_ROW_HEIGHT,
            LibraryView::Table => TABLE_ROW_HEIGHT,
        }
    }

    /// The gap between rows in this view.
    fn gap(self) -> f32 {
        match self {
            LibraryView::Grid | LibraryView::Compact => 24.0,
            LibraryView::Table => 0.0,
        }
    }

    /// How many columns fit in `width` — the grid tiles at a 340px minimum
    /// card width, capped at 4 so wide windows keep the reference's large
    /// cards; the other views are single-column lists.
    fn columns(self, width: f32) -> usize {
        match self {
            LibraryView::Grid => ((width / 340.0) as usize).clamp(1, 4),
            LibraryView::Compact | LibraryView::Table => 1,
        }
    }

    fn label(self) -> &'static str {
        match self {
            LibraryView::Grid => "Grid",
            LibraryView::Compact => "Compact",
            LibraryView::Table => "Table",
        }
    }
}

/// The instance-list filter tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryFilter {
    /// Every instance, most recent first.
    #[default]
    Recent,
    /// Only pinned instances.
    Pinned,
    /// Modded loaders only.
    Modded,
    /// Vanilla only.
    Vanilla,
}

impl LibraryFilter {
    fn label(self) -> &'static str {
        match self {
            LibraryFilter::Recent => "Recent",
            LibraryFilter::Pinned => "Pinned",
            LibraryFilter::Modded => "Modded",
            LibraryFilter::Vanilla => "Vanilla",
        }
    }

    fn matches(self, instance: &Instance) -> bool {
        match self {
            LibraryFilter::Recent => true,
            LibraryFilter::Pinned => instance.pinned,
            LibraryFilter::Modded => instance.loader != Loader::Vanilla,
            LibraryFilter::Vanilla => instance.loader == Loader::Vanilla,
        }
    }
}

/// The Library page state.
#[derive(Debug, Default)]
pub struct Library {
    /// Every instance, loaded from the database.
    instances: Vec<Instance>,
    /// The search query — filters the list by name.
    search: String,
    /// The selected filter tab.
    filter: LibraryFilter,
    /// The selected view mode.
    view: LibraryView,
    /// The scroll region's offset — drives the virtualized windowing.
    scroll_y: f32,
}

impl Library {
    /// Applies a display-state change. Database effects (`Reload`) are
    /// intercepted by the root before reaching here.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SearchChanged(search) => {
                self.search = search;
                self.scroll_y = 0.0;
            }
            Message::FilterChanged(filter) => {
                self.filter = filter;
                self.scroll_y = 0.0;
            }
            Message::ViewChanged(view) => {
                self.view = view;
                self.scroll_y = 0.0;
            }
            Message::Scrolled(offset) => self.scroll_y = offset.max(0.0),
            Message::Play(name) => info!("Play requested for {name} (dummy)"),
            Message::InstanceMenu { instance, action } => info!("Instance '{instance}' menu: {action} (dummy)"),
            Message::QuickAction(action) => info!("Quick action '{action}' (dummy)"),
            Message::Reload => {} // handled by the root
        }
    }

    /// Replaces the loaded instances (the root re-queried the database).
    pub fn set_instances(&mut self, instances: Vec<Instance>) {
        self.instances = instances;
        self.scroll_y = 0.0;
    }

    /// Overrides the initial view mode (the `LODESTONE_LIBRARY_VIEW` probe).
    pub fn set_view(&mut self, view: LibraryView) {
        self.view = view;
    }

    /// The instances after the search query and filter tab, most recent
    /// first.
    fn visible(&self) -> Vec<&Instance> {
        let query = self.search.to_lowercase();
        let mut visible: Vec<&Instance> = self
            .instances
            .iter()
            .filter(|instance| self.filter.matches(instance))
            .filter(|instance| query.is_empty() || instance.name.to_lowercase().contains(&query))
            .collect();
        visible.sort_by_key(|instance| std::cmp::Reverse(instance.recency()));
        visible
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![self.header(), self.hero(), self.quick_actions(), self.instances_section(),]
            .spacing(24)
            .padding(Padding::new(0.0).left(28).right(28).top(24).bottom(28))
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// The page header — title, stats, search and actions.
    fn header(&self) -> Element<'_, Message> {
        let total_hours: u32 = self.instances.iter().map(|instance| instance.playtime_seconds / 3600).sum();
        row![
            column![
                Text::new("Library").size(24).font(fonts::Inter::EXTRA_BOLD).color(color::Text::HEADLINE),
                Text::new(format!(
                    "{} instance{} · {}h total playtime",
                    self.instances.len(),
                    if self.instances.len() == 1 { "" } else { "s" },
                    total_hours
                ))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
            ]
            .spacing(2),
            Space::new().width(Fill),
            self.search_box(),
            IconButton::new(Icon::lucide().refresh_cw())
                .width(44)
                .height(44)
                .icon_size(16.0)
                .on_press(Message::Reload),
            IconButton::new(Icon::lucide().bell())
                .width(44)
                .height(44)
                .icon_size(16.0)
                .on_press(Message::QuickAction("notifications")),
            TextButton::content(
                row![
                    icon(Icon::lucide().plus(), 14.0, color::Primary::ON, 1.0),
                    Text::new("New Instance").size(13).font(fonts::Inter::BOLD).color(color::Primary::ON),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center)
            )
            .variant(crate::app::components::button::Variant::Primary)
            .padding(Padding::new(0.0).left(18).right(18).top(12).bottom(12))
            .on_press(Message::QuickAction("New Instance")),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill)
        .into()
    }

    /// The search input with its leading icon.
    fn search_box(&self) -> Element<'_, Message> {
        stack![
            text_input("Search instances...", &self.search)
                .on_input(Message::SearchChanged)
                .size(13)
                .padding(Padding::new(0.0).left(40).right(12).top(11).bottom(11))
                .width(Length::Fixed(260.0)),
            container(icon(Icon::lucide().search(), 15.0, color::Text::PLACEHOLDER, 1.0))
                .width(Length::Fixed(260.0))
                .height(Fill)
                .align_x(alignment::Horizontal::Left)
                .align_y(alignment::Vertical::Center)
                .padding(Padding::new(0.0).left(14)),
        ]
        .width(260)
        .into()
    }

    /// The most-recent instance's wide banner card.
    fn hero(&self) -> Element<'_, Message> {
        let Some(hero) = self.instances.iter().max_by_key(|instance| instance.recency().timestamp_millis()) else {
            return Space::new().height(0).into();
        };
        let playing = hero.state == InstanceState::Playing;

        let chips = row![
            status_chip(hero.state),
            chip(
                match &hero.loader_version {
                    Some(version) => format!("{} {version}", hero.loader.label()),
                    None => hero.loader.label().to_string(),
                },
                match hero.loader {
                    Loader::Vanilla => color::Primary::DEFAULT,
                    Loader::Fabric => color::Accents::VIOLET_LIGHT,
                    Loader::Forge => color::Accents::AMBER,
                    Loader::Quilt => color::Accents::CYAN,
                    Loader::NeoForge => color::Accents::PINK,
                },
                match hero.loader {
                    Loader::Vanilla => color::Primary::TINT,
                    Loader::Fabric => color::Accents::VIOLET_TINT,
                    Loader::Forge => color::Accents::AMBER_TINT,
                    Loader::Quilt => color::Accents::CYAN_TINT,
                    Loader::NeoForge => color::Accents::PINK_TINT,
                },
            ),
            chip(format!("MC {}", hero.mc_version), color::Text::MUTED, rgb!(255, 255, 255, 6%)),
        ]
        .spacing(8);

        let mut meta = format!("{} mods", hero.mod_count);
        if let Some(played) = hero.last_played {
            meta.push_str(&format!(" · last played {}", relative_time(played)));
        } else {
            meta.push_str(" · never played");
        }

        stack![
            // The biome scene fills the banner behind the content.
            container(
                svg(scene_handle(hero.biome, hero.art_seed))
                    .content_fit(iced::ContentFit::Cover)
                    .width(Fill)
                    .height(Fill)
            )
            .width(Fill)
            .height(Fill),
            container(
                column![
                    chips,
                    Text::new(hero.name.clone())
                        .size(34)
                        .font(fonts::Inter::EXTRA_BOLD)
                        .color(color::Text::HEADLINE),
                    Text::new(meta).size(13).font(fonts::Inter::REGULAR).color(color::Text::DEFAULT),
                    row![
                        TextButton::content(
                            row![
                                icon(Icon::lucide().play(), 14.0, color::Primary::ON, 1.0),
                                Text::new(if playing { "Resume Session" } else { "Play" })
                                    .size(13)
                                    .font(fonts::Inter::BOLD)
                                    .color(color::Primary::ON),
                            ]
                            .spacing(8)
                            .align_y(alignment::Vertical::Center)
                        )
                        .variant(crate::app::components::button::Variant::Primary)
                        .padding(Padding::new(0.0).left(22).right(22).top(14).bottom(14))
                        .on_press(Message::Play(hero.name.clone())),
                        hero_menu_button(&hero.name, "folder", Icon::lucide().folder()),
                        hero_menu_button(&hero.name, "settings", Icon::lucide().settings()),
                        hero_menu_button(&hero.name, "more", Icon::lucide().ellipsis_vertical()),
                    ]
                    .spacing(10)
                    .align_y(alignment::Vertical::Center),
                ]
                .spacing(14)
                .padding(Padding::new(0.0).left(28).right(28).bottom(28))
            )
            .width(Fill)
            .height(Fill)
            .align_y(alignment::Vertical::Center),
            // The live session readout sits top-right while playing.
            container(
                container(chip("2.1 GB / 6 GB · 58 FPS".into(), color::Text::DEFAULT, rgb!(0, 0, 0, 45%)))
                    .padding(Padding::new(0.0).right(16).top(16))
            )
            .width(Fill)
            .height(Fill)
            .align_x(alignment::Horizontal::Right)
            .align_y(alignment::Vertical::Top),
        ]
        .width(Fill)
        .height(300)
        .into()
    }

    /// The four quick-action cards — dummy entries until the features land.
    fn quick_actions(&self) -> Element<'_, Message> {
        let cards: [(&'static str, Color, Color, &'static str, &'static str); 4] = [
            (
                Icon::lucide().plus(),
                color::Primary::DEFAULT,
                color::Primary::TINT,
                "New Instance",
                "From modpack, CurseForge, or scratch",
            ),
            (
                Icon::lucide().download(),
                color::Accents::CYAN,
                color::Accents::CYAN_TINT,
                "Import",
                ".zip, .mrpack, CurseForge",
            ),
            (
                Icon::lucide().server(),
                color::Accents::AMBER,
                color::Accents::AMBER_TINT,
                "Join Server",
                "Realms · hosted · friends",
            ),
            (
                Icon::lucide().users(),
                color::Accents::VIOLET_LIGHT,
                color::Accents::VIOLET_TINT,
                "Co-op Sync",
                "3 friends online now",
            ),
        ];
        row(cards
            .into_iter()
            .map(|(icon_markup, ink, fill, title, subtitle)| quick_action_card(icon_markup, ink, fill, title, subtitle, Message::QuickAction(title))))
        .spacing(20)
        .width(Fill)
        .into()
    }

    /// The "Your Instances" section — title, filter tabs, view switcher,
    /// and the virtualized list.
    fn instances_section(&self) -> Element<'_, Message> {
        column![
            row![
                Text::new("Your Instances")
                    .size(18)
                    .font(fonts::Inter::EXTRA_BOLD)
                    .color(color::Text::HEADLINE),
                Space::new().width(Fill),
                self.filter_tabs(),
                Space::new().width(12),
                self.view_switcher(),
            ]
            .align_y(alignment::Vertical::Center)
            .width(Fill),
            container(self.instances_list()).width(Fill).height(Fill),
        ]
        .spacing(16)
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// The filter tabs — Recent / Pinned / Modded / Vanilla.
    fn filter_tabs(&self) -> Element<'_, Message> {
        row([
            LibraryFilter::Recent,
            LibraryFilter::Pinned,
            LibraryFilter::Modded,
            LibraryFilter::Vanilla,
        ]
        .into_iter()
        .map(|filter| {
            let label = filter.label();
            let content = if self.filter == filter {
                Text::new(label).size(12).font(fonts::Inter::SEMI_BOLD).color(color::Primary::DEFAULT)
            } else {
                Text::new(label).size(12).font(fonts::Inter::MEDIUM).color(color::Text::MUTED)
            };
            TextButton::content(content)
                .padding(Padding::new(0.0).left(14).right(14).top(8).bottom(8))
                .on_press(Message::FilterChanged(filter))
                .into()
        }))
        .spacing(4)
        .into()
    }

    /// The view switcher — Grid / Compact / Table.
    fn view_switcher(&self) -> Element<'_, Message> {
        let entries = [
            (LibraryView::Grid, Icon::lucide().layout_grid()),
            (LibraryView::Compact, Icon::lucide().rows_3()),
            (LibraryView::Table, Icon::lucide().table()),
        ];
        row(entries.into_iter().map(|(view, icon_markup)| {
            let active = self.view == view;
            let (ink, weight) = if active {
                (color::Primary::DEFAULT, fonts::Inter::SEMI_BOLD)
            } else {
                (color::Text::MUTED, fonts::Inter::MEDIUM)
            };
            TextButton::content(
                row![
                    icon(icon_markup, 13.0, ink, 1.0),
                    Text::new(view.label()).size(12).font(weight).color(ink),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding(Padding::new(0.0).left(12).right(12).top(7).bottom(7))
            .on_press(Message::ViewChanged(view))
            .into()
        }))
        .spacing(2)
        .into()
    }

    /// The virtualized instance list. `responsive` measures the section so
    /// the viewport height and the grid's column count are exact; only the
    /// rows around the scroll offset are built, with top/bottom spacers
    /// holding the scroll height stable.
    fn instances_list(&self) -> Element<'_, Message> {
        let visible = self.visible();
        if visible.is_empty() {
            return container(
                Text::new(if self.instances.is_empty() {
                    "No instances yet — create your first one above."
                } else {
                    "No instances match the current search or filter."
                })
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
            )
            .width(Fill)
            .height(Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .into();
        }

        let view = self.view;
        let pitch = view.row_pitch();
        let gap = view.gap();
        let item_height = view.item_height();
        let scroll_y = self.scroll_y;

        responsive(move |size| {
            let columns = view.columns(size.width);
            let total_rows = visible.len().div_ceil(columns);
            let first = ((scroll_y / pitch).floor() as usize).saturating_sub(1);
            let viewport_rows = (size.height / pitch).ceil() as usize;
            let end = (first + viewport_rows + 2).min(total_rows);

            let mut content = column![Space::new().height(first as f32 * pitch)].spacing(gap);
            for row_index in first..end {
                let start = row_index * columns;
                let slice = &visible[start..(start + columns).min(visible.len())];
                let mut items: Vec<Element<'static, Message>> = Vec::with_capacity(columns);
                match view {
                    LibraryView::Grid => {
                        for instance in slice {
                            items.push(instance_card(instance, Message::Play(instance.name.clone())));
                        }
                    }
                    LibraryView::Compact => {
                        for instance in slice {
                            items.push(compact_row(instance, Message::Play(instance.name.clone())));
                        }
                    }
                    LibraryView::Table => {
                        for (offset, instance) in slice.iter().enumerate() {
                            items.push(table_row(start + offset, instance, Message::Play(instance.name.clone())));
                        }
                    }
                }
                // Pad short rows so Fill-width items keep a uniform size.
                for _ in slice.len()..columns {
                    items.push(Space::new().width(Fill).height(item_height).into());
                }
                content = content.push(row(items).spacing(24).width(Fill));
            }
            let bottom = (total_rows - end) as f32 * pitch;
            content = content.push(Space::new().height(bottom));

            let list = scrollable(content)
                .width(Fill)
                .height(Fill)
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y));

            // The table keeps its header pinned above the scrolling rows.
            if view == LibraryView::Table {
                column![table_header(), list].width(Fill).into()
            } else {
                list.into()
            }
        })
        .into()
    }
}

/// A hero icon button — one of the instance's context actions.
fn hero_menu_button(instance: &str, action: &'static str, icon_markup: &'static str) -> Element<'static, Message> {
    IconButton::new(icon_markup)
        .width(44)
        .height(44)
        .icon_size(16.0)
        .on_press(Message::InstanceMenu {
            instance: instance.to_string(),
            action,
        })
        .into()
}

/// One quick-action card.
fn quick_action_card(
    icon_markup: &'static str,
    ink: Color,
    fill: Color,
    title: &'static str,
    subtitle: &'static str,
    on_press: Message,
) -> Element<'static, Message> {
    TextButton::content(
        row![
            container(icon(icon_markup, 18.0, ink, 1.0))
                .width(44)
                .height(44)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(move |_| container::Style {
                    background: Some(Background::Color(fill)),
                    border: Border {
                        radius: 10.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }),
            column![
                Text::new(title).size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
                Text::new(subtitle).size(11).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill),
    )
    .padding(Padding::new(0.0).left(16).right(16).top(14).bottom(14))
    .width(Fill)
    .on_press(on_press)
    .into()
}

/// The status chip — the pulsing dot plus the uppercase label.
fn status_chip(state: InstanceState) -> Element<'static, Message> {
    let (label, ink, fill, pulsing) = match state {
        InstanceState::Playing => ("PLAYING", color::Primary::DEFAULT, color::Primary::TINT, true),
        InstanceState::Idle => ("IDLE", color::Text::MUTED, rgb!(255, 255, 255, 6%), false),
    };
    container(
        row![
            PulseDot::with_color(ink).size(6.0).pulsing(pulsing),
            Text::new(label).size(10).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding(Padding::new(0.0).left(8).right(8).top(3).bottom(3))
    .style(move |_| container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// An uppercase mono chip on a tinted fill.
fn chip(label: String, ink: Color, fill: Color) -> Element<'static, Message> {
    container(Text::new(label).size(10).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink))
        .padding(Padding::new(0.0).left(8).right(8).top(3).bottom(3))
        .style(move |_| container::Style {
            background: Some(Background::Color(fill)),
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// An SVG icon from a lucide markup string. The tint supplies the color
/// (its alpha is ignored by the svg texture pipeline); `opacity` controls
/// visibility without shifting layout.
fn icon(markup: &'static str, size: f32, tint: Color, opacity: f32) -> Element<'static, Message> {
    svg(iced::widget::svg::Handle::from_memory(markup.as_bytes()))
        .width(size)
        .height(size)
        .opacity(opacity)
        .style(move |_, _| svg::Style { color: Some(tint) })
        .into()
}

/// A coarse humanized "time ago" for the hero meta line.
fn relative_time(then: DateTime<Local>) -> String {
    let delta = Local::now() - then;
    let minutes = delta.num_minutes().max(0);
    if minutes < 1 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{minutes} minutes ago")
    } else if minutes < 60 * 24 {
        format!("{} hours ago", minutes / 60)
    } else {
        format!("{} days ago", minutes / (60 * 24))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library_with(instances: Vec<Instance>) -> Library {
        let mut library = Library::default();
        library.set_instances(instances);
        library
    }

    #[test]
    fn filters_narrow_the_list() {
        let mut library = library_with(Instance::seed_pack(Local::now()));
        assert_eq!(library.visible().len(), 48, "Recent shows everything");

        library.update(Message::FilterChanged(LibraryFilter::Pinned));
        assert!(library.visible().iter().all(|instance| instance.pinned), "Pinned shows only pinned");
        assert!(library.visible().len() < 48);
    }

    #[test]
    fn modded_and_vanilla_tabs_are_disjoint() {
        let mut library = library_with(Instance::seed_pack(Local::now()));
        library.update(Message::FilterChanged(LibraryFilter::Modded));
        let modded = library.visible().clone();
        assert!(!modded.is_empty());
        assert!(modded.iter().all(|instance| instance.loader != Loader::Vanilla));

        library.update(Message::FilterChanged(LibraryFilter::Vanilla));
        let vanilla = library.visible().clone();
        assert!(!vanilla.is_empty());
        assert!(vanilla.iter().all(|instance| instance.loader == Loader::Vanilla));
    }

    #[test]
    fn search_matches_by_name_case_insensitively() {
        let mut library = library_with(Instance::seed_pack(Local::now()));
        library.update(Message::SearchChanged("AETHER".to_string()));
        let visible = library.visible();
        assert!(!visible.is_empty(), "the first seed instance is 'Aether Legacy'");
        assert!(visible.iter().all(|instance| instance.name.to_lowercase().contains("aether")));

        library.update(Message::SearchChanged("no instance has this name".to_string()));
        assert!(library.visible().is_empty());
    }

    #[test]
    fn visible_sorts_most_recent_first() {
        let library = library_with(Instance::seed_pack(Local::now()));
        let visible = library.visible();
        for pair in visible.windows(2) {
            assert!(pair[0].recency() >= pair[1].recency(), "instances must sort most-recent first");
        }
    }

    #[test]
    fn view_changes_and_searches_reset_the_scroll_offset() {
        let mut library = library_with(Instance::seed_pack(Local::now()));
        library.update(Message::Scrolled(2500.0));
        assert_eq!(library.scroll_y, 2500.0);
        library.update(Message::SearchChanged("a".to_string()));
        assert_eq!(library.scroll_y, 0.0, "a new search restarts at the top");

        library.update(Message::Scrolled(1800.0));
        library.update(Message::ViewChanged(LibraryView::Table));
        assert_eq!(library.scroll_y, 0.0);
        assert_eq!(library.view, LibraryView::Table);
    }

    #[test]
    fn grid_columns_follow_width_with_a_cap() {
        assert_eq!(LibraryView::Grid.columns(340.0), 1);
        assert_eq!(LibraryView::Grid.columns(1156.0), 3, "1280px window minus rail and padding");
        assert_eq!(LibraryView::Grid.columns(4000.0), 4, "wide windows cap at four columns");
        assert_eq!(LibraryView::Compact.columns(4000.0), 1);
        assert_eq!(LibraryView::Table.columns(4000.0), 1);
    }

    #[test]
    fn row_pitches_match_the_fixed_component_heights() {
        assert_eq!(LibraryView::Grid.row_pitch(), CARD_HEIGHT + 24.0);
        assert_eq!(LibraryView::Compact.row_pitch(), COMPACT_ROW_HEIGHT + 24.0);
        assert_eq!(LibraryView::Table.row_pitch(), TABLE_ROW_HEIGHT);
    }
}
