use super::*;
use ::image::Rgba;

#[test]
fn a_picture_keeps_its_proportions_within_the_longest_side() {
    assert_eq!(fitted(512, 256, 300), (300, 150));
    assert_eq!(fitted(256, 512, 300), (150, 300));
    assert_eq!(fitted(300, 300, 300), (300, 300));
    assert_eq!(fitted(0, 0, 300), (300, 300), "an empty picture gets the square");
}

#[test]
fn the_banner_has_the_banner_size_and_centres_the_picture() {
    let picture = RgbaImage::from_pixel(64, 64, Rgba([200, 30, 30, 255]));
    let banner = banner(&picture);
    assert_eq!(banner.dimensions(), (BANNER_WIDTH, BANNER_HEIGHT));
    let centre = banner.get_pixel(BANNER_WIDTH / 2, BANNER_HEIGHT / 2);
    assert_eq!(*centre, Rgba([200, 30, 30, 255]), "the picture itself sits in the middle");
    let corner = banner.get_pixel(0, 0);
    assert!(corner[0] < 200 && corner[0] > 0, "the corners are the darkened, blurred backdrop");
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
