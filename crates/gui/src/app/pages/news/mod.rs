//! The News home: Mojang's posts and the Java Edition patch notes, from the launcher's feed. A post opens on minecraft.net, and
//! a patch note reads in the launcher. The feed is kept on disk, so the page shows the last copy while the next one loads.

mod feature;
mod grid;
mod reader;

use std::collections::HashSet;
use std::sync::Arc;

use aurora_leaf::Icon;
use chrono::{DateTime, Duration, Local};
use iced::widget::{Column, Space, Text, column, container, image, markdown, row};
use iced::{Element, Fill, Padding, Size, alignment};
use piston_mc::news::Category;

use crate::app::components::icon_button::{IconButton, ghost_button_style};
use crate::app::components::{Tabs, TextButton, scrollbar};
use crate::app::lang;
use crate::app::news::feed::{Feed, absolute};
use crate::app::news::markdown::to_markdown;
use crate::app::retention::Retained;
use crate::app::theme::{color, fonts};

/// How many posts, or patch notes, the page shows before "Show more".
const PAGE_SIZE: usize = 24;
/// How old the feed may be before the page fetches it again when it opens.
const FRESH_MINUTES: i64 = 30;

/// Which list the page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// Mojang's posts, newest first, with the category filters.
    #[default]
    Posts,
    /// The Java Edition patch notes, newest first.
    Patches,
}

/// The patch note the reader shows: its index in the feed, and its body as markdown.
struct Reading {
    index: usize,
    article: markdown::Content,
}

/// The News page's state: the feed, the pictures it has loaded, and what the page shows of it.
#[derive(Default)]
pub struct State {
    feed: Option<Arc<Feed>>,
    /// Whether a fetch of the feed is under way.
    loading: bool,
    /// Why the latest fetch failed, until one succeeds.
    error: Option<String>,
    tab: Tab,
    category: Option<Category>,
    /// How many times "Show more" was pressed since the list last changed.
    more: usize,
    reading: Option<Reading>,
    pictures: Retained<String, image::Handle>,
    /// The pictures asked for, loaded or not, so no picture is asked for twice.
    requested: HashSet<String>,
}

/// What the page sends, and what the root sends back when the work it started finishes.
#[derive(Debug, Clone)]
pub enum Message {
    /// The cached feed was read, when there was one.
    Cached(Option<Feed>),
    /// The latest feed was fetched, or the fetch failed.
    Fetched(Result<Feed, String>),
    /// A picture finished loading, or failed (`None`).
    Picture { url: String, handle: Option<image::Handle> },
    /// The refresh button was pressed.
    Refresh,
    /// The button that opens minecraft.net was pressed.
    OpenSite,
    /// A tab was picked.
    Tab(Tab),
    /// A category was picked, or `None` for all of them.
    Category(Option<Category>),
    /// "Show more" was pressed.
    ShowMore,
    /// A post was pressed. Its article opens on minecraft.net. The index is the post's place in the feed.
    OpenPost(usize),
    /// A patch note was pressed, so the reader opens it. The index is its place in the feed.
    ReadPatch(usize),
    /// The reader's back button was pressed.
    CloseReader,
    /// A link in a patch note was pressed.
    Link(String),
}

impl State {
    /// Whether a feed, cached or fetched, is on hand.
    pub fn has_feed(&self) -> bool {
        self.feed.is_some()
    }

    /// Whether the feed is missing, or older than the page keeps it fresh for.
    pub fn is_stale(&self, now: DateTime<Local>) -> bool {
        self.feed
            .as_ref()
            .is_none_or(|feed| now - feed.fetched_at >= Duration::minutes(FRESH_MINUTES))
    }

    /// Marks a fetch as under way. It returns `false` when one already is, so no fetch starts twice.
    pub fn begin_fetch(&mut self) -> bool {
        if self.loading {
            return false;
        }
        self.loading = true;
        true
    }

    /// Takes the cached feed, unless a fetch has already brought a newer one.
    pub fn cached(&mut self, feed: Option<Feed>) {
        if self.feed.is_none()
            && let Some(feed) = feed
        {
            self.feed = Some(Arc::new(feed));
        }
    }

    /// Takes the latest feed, or keeps the reason its fetch failed.
    pub fn fetched(&mut self, result: Result<Feed, String>) {
        self.loading = false;
        match result {
            Ok(feed) => {
                self.feed = Some(Arc::new(feed));
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Keeps a loaded picture. A picture that failed to load is kept as nothing, and its place shows a plain panel.
    pub fn picture(&mut self, url: String, handle: Option<image::Handle>) {
        if let Some(handle) = handle {
            self.pictures.insert(url, handle, std::time::Instant::now());
        }
    }

    /// Counts the pictures of the News page as used while the page is on show, and drops the ones nobody has used for a
    /// while. The page forgets what it asked for of each one it drops, so it asks again when the picture comes back. Returns
    /// how many it dropped.
    pub fn sweep_pictures(&mut self, now: std::time::Instant, current: bool) -> usize {
        if current {
            self.pictures.touch_all(now);
        }
        let dropped = self.pictures.sweep(now);
        let count = dropped.len();
        for url in dropped {
            self.requested.remove(&url);
        }
        count
    }

    /// Drops every picture the page holds, and what it asked for, so the page asks again.
    pub fn clear_pictures(&mut self) {
        self.pictures.clear();
        self.requested.clear();
    }

    /// The minecraft.net article a post opens on, by the post's place in the feed.
    pub fn post_link(&self, index: usize) -> Option<String> {
        self.feed.as_ref()?.posts.get(index).map(|post| post.read_more_link.clone())
    }

    /// Applies a message the page answers itself. The messages the root answers are left alone.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tab(tab) => {
                self.tab = tab;
                self.more = 0;
                self.reading = None;
            }
            Message::Category(category) => {
                self.category = category;
                self.more = 0;
            }
            Message::ShowMore => self.more += 1,
            Message::ReadPatch(index) => self.read(index),
            Message::CloseReader => self.reading = None,
            Message::Refresh
            | Message::OpenSite
            | Message::OpenPost(_)
            | Message::Link(_)
            | Message::Cached(_)
            | Message::Fetched(_)
            | Message::Picture { .. } => {}
        }
    }

    /// Opens the reader on a patch note, with its body turned into markdown.
    fn read(&mut self, index: usize) {
        let Some(feed) = &self.feed else {
            return;
        };
        if let Some(patch) = feed.patches.get(index) {
            self.reading = Some(Reading {
                index,
                article: markdown::Content::parse(&to_markdown(&patch.body)),
            });
        }
    }

    /// How many posts or patch notes the list shows.
    fn limit(&self) -> usize {
        PAGE_SIZE * (self.more + 1)
    }

    /// The places in the feed of the posts the category filter lets through, newest first.
    fn filtered(&self, feed: &Feed) -> Vec<usize> {
        let mut places: Vec<usize> = feed
            .posts
            .iter()
            .enumerate()
            .filter(|(_, post)| self.category.as_ref().is_none_or(|category| post.category == *category))
            .map(|(place, _)| place)
            .collect();
        // Dates are ISO strings, so they sort as dates do. The sort is stable, so equal dates keep the feed's order.
        places.sort_by(|a, b| feed.posts[*b].date.cmp(&feed.posts[*a].date));
        places
    }

    /// The pictures the page shows that have not been asked for yet, marked as asked for. The featured post uses its wide
    /// picture, and the rest of the grid their cards'.
    pub fn take_missing_pictures(&mut self) -> Vec<String> {
        let Some(feed) = self.feed.clone() else {
            return Vec::new();
        };
        let mut urls = Vec::new();
        match &self.reading {
            Some(reading) => urls.extend(feed.patches.get(reading.index).map(|patch| absolute(&patch.image.url))),
            None => match self.tab {
                Tab::Posts => {
                    for (position, place) in self.filtered(&feed).into_iter().take(self.limit() + 1).enumerate() {
                        let post = &feed.posts[place];
                        let url = if position == 0 {
                            &post.news_page_image.url
                        } else {
                            &post.play_page_image.url
                        };
                        urls.push(absolute(url));
                    }
                }
                Tab::Patches => urls.extend(feed.patches.iter().take(self.limit()).map(|patch| absolute(&patch.image.url))),
            },
        }
        urls.into_iter().filter(|url| self.requested.insert(url.clone())).collect()
    }

    /// The page: its header and tabs, and the list or the reader beneath them.
    pub fn view(&self, viewport: Size) -> Element<'_, Message> {
        let columns = if viewport.width >= 1100.0 { 3 } else { 2 };
        let body: Element<'_, Message> = match (&self.reading, &self.feed) {
            (Some(reading), Some(feed)) => reader::view(reading, feed, &self.pictures),
            (None, Some(feed)) => self.listing(feed, columns),
            (_, None) => self.empty(),
        };
        let page = column![self.header(), self.tabs()]
            .spacing(18)
            .push(scrollbar::scroll(body).width(Fill).height(Fill));
        container(page)
            .padding(Padding::new(0.0).top(24).left(28).right(28).bottom(0))
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// The title, the time the feed was updated, and the refresh and minecraft.net buttons.
    fn header(&self) -> Element<'_, Message> {
        let title = column![
            Text::new(lang::t("news.title"))
                .size(24)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(color::text_headline()),
            Text::new(self.subtitle()).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
        ]
        .spacing(2);
        row![
            title,
            Space::new().width(Fill),
            TextButton::ghost(lang::t("news.open_site")).on_press(Message::OpenSite),
            IconButton::new(Icon::lucide().refresh_cw())
                .size(36)
                .radius(12.0)
                .icon_size(16.0)
                .style(ghost_button_style())
                .disabled(self.loading)
                .on_press(Message::Refresh),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill)
        .into()
    }

    /// The line under the title: that the feed is loading, when it was updated, or why it could not be.
    fn subtitle(&self) -> String {
        match (&self.feed, &self.error) {
            (_, Some(error)) => lang::t_with("news.failed", &[("error", error.as_str())]),
            (Some(feed), None) if self.loading => lang::t_with("news.loading_over", &[("ago", ago(feed.fetched_at, Local::now()).as_str())]),
            (Some(feed), None) => lang::t_with("news.updated", &[("ago", ago(feed.fetched_at, Local::now()).as_str())]),
            (None, None) => lang::t("news.loading").to_owned(),
        }
    }

    /// The tabs: Posts and Patch notes, underlined.
    fn tabs(&self) -> Element<'_, Message> {
        let active = usize::from(self.tab == Tab::Patches);
        Tabs::new([lang::t("news.tab.posts"), lang::t("news.tab.patches")])
            .active(active)
            .on_select(|index| Message::Tab(if index == 0 { Tab::Posts } else { Tab::Patches }))
            .underline()
            .into()
    }

    /// The list under the tabs: the category filters and the featured post, then the grid, with "Show more" when it
    /// has more to show. Patch notes list without filters.
    fn listing<'a>(&'a self, feed: &'a Feed, columns: usize) -> Element<'a, Message> {
        let limit = self.limit();
        match self.tab {
            Tab::Posts => {
                let places = self.filtered(feed);
                let mut page = Column::new().spacing(20).width(Fill).push(self.filters());
                if let Some(&featured) = places.first() {
                    page = page.push(feature::view(feed, featured, &self.pictures));
                }
                let grid: Vec<usize> = places.iter().skip(1).take(limit).copied().collect();
                page = page.push(grid::posts(feed, &grid, &self.pictures, columns));
                if places.len() > limit + 1 {
                    page = page.push(self.more_button());
                }
                page.into()
            }
            Tab::Patches => {
                let mut page = Column::new().spacing(20).width(Fill);
                page = page.push(grid::patches(feed, limit, &self.pictures, columns));
                if feed.patches.len() > limit {
                    page = page.push(self.more_button());
                }
                page.into()
            }
        }
    }

    /// The category filters: All, then the four categories the feed names.
    fn filters(&self) -> Element<'_, Message> {
        let choices: [(Option<Category>, &'static str); 5] = [
            (None, "news.filter.all"),
            (Some(Category::MinecraftJavaEdition), "news.filter.java"),
            (Some(Category::MinecraftForWindows), "news.filter.bedrock"),
            (Some(Category::MinecraftDungeons), "news.filter.dungeons"),
            (Some(Category::MinecraftLegends), "news.filter.legends"),
        ];
        let chips = choices.into_iter().map(|(category, label)| {
            let active = self.category == category;
            let content = if active {
                Text::new(lang::t(label)).size(12).font(fonts::Inter::SEMI_BOLD).color(color::primary())
            } else {
                Text::new(lang::t(label)).size(12).font(fonts::Inter::MEDIUM).color(color::text_muted())
            };
            TextButton::content(content)
                .padding(Padding::new(0.0).left(14.0).right(14.0).top(8.0).bottom(8.0))
                .on_press(Message::Category(category))
                .into()
        });
        row(chips).spacing(4).into()
    }

    /// The "Show more" button under the list.
    fn more_button(&self) -> Element<'_, Message> {
        TextButton::ghost(lang::t("news.show_more")).on_press(Message::ShowMore).into()
    }

    /// What the page shows until a feed is on hand: that it is loading, or why it is not.
    fn empty(&self) -> Element<'_, Message> {
        let text = match &self.error {
            Some(error) => lang::t_with("news.failed", &[("error", error.as_str())]),
            None => lang::t("news.loading").to_owned(),
        };
        container(Text::new(text).size(13).font(fonts::Inter::REGULAR).color(color::text_muted()))
            .padding(Padding::new(16.0))
            .into()
    }
}

/// How long ago the feed was fetched, in words, from `now`.
fn ago(fetched: DateTime<Local>, now: DateTime<Local>) -> String {
    let minutes = usize::try_from((now - fetched).num_minutes()).unwrap_or(0);
    match minutes {
        0 => lang::t("news.ago.now").to_owned(),
        1..60 => lang::t_count("news.ago.minutes", minutes),
        60..1440 => lang::t_count("news.ago.hours", minutes / 60),
        _ => lang::t_count("news.ago.days", minutes / 1440),
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/news/mod.rs"]
mod tests;
