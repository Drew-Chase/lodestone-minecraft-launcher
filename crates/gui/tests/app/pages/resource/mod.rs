use super::*;
use lodestone_platforms::{GalleryImage, ProjectInfo};

/// The project a card would open.
fn listing() -> Listing {
    Listing {
        source: Source::Modrinth,
        content: ContentType::Mod,
        project_id: "sodium".to_string(),
        name: "Sodium".to_string(),
        author: None,
        summary: String::new(),
        downloads: 0,
        icon_url: None,
        web_url: None,
        updated: None,
    }
}

/// The details of a project with `gallery` pictures.
fn details(gallery: usize) -> ContentDetails {
    ContentDetails {
        id: "sodium".to_string(),
        slug: None,
        name: "Sodium".to_string(),
        author: None,
        summary: String::new(),
        description: None,
        description_format: DescriptionFormat::Markdown,
        downloads: 0,
        icon_url: None,
        web_url: None,
        content_type: ContentType::Mod,
        info: ProjectInfo {
            gallery: (0..gallery)
                .map(|index| GalleryImage {
                    thumbnail_url: format!("thumb-{index}"),
                    full_url: format!("full-{index}"),
                    title: None,
                    description: None,
                    featured: false,
                })
                .collect(),
            ..ProjectInfo::default()
        },
    }
}

/// A release for the given channel and Minecraft versions.
fn release(id: &str, channel: Option<Channel>, minecraft: &[&str]) -> ContentVersion {
    ContentVersion {
        id: id.to_string(),
        project_id: Some("sodium".to_string()),
        name: None,
        version_number: id.to_string(),
        content_type: ContentType::Mod,
        minecraft_versions: minecraft.iter().map(|version| version.to_string()).collect(),
        loaders: vec!["fabric".to_string()],
        downloads: 0,
        date: None,
        changelog_url: None,
        files: Vec::new(),
        changelog: None,
        channel,
        dependencies: Vec::new(),
    }
}

#[test]
fn a_page_offers_only_the_tabs_its_platform_has_data_for() {
    let mut page = ResourcePage::default();
    page.show(listing(), None);
    assert_eq!(page.tabs(), vec![Tab::Overview, Tab::Versions], "the releases are still loading");
    let generation = page.generation();
    page.update(Message::Details {
        generation,
        result: Ok(Box::new(details(2))),
    });
    page.update(Message::Versions {
        generation,
        result: Ok(vec![release("a", None, &["1.21.1"])]),
    });
    assert_eq!(
        page.tabs(),
        vec![Tab::Overview, Tab::Gallery, Tab::Versions],
        "no release has a changelog"
    );
}

#[test]
fn an_answer_for_a_project_the_page_has_left_is_dropped() {
    let mut page = ResourcePage::default();
    page.show(listing(), None);
    let stale = page.generation();
    page.show(listing(), None);
    page.update(Message::Details {
        generation: stale,
        result: Ok(Box::new(details(3))),
    });
    assert!(matches!(page.details, Load::Loading));
}

#[test]
fn the_release_filters_keep_the_releases_on_their_channel_and_version() {
    let mut page = ResourcePage::default();
    page.show(listing(), None);
    let generation = page.generation();
    page.update(Message::Versions {
        generation,
        result: Ok(vec![
            release("a", Some(Channel::Release), &["1.21.1"]),
            release("b", Some(Channel::Beta), &["1.21.1"]),
            release("c", Some(Channel::Release), &["1.20.1"]),
        ]),
    });
    assert_eq!(page.filtered_versions(), vec![0, 1, 2]);
    page.update(Message::ChannelSelected(1));
    assert_eq!(page.filtered_versions(), vec![0, 2], "release only");
    page.update(Message::GameVersionSelected(2));
    assert_eq!(page.filtered_versions(), vec![2], "and 1.20.1 only");
    page.update(Message::GameVersionSelected(0));
    assert_eq!(page.filtered_versions(), vec![0, 2], "any version again");
}

#[test]
fn each_game_version_is_listed_once_in_the_order_the_releases_give_them() {
    let versions = vec![
        release("a", None, &["1.21.1", "1.21"]),
        release("b", None, &["1.21.1"]),
        release("c", None, &["1.20.1"]),
    ];
    assert_eq!(game_versions_of(&versions), ["1.21.1", "1.21", "1.20.1"]);
}

#[test]
fn a_gallery_picture_steps_within_the_gallery_only() {
    let mut page = ResourcePage::default();
    page.show(listing(), None);
    let generation = page.generation();
    page.update(Message::Details {
        generation,
        result: Ok(Box::new(details(2))),
    });
    page.update(Message::OpenPicture(0));
    assert_eq!(page.gallery_open, Some(0));
    page.update(Message::StepPicture(-1));
    assert_eq!(page.gallery_open, Some(0), "there is no picture before the first");
    page.update(Message::StepPicture(1));
    assert_eq!(page.gallery_open, Some(1));
    page.update(Message::StepPicture(1));
    assert_eq!(page.gallery_open, Some(1), "there is no picture after the last");
}

#[test]
fn back_asks_the_root_to_leave_the_page() {
    let mut page = ResourcePage::default();
    page.show(listing(), None);
    assert!(matches!(page.update(Message::Back).as_slice(), [Effect::Back]));
}

#[test]
fn the_page_returns_to_the_instance_that_opened_it_only_until_another_project_is_shown() {
    let instance = InstanceChoice {
        id: "ad-astra".to_string(),
        name: "Ad Astra".to_string(),
        loader: crate::app::pages::library::instance::Loader::Fabric,
        mc_version: "1.20.1".to_string(),
        locked: false,
    };
    let mut page = ResourcePage::default();
    page.show(listing(), Some(instance));
    assert_eq!(page.origin().map(|origin| origin.name.as_str()), Some("Ad Astra"));
    page.show(listing(), None);
    assert!(page.origin().is_none(), "a project opened from Discover returns to Discover");
}

/// An instance on `minecraft`, the one a page is opened from.
fn instance(minecraft: &str) -> InstanceChoice {
    InstanceChoice {
        id: "ad-astra".to_string(),
        name: "Ad Astra".to_string(),
        loader: crate::app::pages::library::instance::Loader::Fabric,
        mc_version: minecraft.to_string(),
        locked: false,
    }
}

#[test]
fn a_page_opened_from_an_instance_filters_to_its_minecraft_version() {
    let mut page = ResourcePage::default();
    page.show(listing(), Some(instance("1.20.1")));
    let generation = page.generation();
    page.update(Message::Versions {
        generation,
        result: Ok(vec![release("new", None, &["1.20.1"]), release("old", None, &["1.19.4"])]),
    });
    assert_eq!(page.game_version.as_deref(), Some("1.20.1"));
    assert_eq!(page.filtered_versions(), vec![0]);
}

#[test]
fn a_page_keeps_any_version_when_no_release_fits_the_instance() {
    let mut page = ResourcePage::default();
    page.show(listing(), Some(instance("1.20.1")));
    let generation = page.generation();
    page.update(Message::Versions {
        generation,
        result: Ok(vec![release("old", None, &["1.19.4"])]),
    });
    assert_eq!(page.game_version, None, "the filter must match what the selector shows");
    assert_eq!(page.filtered_versions(), vec![0]);
}

#[test]
fn the_page_names_the_project_by_its_details_once_they_load() {
    let mut listing = listing();
    listing.name = "sodium-fabric-0.5.8".to_string();
    let mut page = ResourcePage::default();
    page.show(listing, None);
    assert_eq!(page.name(), Some("sodium-fabric-0.5.8"));
    let generation = page.generation();
    page.update(Message::Details {
        generation,
        result: Ok(Box::new(details(0))),
    });
    assert_eq!(page.name(), Some("Sodium"));
}

/// The details of a CurseForge project whose description is `html`.
fn html_details(html: &str) -> ContentDetails {
    ContentDetails {
        description: Some(html.to_string()),
        description_format: DescriptionFormat::Html,
        ..details(0)
    }
}

/// The picture addresses the effects ask for, in order.
fn asked_pictures(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .flat_map(|effect| match effect {
            Effect::Pictures(urls) => urls.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// A description with a picture before a spoiler, one inside it, and one after it.
const DESCRIPTION: &str = "<p><img src=\"https://media.forgecdn.net/a.png\"></p>\
    <h3>Mods</h3><div class=\"spoiler\"><img src=\"//i.imgur.com/b.png\"></div>\
    <p><a href=\"/linkout?remoteUrl=https%253a%252f%252fdiscord.com%252finvite%252fx\"><img src=\"https://media.forgecdn.net/c.png\"></a></p>";

#[test]
fn an_html_description_is_read_and_its_visible_pictures_are_asked_for_in_reading_order() {
    let mut page = ResourcePage::default();
    page.show(listing(), None);
    let generation = page.generation();
    let effects = page.update(Message::Details {
        generation,
        result: Ok(Box::new(html_details(DESCRIPTION))),
    });
    assert!(page.readme.is_none(), "HTML is not read as markdown");
    let description = page.description.as_ref().expect("the description is read");
    assert_eq!(description.links, vec!["https://discord.com/invite/x".to_string()]);
    assert_eq!(
        asked_pictures(&effects),
        vec!["https://media.forgecdn.net/a.png", "https://media.forgecdn.net/c.png"],
        "the spoiler's picture waits until it is opened"
    );
}

#[test]
fn opening_a_spoiler_asks_for_its_pictures_once() {
    let mut page = ResourcePage::default();
    page.show(listing(), None);
    let generation = page.generation();
    page.update(Message::Details {
        generation,
        result: Ok(Box::new(html_details(DESCRIPTION))),
    });
    let opened = page.update(Message::Spoiler(0));
    assert!(page.spoilers.contains(&0));
    assert_eq!(asked_pictures(&opened), vec!["https://i.imgur.com/b.png"]);
    let closed = page.update(Message::Spoiler(0));
    assert!(!page.spoilers.contains(&0));
    assert!(closed.is_empty());
    assert!(page.update(Message::Spoiler(0)).is_empty(), "the picture was asked for already");
}

#[test]
fn a_markdown_readme_has_no_html_description() {
    let mut page = ResourcePage::default();
    page.show(listing(), None);
    let generation = page.generation();
    let mut readme = details(0);
    readme.description = Some("# Sodium".to_string());
    page.update(Message::Details {
        generation,
        result: Ok(Box::new(readme)),
    });
    assert!(page.readme.is_some());
    assert!(page.description.is_none());
}

#[test]
fn showing_another_project_forgets_the_description_and_its_open_spoilers() {
    let mut page = ResourcePage::default();
    page.show(listing(), None);
    let generation = page.generation();
    page.update(Message::Details {
        generation,
        result: Ok(Box::new(html_details(DESCRIPTION))),
    });
    page.update(Message::Spoiler(0));
    page.show(listing(), None);
    assert!(page.description.is_none());
    assert!(page.spoilers.is_empty());
}
