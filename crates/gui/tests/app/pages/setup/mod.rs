//! Tests for `pages/setup/mod.rs`: the order of the steps, and the moves the wizard makes between them.

use super::*;

#[test]
fn the_steps_run_in_order_from_sign_in_to_done() {
    let mut step = Step::SignIn;
    let mut order = vec![step];
    while step != Step::Done {
        step = step.next();
        order.push(step);
    }
    assert_eq!(order, [Step::SignIn, Step::Folders, Step::Import, Step::Discover, Step::Done]);
}

#[test]
fn back_undoes_next_and_the_sign_in_is_its_own_previous_step() {
    assert_eq!(Step::SignIn.prev(), Step::SignIn);
    for step in [Step::Folders, Step::Import, Step::Discover] {
        assert_eq!(step.next().prev(), step);
    }
    assert_eq!(Step::Done.prev(), Step::Discover);
}

#[test]
fn the_stepper_index_counts_the_listed_steps_and_done_lies_past_them() {
    assert_eq!(Step::SignIn.index(), 0);
    assert_eq!(Step::Discover.index(), 3);
    assert_eq!(Step::Done.index(), Step::LISTED.len());
}

#[test]
fn next_and_back_move_the_wizard_and_skip_and_finish_leave_it_in_place() {
    let mut setup = Setup::default();
    setup.update(Message::Next);
    assert_eq!(setup.step(), Step::Folders);
    setup.update(Message::Back);
    assert_eq!(setup.step(), Step::SignIn);
    setup.update(Message::Skip);
    setup.update(Message::Finish);
    assert_eq!(setup.step(), Step::SignIn);
}
