use super::*;
use crate::app::pages::discover::{Message, TABS};

/// An instance choice with the given loader and version.
fn choice(id: &str, loader: Loader, mc_version: &str) -> InstanceChoice {
    InstanceChoice {
        id: id.to_string(),
        name: format!("Instance {id}"),
        loader,
        mc_version: mc_version.to_string(),
    }
}

/// A Modrinth mod, as a card asks for it.
fn project(project_id: &str) -> Project {
    Project {
        source: Source::Modrinth,
        content: ContentType::Mod,
        project_id: project_id.to_string(),
        name: "Sodium".to_string(),
        web_url: None,
        icon_url: None,
        game_version: None,
        version_id: None,
    }
}

/// The record a finished add leaves behind.
fn record(instance: &str, project_id: &str) -> ModSource {
    ModSource {
        instance: instance.to_string(),
        platform: "modrinth".to_string(),
        project_id: project_id.to_string(),
        version_id: "v1".to_string(),
        name: "Sodium".to_string(),
        file_name: "sodium.jar".to_string(),
        sha1: None,
    }
}

/// A page on the Mods tab with one instance, picked.
fn mods_page() -> Discover {
    let mut page = Discover::new(Source::Modrinth);
    page.update(Message::ContentSelected(ContentType::Mod));
    page.set_instances(vec![choice("a", Loader::Fabric, "1.21.1")]);
    page.update(Message::InstanceSelected(0));
    page
}

#[test]
fn the_mods_tab_sits_between_the_modpacks_and_the_resource_packs() {
    assert_eq!(TABS[0], ContentType::Modpack);
    assert_eq!(TABS[1], ContentType::Mod);
    assert_eq!(TABS[2], ContentType::ResourcePack);
}

#[test]
fn picking_an_instance_sets_its_loader_and_version_as_the_filters() {
    let mut page = Discover::new(Source::Modrinth);
    page.update(Message::ContentSelected(ContentType::Mod));
    page.set_instances(vec![choice("a", Loader::Fabric, "1.21.1")]);
    let effects = page.update(Message::InstanceSelected(0));
    assert_eq!(page.target.as_deref(), Some("a"));
    assert_eq!(page.loader.as_deref(), Some("fabric"));
    assert_eq!(page.game_version.as_deref(), Some("1.21.1"));
    assert!(
        effects.iter().any(|effect| matches!(effect, Effect::Search(_))),
        "the filtered search starts"
    );
}

#[test]
fn a_vanilla_instance_takes_no_mods_and_filters_by_no_loader() {
    let vanilla = choice("v", Loader::Vanilla, "1.21.1");
    assert!(!vanilla.takes_mods());
    assert_eq!(vanilla.loader_filter(), None);
    assert!(choice("f", Loader::Fabric, "1.21.1").takes_mods());
}

#[test]
fn the_filters_of_another_tab_are_left_alone_by_the_instance() {
    let mut page = Discover::new(Source::Modrinth);
    page.set_instances(vec![choice("a", Loader::Fabric, "1.21.1")]);
    page.update(Message::InstanceSelected(0));
    assert_eq!(page.loader, None, "the modpack tab keeps its own filters");
    assert_eq!(page.game_version, None);
}

#[test]
fn adding_a_mod_needs_an_instance_that_takes_mods_and_starts_once() {
    let mut page = Discover::new(Source::Modrinth);
    page.update(Message::ContentSelected(ContentType::Mod));
    page.set_instances(vec![choice("a", Loader::Fabric, "1.21.1")]);
    assert!(page.add_mod(project("p1")).is_empty(), "no instance is picked yet");
    page.update(Message::InstanceSelected(0));
    let effects = page.add_mod(project("p1"));
    assert!(
        matches!(effects.as_slice(), [Effect::AddMod(request)] if request.instance.id == "a" && request.project.project_id == "p1"),
        "the mod goes to the picked instance"
    );
    assert!(page.add_mod(project("p1")).is_empty(), "a mod already being added is not added twice");
}

#[test]
fn a_vanilla_instance_refuses_mods() {
    let mut page = Discover::new(Source::Modrinth);
    page.update(Message::ContentSelected(ContentType::Mod));
    page.set_instances(vec![choice("v", Loader::Vanilla, "1.21.1")]);
    page.update(Message::InstanceSelected(0));
    assert!(page.add_mod(project("p1")).is_empty());
}

#[test]
fn a_finished_add_is_remembered_and_recorded_and_then_refused_again() {
    let mut page = mods_page();
    page.adding.insert(("a".to_string(), "modrinth".to_string(), "p1".to_string()));
    let effects = page.mod_added("a".to_string(), "modrinth".to_string(), "p1".to_string(), Ok(vec![record("a", "p1")]));
    assert!(matches!(effects.as_slice(), [Effect::RecordModSources(saved)] if *saved == vec![record("a", "p1")]));
    assert!(page.added_to_target("p1"));
    assert!(!page.adding_to_target("p1"));
    assert!(page.add_mod(project("p1")).is_empty(), "a mod already in the instance is not added again");
}

#[test]
fn the_dependencies_an_add_wrote_count_as_added_too() {
    let mut page = mods_page();
    page.adding.insert(("a".to_string(), "modrinth".to_string(), "p1".to_string()));
    let dependency = ModSource {
        project_id: "p2".to_string(),
        file_name: "fabric-api.jar".to_string(),
        ..record("a", "p2")
    };
    page.mod_added(
        "a".to_string(),
        "modrinth".to_string(),
        "p1".to_string(),
        Ok(vec![record("a", "p1"), dependency]),
    );
    assert!(page.added_to_target("p2"), "a dependency the add wrote is in the instance");
}

#[test]
fn a_failed_add_is_reported_and_can_be_tried_again() {
    let mut page = mods_page();
    page.adding.insert(("a".to_string(), "modrinth".to_string(), "p1".to_string()));
    let effects = page.mod_added(
        "a".to_string(),
        "modrinth".to_string(),
        "p1".to_string(),
        Err("no release fits".to_string()),
    );
    assert!(matches!(effects.as_slice(), [Effect::Notify(error)] if error == "no release fits"));
    assert!(!page.added_to_target("p1"));
    assert!(!page.add_mod(project("p1")).is_empty());
}

#[test]
fn mods_saved_by_an_earlier_launch_show_as_added() {
    let mut page = mods_page();
    page.set_added(&[record("a", "p1")]);
    assert!(page.added_to_target("p1"));
    assert!(!page.added_to_target("p2"));
}

#[test]
fn the_target_is_forgotten_when_its_instance_is_no_longer_listed() {
    let mut page = mods_page();
    page.set_instances(vec![choice("b", Loader::Forge, "1.20.1")]);
    assert_eq!(page.target, None);
    assert!(page.target().is_none());
}

#[test]
fn preparing_mods_from_an_instance_moves_to_a_platform_that_hosts_them() {
    let mut page = Discover::new(Source::Ftb);
    page.prepare_mods(choice("a", Loader::Forge, "1.20.1"));
    assert_eq!(page.source, Source::Modrinth);
    assert_eq!(page.content, ContentType::Mod);
    assert_eq!(page.target.as_deref(), Some("a"));
    assert_eq!(page.loader.as_deref(), Some("forge"));
    assert_eq!(page.game_version.as_deref(), Some("1.20.1"));
}
