//! Watches one folder for the Files tab, so the list follows files that come and go while it is on screen: a game
//! writing a config, a download landing, a file manager moving something in.
//!
//! Only the folder itself is watched, not what is below it, since the list shows one level. Events come in bursts, so
//! they are gathered and reported once the folder has been quiet for a moment.

use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::futures::channel::mpsc::Sender;
use iced::futures::{SinkExt, Stream};
use notify::{RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::warn;

use super::Message;
use crate::app::pages::instance::watch::relevant;

/// How long the folder must stay quiet before a change is reported.
const QUIET: Duration = Duration::from_millis(300);
/// The longest a report may wait, so a folder written without pause still refreshes.
const LONGEST: Duration = Duration::from_millis(1500);

/// The changes in the folder at `folder`, as messages. The folder stays watched for as long as the subscription lives.
#[allow(clippy::ptr_arg, reason = "Subscription::run_with hands its builder the data by reference, as a PathBuf")]
pub fn subscription(folder: &PathBuf) -> impl Stream<Item = Message> + use<> {
    let folder = folder.clone();
    iced::stream::channel(4, async move |mut output| {
        if let Err(error) = run(&folder, &mut output).await {
            warn!("Watching {} failed: {error}", folder.display());
        }
    })
}

/// Watches `folder` and reports each burst of changes once it ends, until the subscription is dropped.
async fn run(folder: &Path, output: &mut Sender<Message>) -> notify::Result<()> {
    let (events_tx, mut events) = mpsc::unbounded_channel::<notify::Event>();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event {
            // The receiver is gone only once the watch has ended, when nothing needs the event.
            let _ = events_tx.send(event);
        }
    })?;
    watcher.watch(folder, RecursiveMode::NonRecursive)?;

    let mut burst = Burst::default();
    loop {
        let deadline = burst.deadline();
        let quiet = async move {
            match deadline {
                Some(at) => tokio::time::sleep_until(at).await,
                None => std::future::pending::<()>().await,
            }
        };
        tokio::select! {
            event = events.recv() => {
                let Some(event) = event else {
                    break;
                };
                if relevant(&event.kind) {
                    burst.push(Instant::now());
                }
            }
            () = quiet => {
                burst = Burst::default();
                if output.send(Message::Changed(folder.to_path_buf())).await.is_err() {
                    break;
                }
            }
        }
    }
    Ok(())
}

/// When the burst of events being gathered began, and when it last grew.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Burst {
    first: Option<Instant>,
    latest: Option<Instant>,
}

impl Burst {
    /// Records an event seen at `now`.
    fn push(&mut self, now: Instant) {
        self.first.get_or_insert(now);
        self.latest = Some(now);
    }

    /// When the burst is reported: after a quiet spell of [`QUIET`], and no later than [`LONGEST`] after it began.
    /// `None` while no event has been seen.
    fn deadline(&self) -> Option<Instant> {
        let (first, latest) = (self.first?, self.latest?);
        Some((latest + QUIET).min(first + LONGEST))
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/browser/watch.rs"]
mod tests;
