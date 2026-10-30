use super::*;

#[test]
fn an_svg_raster_counts_its_pixels() {
    assert_eq!(
        classify("allocating 6249826457760883942 408x179"),
        Some(IcedEvent::SvgRaster { pixels: 73_032 })
    );
}

#[test]
fn atlas_entries_and_dropped_images_are_classified() {
    assert_eq!(
        classify("Allocated atlas entry: Contiguous(Partial { layer: 0 })"),
        Some(IcedEvent::AtlasAlloc)
    );
    assert_eq!(
        classify("Removing atlas entry: Contiguous(Partial { layer: 0 })"),
        Some(IcedEvent::AtlasEvict)
    );
    assert_eq!(classify("Dropping image allocation: Id(3)"), Some(IcedEvent::ImageDrop));
}

#[test]
fn other_iced_messages_are_ignored() {
    assert_eq!(classify("Deallocating atlas: Partial { layer: 0 }"), None);
    assert_eq!(classify("allocating 12 notasize"), None);
}
