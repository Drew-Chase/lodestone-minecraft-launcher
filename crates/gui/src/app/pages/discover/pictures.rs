//! The Discover page's pictures: which of them the screen draws, the loads the page asks for, and the pictures it lets go of.
//! A picture is kept while the screen uses it and for [`IDLE`](crate::app::retention::IDLE) after that. The sweep then drops
//! it and the page forgets that it asked, so a picture that comes back on screen is asked for again.

use std::time::Instant;

use iced::Size;
use iced::widget::image::Handle;

use super::card::{THUMB_SIZE, TITLE_ICON_SIZE};
use super::images::Asset;
use super::types::Artwork;
use super::{Discover, DiscoverView, hero, window};
use crate::app::scale;

/// How many pictures load at once. A page of results asks for all its pictures together, and decoding them as one burst
/// is what stalls the window when the tabs change, so the rest wait for a place.
const ICONS_AT_ONCE: usize = 4;

impl Discover {
    /// The pictures the screen draws: the banners and icons of the rows the window builds, and the hero's banner while the top
    /// of the page shows it. Top to bottom, each listed once per use.
    pub(super) fn on_screen(&self, window: Size) -> Vec<Asset> {
        let mut assets = Vec::new();
        let top = window::results_top(self, window.width);
        // The hero's banner is drawn while the top of the page is.
        if self.scroll_y < top
            && hero::shown(self)
            && let Some(url) = self.results.hits.first().and_then(|hit| hit.banner_url.as_deref())
        {
            assets.push(Asset::Banner(url.to_string()));
        }
        let shape = window::shape(self.view, window::results_width(self, window.width));
        let hits = &self.results.hits;
        let total = hits.len().div_ceil(shape.columns);
        let rows = window::rows(self.scroll_y, top, window.height, shape.pitch(), total);
        let end = (rows.end * shape.columns).min(hits.len());
        let start = (rows.first * shape.columns).min(end);
        for hit in &hits[start..end] {
            if matches!(self.view, DiscoverView::Grid) {
                if let Some(url) = hit.banner_url.as_deref() {
                    assets.push(Asset::Banner(url.to_string()));
                }
                if let Some(url) = hit.icon_url.as_deref() {
                    assets.push(Asset::Icon {
                        url: url.to_string(),
                        px: scale::device_px(TITLE_ICON_SIZE),
                    });
                }
            } else if let Some(url) = hit.icon_url.as_deref() {
                assets.push(Asset::Icon {
                    url: url.to_string(),
                    px: scale::device_px(THUMB_SIZE),
                });
            }
        }
        assets
    }

    /// The pictures to start loading now: each of the screen's pictures once, and as many as the loads in flight leave room
    /// for, from the front of the queue. The pictures on screen count as used. A queued picture that has left the screen is
    /// dropped, and asked for again if it comes back.
    pub fn wanted(&mut self, window: Size) -> Vec<Asset> {
        let on_screen = self.on_screen(window);
        let mut left = Vec::new();
        self.icon_queue.retain(|asset| {
            let keep = on_screen.contains(asset);
            if !keep {
                left.push(asset.clone());
            }
            keep
        });
        for asset in left {
            self.requested.remove(&asset);
        }
        for asset in &on_screen {
            if self.requested.insert(asset.clone()) {
                self.icon_queue.push_back(asset.clone());
            }
        }
        let now = Instant::now();
        for asset in &on_screen {
            self.pictures.touch(asset, now);
        }
        let room = ICONS_AT_ONCE.saturating_sub(self.icons_loading);
        let started = room.min(self.icon_queue.len());
        self.icons_loading += started;
        self.icon_queue.drain(..started).collect()
    }

    /// A picture finished loading, or could not be had (`None`). Its place in the loads is free for the next one in the queue.
    pub(super) fn picture_ready(&mut self, asset: Asset, handle: Option<Handle>) {
        self.icons_loading = self.icons_loading.saturating_sub(1);
        match handle {
            Some(handle) => self.pictures.insert(asset, handle, Instant::now()),
            None => {
                self.failed.insert(asset);
            }
        }
    }

    /// Counts the pictures the screen uses as used, while the page is the one on show, and drops the pictures nobody has used
    /// for [`IDLE`](crate::app::retention::IDLE). The page forgets what it asked for of each one it drops. Returns how many it
    /// dropped.
    pub fn sweep_pictures(&mut self, now: Instant, window: Size, current: bool) -> usize {
        if current {
            for asset in self.on_screen(window) {
                self.pictures.touch(&asset, now);
            }
        }
        let dropped = self.pictures.sweep(now);
        let count = dropped.len();
        for asset in dropped {
            self.requested.remove(&asset);
        }
        count
    }

    /// Drops every picture the page holds, and what it asked for, so the screen asks again.
    pub fn clear_pictures(&mut self) {
        self.pictures.clear();
        self.requested.clear();
        self.failed.clear();
        self.icon_queue.clear();
    }
}

impl Discover {
    /// The art a project shows as its banner: the platform's picture once it has loaded, a placeholder while it loads, and the
    /// generated scene only when the platform gave no picture or it could not be had.
    pub(super) fn artwork(&self, url: Option<&str>) -> Artwork<'_> {
        self.art(url, |url| Asset::Banner(url.to_string()))
    }

    /// The art a project shows as its icon, drawn `logical` pixels square: the picture at the window's scale. It stands in as
    /// the banner does.
    pub(super) fn icon_artwork(&self, url: Option<&str>, logical: f32) -> Artwork<'_> {
        let px = scale::device_px(logical);
        self.art(url, |url| Asset::Icon { url: url.to_string(), px })
    }

    /// The art for the picture `asset` makes of `url`.
    fn art(&self, url: Option<&str>, asset: impl FnOnce(&str) -> Asset) -> Artwork<'_> {
        let Some(url) = url else {
            return Artwork::Scene;
        };
        let asset = asset(url);
        match self.pictures.get(&asset) {
            Some(handle) => Artwork::Loaded(handle),
            None if self.failed.contains(&asset) => Artwork::Scene,
            None => Artwork::Loading,
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/pictures.rs"]
mod tests;
