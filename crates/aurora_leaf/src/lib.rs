mod internal {
    aurora_iconify::icon_sets!("lucide", "material-symbols", "mage", "material-icon-theme", "selfhst", "at-icons");
    aurora_fonts::font_families!("Inter", "JetBrains Mono");
}

pub use crate::internal::{FontFamily, Icon};
