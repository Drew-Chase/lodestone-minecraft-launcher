//! The News home's feed and the work around it: reading the cached copy, fetching the latest one, loading the pictures the
//! page shows, and opening the links it offers. The page itself is `pages::news`.

pub mod cache;
pub mod feed;
pub mod markdown;

use chrono::{DateTime, Local};
use iced::Task;

use super::instances::open_source;
use super::pages::discover::images::load_image;
use super::pages::news::Message;
use super::{LodestoneAppMessage, LodestoneLauncher};
use feed::Feed;

/// The Minecraft site the page's button opens.
const SITE: &str = "https://www.minecraft.net";

impl LodestoneLauncher {
    /// Opens the News feed: the cached copy at once when there is one, and the latest feed when there is none or the copy
    /// is older than the page keeps it fresh for.
    pub(super) fn open_news(&mut self) -> Task<LodestoneAppMessage> {
        let mut tasks = Vec::new();
        if !self.news.has_feed() {
            tasks.push(read_cache());
        }
        if self.news.is_stale(Local::now()) && self.news.begin_fetch() {
            tasks.push(fetch());
        }
        tasks.push(self.news_pictures());
        Task::batch(tasks)
    }

    /// Applies a message from the News page. The pictures a message makes the page show are loaded after it.
    pub(super) fn news_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Refresh => {
                if self.news.begin_fetch() {
                    fetch()
                } else {
                    Task::none()
                }
            }
            Message::OpenSite => open_source(SITE.to_owned()),
            Message::OpenPost(index) => match self.news.post_link(index) {
                Some(link) => open_source(link),
                None => Task::none(),
            },
            Message::Link(url) => open_source(url),
            Message::Cached(feed) => {
                self.news.cached(feed);
                self.news_pictures()
            }
            Message::Fetched(result) => {
                self.news.fetched(result);
                self.news_pictures()
            }
            Message::Picture { url, handle } => {
                self.news.picture(url, handle);
                Task::none()
            }
            other => {
                self.news.update(other);
                self.news_pictures()
            }
        }
    }

    /// Starts loading the pictures the page shows and has not asked for yet.
    pub(super) fn news_pictures(&mut self) -> Task<LodestoneAppMessage> {
        Task::batch(self.news.take_missing_pictures().into_iter().map(picture))
    }
}

/// Loads the picture at `url` off the UI thread. Its answer is a page message, whether or not it loaded.
fn picture(url: String) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            let handle = load_image(url.clone()).await;
            (url, handle)
        },
        |(url, handle)| LodestoneAppMessage::News(Message::Picture { url, handle }),
    )
}

/// Reads the cached feed off the UI thread. A cache that is missing or does not read gives nothing.
fn read_cache() -> Task<LodestoneAppMessage> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                cache::read().and_then(|cached| {
                    let written = DateTime::<Local>::from(cached.written);
                    Feed::read(&cached.news, &cached.patches, written).ok()
                })
            })
            .await
            .unwrap_or_default()
        },
        |feed| LodestoneAppMessage::News(Message::Cached(feed)),
    )
}

/// Fetches the latest feed, writes its copy to the cache, and reads it, all off the UI thread.
fn fetch() -> Task<LodestoneAppMessage> {
    Task::perform(
        async {
            let (news, patches) = feed::download().await?;
            tokio::task::spawn_blocking(move || {
                cache::write(&news, &patches);
                Feed::read(&news, &patches, Local::now())
            })
            .await
            .map_err(|error| error.to_string())?
        },
        |result| LodestoneAppMessage::News(Message::Fetched(result)),
    )
}
