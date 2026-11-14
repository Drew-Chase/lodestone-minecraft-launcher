//! The resource page's pictures and icons: what the page asks for, how long it keeps each, and the sweep that lets go of the ones it no
//! longer shows. A picture the sweep drops is forgotten by the page, so it is asked for again when it returns.

use super::*;

impl ResourcePage {
    /// The icon at `url`, drawn `logical` pixels square, once it has loaded at the window's scale.
    pub(super) fn icon(&self, url: &str, logical: f32) -> Option<&image::Handle> {
        self.icons.get(&(url.to_string(), scale::device_px(logical)))
    }

    /// The icon addresses not yet asked for, each returned once at the side they are drawn at, as one icon request.
    pub(super) fn wanted_icons(&mut self, urls: impl IntoIterator<Item = String>, logical: f32) -> Vec<Effect> {
        let px = scale::device_px(logical);
        let fresh: Vec<(String, u32)> = urls
            .into_iter()
            .map(|url| (url, px))
            .filter(|key| self.requested_icons.insert(key.clone()))
            .collect();
        if fresh.is_empty() { Vec::new() } else { vec![Effect::Icons(fresh)] }
    }

    /// Counts the pictures and icons of a page on show as used, and drops the ones nobody has used for a while. Returns how many
    /// it dropped. The page forgets what it asked for of each one it drops, so it asks again if the picture comes back.
    pub fn sweep_pictures(&mut self, now: Instant, current: bool) -> usize {
        if current {
            self.pictures.touch_all(now);
            self.icons.touch_all(now);
        }
        let dropped = self.pictures.sweep(now);
        let mut count = dropped.len();
        for url in dropped {
            self.requested.remove(&url);
        }
        let dropped = self.icons.sweep(now);
        count += dropped.len();
        for key in dropped {
            self.requested_icons.remove(&key);
        }
        count
    }

    /// The requests for the pictures the page shows, made again when it comes back on screen. A picture the sweep dropped while
    /// the page was away is asked for; one still held is not asked for twice.
    pub fn reshown(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        let icon = self.listing.as_ref().and_then(|listing| listing.icon_url.clone());
        effects.extend(self.wanted_icons(icon, header::ICON_SIZE));
        let avatars: Vec<String> = match &self.details {
            Load::Loaded(details) => details.info.creators.iter().filter_map(|creator| creator.avatar_url.clone()).collect(),
            _ => Vec::new(),
        };
        effects.extend(self.wanted_icons(avatars, SMALL_ICON));
        let bundled: Vec<String> = match &self.contents {
            Load::Loaded(bundled) => bundled.iter().filter_map(|project| project.icon_url.clone()).collect(),
            _ => Vec::new(),
        };
        effects.extend(self.wanted_icons(bundled, SMALL_ICON));
        let readme: Vec<String> = self.readme.as_ref().map(readme_pictures).unwrap_or_default();
        effects.extend(self.wanted_all(readme));
        effects.extend(self.gallery_effects());
        effects.extend(self.description_effects());
        if self.tab == Tab::Changelog {
            effects.extend(self.changelog_effects());
        }
        effects
    }

    /// Drops every picture and icon the page holds, and what it asked for, so the page asks again.
    pub fn clear_pictures(&mut self) {
        self.pictures.clear();
        self.requested.clear();
        self.icons.clear();
        self.requested_icons.clear();
    }
}
