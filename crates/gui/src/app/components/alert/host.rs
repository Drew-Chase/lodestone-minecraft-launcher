//! The alert host — the root-owned queue behind every alert.
//!
//! Alerts queue rather than replace one another: a second alert waits behind
//! the first, so an earlier question is never silently answered for the
//! user. Each queued alert holds the sending half of a oneshot channel; the
//! task [`Alerts::show`] or [`Alerts::choose`] returns awaits the other half.

use std::collections::VecDeque;

use iced::{Element, Task};
use tokio::sync::oneshot;
use tracing::debug;

use super::{Alert, Choice, Message};

/// A shown or waiting alert and where its answer goes.
#[derive(Debug)]
struct Pending {
    alert: Alert,
    responder: oneshot::Sender<Choice>,
}

/// The alert queue. The front alert is the one on screen.
#[derive(Debug, Default)]
pub struct Alerts {
    queue: VecDeque<Pending>,
}

impl Alerts {
    /// Queues `alert` and returns a task that resolves with the answer:
    /// `true` for Yes, Okay or Close; `false` for No, Cancel, the extra
    /// button, Esc or ✕. If the alert is dropped unanswered, it resolves
    /// `false`.
    ///
    /// ```text
    /// Message::DeleteInstance => self
    ///     .alerts
    ///     .show(Alert::danger("Delete “Aether Legacy”? This cannot be undone.").title("Delete instance"))
    ///     .map(Message::DeleteConfirmed),
    /// ```
    pub fn show(&mut self, alert: Alert) -> Task<bool> {
        self.choose(alert).map(|choice| choice == Choice::Affirmative)
    }

    /// Queues `alert` and returns a task that resolves with the button it was answered with. An alert that is dropped
    /// unanswered resolves [`Choice::Negative`].
    pub fn choose(&mut self, alert: Alert) -> Task<Choice> {
        let answer = self.push(alert);
        Task::perform(answer, |answer| answer.unwrap_or(Choice::Negative))
    }

    /// Queues `alert` and hands back the receiver for its answer.
    pub(crate) fn push(&mut self, alert: Alert) -> oneshot::Receiver<Choice> {
        let (responder, answer) = oneshot::channel();
        debug!("Alert queued: {}", alert.heading());
        self.queue.push_back(Pending { alert, responder });
        answer
    }

    /// Answers the alert on screen and shows the next one, if any.
    pub fn respond(&mut self, choice: Choice) {
        let Some(pending) = self.queue.pop_front() else {
            debug!("Alert response {choice:?} ignored: no alert is open");
            return;
        };
        debug!("Alert answered {choice:?}: {}", pending.alert.heading());
        // The receiver is gone only when the awaiting task was dropped, in
        // which case nobody needs the answer.
        let _ = pending.responder.send(choice);
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Respond(choice) => self.respond(choice),
        }
    }

    /// The alert on screen, if any.
    pub fn current(&self) -> Option<&Alert> {
        self.queue.front().map(|pending| &pending.alert)
    }

    pub fn is_open(&self) -> bool {
        !self.queue.is_empty()
    }

    /// The modal for the alert on screen.
    pub fn view(&self) -> Option<Element<'_, Message>> {
        self.current().map(Alert::view)
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/alert/host.rs"]
mod tests;
