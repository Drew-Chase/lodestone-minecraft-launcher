use super::*;
use iced::color;

#[test]
fn scale_builds_chainable_text() {
    let _: Text = display!("Play anything.");
    let _: Text = h1!("Your Library");
    let _: Text = h2!("All the Mods 10");
    let _: Text = h3!("Recently played");
    let _: Text = body!("Hello world").color(color!(0xff00ff));
    let _: Text = small!("Last played 2 hours ago");
    let _: Text = eyebrow!("Instances · 12");
    let _: Text = code!("[14:02:11] INFO Loading 412 mods");
}
