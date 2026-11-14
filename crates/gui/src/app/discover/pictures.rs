//! The Discover page's picture loads: the banners and icons its screen needs, each asked for once and loaded at the size it is
//! drawn at. The page decides what it needs; this turns that into tasks.

use iced::Task;

use super::ICON_TIMEOUT;
use crate::app::pages::discover::{self, Asset, Message};
use crate::app::{LodestoneAppMessage, LodestoneLauncher};

impl LodestoneLauncher {
    /// Loads the pictures the page's screen needs, each one once, at the size each is drawn.
    pub(in crate::app) fn picture_tasks(&mut self) -> Task<LodestoneAppMessage> {
        let wanted = self.discover.wanted(self.window_size);
        Task::batch(wanted.into_iter().map(|asset| {
            let answer = asset.clone();
            let load = async move {
                match asset {
                    Asset::Banner(url) => discover::load_banner(url).await,
                    Asset::Icon { url, px } => discover::load_icon(url, px).await,
                    Asset::Native(url) => discover::load_image(url).await,
                }
            };
            // A picture that takes too long is given up, so its place in the loads is freed for the rest.
            let load = async move { tokio::time::timeout(ICON_TIMEOUT, load).await.ok().flatten() };
            Task::perform(load, move |handle| {
                LodestoneAppMessage::Discover(Message::PictureReady { asset: answer, handle })
            })
        }))
    }
}
