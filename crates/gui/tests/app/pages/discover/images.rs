use super::*;
use ::image::Rgba;

#[test]
fn a_picture_covers_the_banner_within_its_proportions() {
    assert_eq!(
        covering(512, 256, BANNER_WIDTH, BANNER_HEIGHT),
        (1200, 600),
        "a wide picture is cut at its top and bottom"
    );
    assert_eq!(
        covering(600, 600, BANNER_WIDTH, BANNER_HEIGHT),
        (1200, 1200),
        "a square picture is cut at its top and bottom"
    );
    assert_eq!(
        covering(1600, 400, BANNER_WIDTH, BANNER_HEIGHT),
        (1600, 400),
        "a picture as tall as the banner keeps its width"
    );
}

#[test]
fn the_banner_has_the_banner_size_and_is_the_picture_covering_it() {
    let picture = RgbaImage::from_pixel(64, 64, Rgba([200, 30, 30, 255]));
    let banner = banner(&picture);
    assert_eq!(banner.dimensions(), (BANNER_WIDTH, BANNER_HEIGHT));
    assert_eq!(*banner.get_pixel(BANNER_WIDTH / 2, BANNER_HEIGHT / 2), Rgba([200, 30, 30, 255]));
    assert_eq!(
        *banner.get_pixel(0, 0),
        Rgba([200, 30, 30, 255]),
        "the corners are the picture too, with no backdrop"
    );
    assert_eq!(*banner.get_pixel(BANNER_WIDTH - 1, BANNER_HEIGHT - 1), Rgba([200, 30, 30, 255]));
}

#[test]
fn a_wide_picture_is_cropped_at_its_edges_not_squeezed() {
    // Three bands of colour. The banner is the middle band, since 200 columns are cut from each side.
    let mut picture = RgbaImage::new(1600, 400);
    for (x, _, pixel) in picture.enumerate_pixels_mut() {
        *pixel = match x {
            0..200 => Rgba([200, 30, 30, 255]),
            200..1400 => Rgba([30, 200, 30, 255]),
            _ => Rgba([30, 30, 200, 255]),
        };
    }
    let banner = banner(&picture);
    assert_eq!(*banner.get_pixel(0, 0), Rgba([30, 200, 30, 255]));
    assert_eq!(*banner.get_pixel(BANNER_WIDTH - 1, BANNER_HEIGHT - 1), Rgba([30, 200, 30, 255]));
}

#[test]
fn the_icon_is_the_centred_square_of_the_picture() {
    let picture = RgbaImage::from_pixel(300, 200, Rgba([10, 20, 30, 255]));
    assert_eq!(square(&picture).dimensions(), (ICON_SIDE, ICON_SIDE));
}

#[test]
fn an_instance_keeps_its_icon_and_banner_as_pngs() {
    let picture = RgbaImage::from_pixel(64, 32, Rgba([200, 100, 50, 255]));
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("the picture encodes");
    let art = encode_pack_art(&bytes).expect("the art builds");
    let icon = ::image::load_from_memory(&art.icon).expect("the icon is a PNG").to_rgba8();
    let banner = ::image::load_from_memory(&art.banner).expect("the banner is a PNG").to_rgba8();
    assert_eq!(icon.dimensions(), (ICON_SIDE, ICON_SIDE));
    assert_eq!(banner.dimensions(), (BANNER_WIDTH, BANNER_HEIGHT));
}
