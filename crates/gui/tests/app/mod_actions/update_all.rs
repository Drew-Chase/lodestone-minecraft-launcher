use super::*;

fn queued(file_name: &str, name: &str) -> Queued {
    Queued {
        file_name: file_name.to_string(),
        name: name.to_string(),
    }
}

fn queue() -> UpdateQueue {
    UpdateQueue::new(
        3,
        vec![
            queued("sodium.jar", "Sodium"),
            queued("lithium.jar", "Lithium"),
            queued("iris.jar.disabled", "Iris"),
        ],
    )
}

#[test]
fn the_mods_are_updated_in_the_order_they_were_queued() {
    let mut queue = queue();
    let order: Vec<String> = std::iter::from_fn(|| queue.next()).map(|next| next.file_name).collect();
    assert_eq!(order, ["sodium.jar", "lithium.jar", "iris.jar.disabled"]);
    assert_eq!(queue.next(), None, "an empty queue has no next mod");
}

#[test]
fn a_queue_with_no_failure_ends_without_a_summary() {
    let mut queue = queue();
    while queue.next().is_some() {}
    assert_eq!(queue.summary(), None);
}

#[test]
fn the_summary_names_each_mod_that_failed_with_its_reason() {
    let mut queue = queue();
    queue.fail("Sodium".to_string(), "no release".to_string());
    queue.fail("Iris".to_string(), "timed out".to_string());
    let summary = queue.summary().expect("two mods failed");
    let lines: Vec<&str> = summary.lines().collect();
    assert_eq!(lines.len(), 3, "a heading, then one line per mod: {summary}");
    assert!(lines[0].contains('2'), "the heading counts the failures: {}", lines[0]);
    assert!(lines[1].contains("Sodium") && lines[1].contains("no release"));
    assert!(lines[2].contains("Iris") && lines[2].contains("timed out"));
}

#[test]
fn failing_the_rest_empties_the_queue_and_names_every_waiting_mod() {
    let mut queue = queue();
    let _ = queue.next();
    let released = queue.fail_rest("the game is running");
    assert_eq!(released, ["lithium.jar", "iris.jar.disabled"]);
    assert_eq!(queue.next(), None);
    assert_eq!(
        queue.failed,
        [
            Failure {
                name: "Lithium".to_string(),
                error: "the game is running".to_string(),
            },
            Failure {
                name: "Iris".to_string(),
                error: "the game is running".to_string(),
            },
        ]
    );
}
