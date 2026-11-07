use super::super::Message;
use super::super::target::{InstanceChoice, WorldChoice};
use super::*;
use crate::app::pages::library::instance::Loader;
use crate::app::shared_library::AddedContent;

/// An instance choice with the given version.
fn choice(id: &str, mc_version: &str) -> InstanceChoice {
    InstanceChoice {
        id: id.to_string(),
        name: format!("Instance {id}"),
        loader: Loader::Fabric,
        mc_version: mc_version.to_string(),
    }
}

/// A page on `content` with instance "a" picked as the target.
fn page_on(content: ContentType) -> Discover {
    let mut page = Discover::new(Source::Modrinth);
    page.update(Message::ContentSelected(content));
    page.set_instances(vec![choice("a", "1.21.1")]);
    page.update(Message::InstanceSelected(0));
    page
}

/// The request the effects place, when they place one.
fn placed(effects: &[Effect]) -> Option<&PlaceRequest> {
    effects.iter().find_map(|effect| match effect {
        Effect::Place(request) => Some(request.as_ref()),
        _ => None,
    })
}

/// The record a placement into `instance` as `file_name` leaves behind.
fn added(instance: &str, scope: &str, file_name: &str) -> AddedContent {
    AddedContent {
        instance: instance.to_string(),
        scope: scope.to_string(),
        kind: "resourcepacks".to_string(),
        platform: "modrinth".to_string(),
        project_id: "p1".to_string(),
        version_id: "v1".to_string(),
        name: "Pack p1".to_string(),
        file_name: file_name.to_string(),
    }
}

#[test]
fn a_pack_is_placed_into_the_target_instance_and_only_once() {
    let mut page = page_on(ContentType::ResourcePack);
    let effects = page.update(Message::Place("p1".to_string()));
    let request = placed(&effects).expect("the pack is placed");
    assert_eq!(request.destination, Destination::Pack { instance: "a".to_string() });
    assert_eq!(request.minecraft, "1.21.1");
    assert_eq!(request.project.project_id, "p1");
    assert!(
        page.update(Message::Place("p1".to_string())).is_empty(),
        "a pack on its way is not placed again"
    );
}

#[test]
fn nothing_is_placed_without_a_target_instance() {
    let mut page = Discover::new(Source::Modrinth);
    page.update(Message::ContentSelected(ContentType::Shader));
    assert!(page.update(Message::Place("p1".to_string())).is_empty());
}

#[test]
fn a_datapack_waits_for_its_world_and_then_goes_into_it() {
    let mut page = Discover::new(Source::Modrinth);
    page.update(Message::ContentSelected(ContentType::DataPack));
    page.set_instances(vec![choice("a", "1.21.1")]);
    let effects = page.update(Message::InstanceSelected(0));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::LoadWorlds(instance) if instance == "a")),
        "the worlds of the instance are read"
    );
    assert!(page.update(Message::Place("p1".to_string())).is_empty(), "no world is picked yet");

    page.update(Message::WorldsLoaded {
        instance: "a".to_string(),
        worlds: vec![WorldChoice {
            folder: "one".to_string(),
            name: "World One".to_string(),
        }],
    });
    page.update(Message::WorldSelected(0));
    let effects = page.update(Message::Place("p1".to_string()));
    let request = placed(&effects).expect("the datapack is placed");
    assert_eq!(
        request.destination,
        Destination::Datapacks {
            instance: "a".to_string(),
            world: "one".to_string(),
        }
    );
    assert_eq!(page.target_world_name(), Some("World One"));
}

#[test]
fn a_world_is_imported_into_the_target_instance() {
    // Modrinth hosts no worlds, so the page is on CurseForge, which does.
    let mut page = Discover::new(Source::CurseForge);
    page.update(Message::ContentSelected(ContentType::World));
    page.set_instances(vec![choice("a", "1.21.1")]);
    page.update(Message::InstanceSelected(0));
    let effects = page.update(Message::Place("p1".to_string()));
    assert_eq!(
        placed(&effects).expect("the world is imported").destination,
        Destination::World { instance: "a".to_string() }
    );
}

#[test]
fn a_finished_placement_is_kept_recorded_and_refused_again() {
    let mut page = page_on(ContentType::ResourcePack);
    let effects = page.update(Message::Place("p1".to_string()));
    let key = placed(&effects).expect("the pack is placed").placement();
    let outcome = PlaceOutcome {
        added: added("a", "", "pack.zip"),
        shared: None,
    };
    let effects = page.update(Message::Placed {
        key,
        result: Ok(Box::new(outcome)),
    });
    assert!(matches!(effects.as_slice(), [Effect::RecordAdded(record)] if record.file_name == "pack.zip"));
    assert!(page.placed_in_target("p1").is_some());
    assert!(!page.placing_in_target("p1"));
    assert!(
        page.update(Message::Place("p1".to_string())).is_empty(),
        "a pack already there is not placed again"
    );
    let status = page.status_of(Source::Modrinth, ContentType::ResourcePack, "p1");
    assert!(status.added);
    assert_eq!(status.added_version.as_deref(), Some("v1"));
}

#[test]
fn a_failed_placement_is_reported_and_can_be_tried_again() {
    let mut page = page_on(ContentType::ResourcePack);
    let effects = page.update(Message::Place("p1".to_string()));
    let key = placed(&effects).expect("the pack is placed").placement();
    let effects = page.update(Message::Placed {
        key,
        result: Err("no release fits".to_string()),
    });
    assert!(matches!(effects.as_slice(), [Effect::Notify(error)] if error == "no release fits"));
    assert!(page.placed_in_target("p1").is_none());
    assert!(!page.update(Message::Place("p1".to_string())).is_empty());
}

#[test]
fn a_world_picked_that_the_instance_no_longer_has_is_forgotten() {
    let mut page = page_on(ContentType::DataPack);
    page.update(Message::WorldsLoaded {
        instance: "a".to_string(),
        worlds: vec![WorldChoice {
            folder: "one".to_string(),
            name: "World One".to_string(),
        }],
    });
    page.update(Message::WorldSelected(0));
    page.update(Message::WorldsLoaded {
        instance: "a".to_string(),
        worlds: Vec::new(),
    });
    assert_eq!(page.target_world, None);
}

#[test]
fn the_records_the_database_holds_show_as_placed() {
    let mut page = page_on(ContentType::ResourcePack);
    page.set_placed(&[added("a", "", "pack.zip")]);
    assert!(page.placed_in_target("p1").is_some());
    let other = added("b", "", "pack.zip");
    page.set_placed(&[other]);
    assert!(page.placed_in_target("p1").is_none(), "a pack in another instance is not this one");
}
