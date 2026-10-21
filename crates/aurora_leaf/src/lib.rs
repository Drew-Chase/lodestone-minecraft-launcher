mod internal {
    aurora_iconify::icon_sets!("lucide", "material-symbols", "mage");
    aurora_fonts::font_families!("Inter", "JetBrains Mono");
}

pub use crate::internal::{FontFamily, Icon};
