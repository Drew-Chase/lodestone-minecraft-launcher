//! The instance page's pictures: the Mods tab's project pictures and the Screenshots grid's thumbnails. The page counts them as used while it
//! is on show, and drops the ones nobody has used for a while.

use super::*;

impl InstancePage {
    /// Counts the page's pictures as used while the page is on show, and drops those nobody has used for a while: the
    /// Mods tab's project pictures, and the Screenshots grid's thumbnails while that tab is open. Returns how many it dropped.
    pub fn sweep_pictures(&mut self, now: std::time::Instant, current: bool) -> usize {
        let thumbnails = self.screenshots.sweep_thumbnails(now, current && self.tab == Tab::Screenshots);
        thumbnails + self.mods.sweep_icons(now, current)
    }

    /// Drops the pictures the page holds, so they are asked for and decoded again when the page shows them.
    pub fn clear_pictures(&mut self) {
        self.screenshots.clear_thumbnails();
        self.mods.clear_icons();
    }
}
