//! The clipboard for copied images. `arboard` on Linux serves the X11 selection from the `Clipboard`
//! that set it, so an image goes when that object does. One thread keeps a `Clipboard` for the
//! launcher's lifetime, and copies are sent to it, so the image stays on the clipboard after it is copied.

use std::sync::mpsc::{self, Sender};
use std::thread;

use arboard::{Clipboard, ImageData};
use tokio::sync::oneshot;

/// The message for a clipboard thread that has stopped.
const STOPPED: &str = "the clipboard thread has stopped";

/// One image waiting for the clipboard thread, and where its outcome goes.
pub struct Job {
    image: ImageData<'static>,
    reply: oneshot::Sender<Result<(), String>>,
}

/// The way to the clipboard thread. The thread starts on the first copy.
#[derive(Default)]
pub struct Worker {
    sender: Option<Sender<Job>>,
}

impl Worker {
    /// A sender to the clipboard thread, started now when it is not running.
    pub fn sender(&mut self) -> Sender<Job> {
        self.sender.get_or_insert_with(spawn).clone()
    }
}

/// Sends `image` to the clipboard thread and resolves with the outcome.
pub async fn copy(sender: Sender<Job>, image: ImageData<'static>) -> Result<(), String> {
    let (reply, outcome) = oneshot::channel();
    sender.send(Job { image, reply }).map_err(|_| STOPPED.to_string())?;
    outcome.await.map_err(|_| STOPPED.to_string())?
}

/// Starts the clipboard thread. It opens the clipboard on its first job, and tries again on a later job
/// when opening it failed.
fn spawn() -> Sender<Job> {
    let (sender, jobs) = mpsc::channel::<Job>();
    thread::spawn(move || {
        let mut clipboard = None;
        for Job { image, reply } in jobs {
            let outcome = write(&mut clipboard, image);
            // The copy's caller may have given up, in which case nobody needs the outcome.
            let _ = reply.send(outcome);
        }
    });
    sender
}

/// Sets `image` as the clipboard's image, opening the clipboard first when it is not open.
fn write(clipboard: &mut Option<Clipboard>, image: ImageData<'static>) -> Result<(), String> {
    if clipboard.is_none() {
        *clipboard = Some(Clipboard::new().map_err(|error| error.to_string())?);
    }
    match clipboard {
        Some(clipboard) => clipboard.set_image(image).map_err(|error| error.to_string()),
        None => Err("the clipboard is not open".to_string()),
    }
}
