//! The clipboard for copied images and files. `arboard` on Linux serves the X11 selection from the `Clipboard` that set
//! it, so what was copied goes when that object does. One thread keeps a `Clipboard` for the launcher's lifetime, and
//! every copy and read is sent to it, so an image or a list of files stays on the clipboard after it is copied.

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;

use arboard::{Clipboard, ImageData};
use tokio::sync::oneshot;

/// The message for a clipboard thread that has stopped.
const STOPPED: &str = "the clipboard thread has stopped";

/// One request waiting for the clipboard thread, and where its outcome goes.
pub enum Job {
    /// Sets an image as the clipboard's content.
    Image {
        image: ImageData<'static>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    /// Sets a list of files as the clipboard's content, the way a file manager's Copy does.
    SetFiles {
        paths: Vec<PathBuf>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    /// Reads the list of files the clipboard holds. A clipboard that holds something else is an error.
    GetFiles { reply: oneshot::Sender<Result<Vec<PathBuf>, String>> },
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
    sender.send(Job::Image { image, reply }).map_err(|_| STOPPED.to_string())?;
    outcome.await.map_err(|_| STOPPED.to_string())?
}

/// Puts the files at `paths` on the clipboard, and resolves with the outcome.
pub async fn copy_files(sender: Sender<Job>, paths: Vec<PathBuf>) -> Result<(), String> {
    let (reply, outcome) = oneshot::channel();
    sender.send(Job::SetFiles { paths, reply }).map_err(|_| STOPPED.to_string())?;
    outcome.await.map_err(|_| STOPPED.to_string())?
}

/// Reads the files the clipboard holds. Resolves with an error when it holds none, or cannot be read.
pub async fn read_files(sender: Sender<Job>) -> Result<Vec<PathBuf>, String> {
    let (reply, outcome) = oneshot::channel();
    sender.send(Job::GetFiles { reply }).map_err(|_| STOPPED.to_string())?;
    outcome.await.map_err(|_| STOPPED.to_string())?
}

/// Starts the clipboard thread. It opens the clipboard on its first job, and tries again on a later job
/// when opening it failed.
fn spawn() -> Sender<Job> {
    let (sender, jobs) = mpsc::channel::<Job>();
    thread::spawn(move || {
        let mut clipboard = None;
        for job in jobs {
            // A job's caller may have given up, in which case nobody needs the outcome.
            match job {
                Job::Image { image, reply } => {
                    let _ = reply.send(with(&mut clipboard, |clipboard| clipboard.set_image(image)));
                }
                Job::SetFiles { paths, reply } => {
                    let _ = reply.send(with(&mut clipboard, |clipboard| clipboard.set().file_list(&paths)));
                }
                Job::GetFiles { reply } => {
                    let _ = reply.send(with(&mut clipboard, |clipboard| clipboard.get().file_list()));
                }
            }
        }
    });
    sender
}

/// Runs `job` on the clipboard, opening it first when it is not open.
fn with<T>(clipboard: &mut Option<Clipboard>, job: impl FnOnce(&mut Clipboard) -> Result<T, arboard::Error>) -> Result<T, String> {
    if clipboard.is_none() {
        *clipboard = Some(Clipboard::new().map_err(|error| error.to_string())?);
    }
    match clipboard {
        Some(clipboard) => job(clipboard).map_err(|error| error.to_string()),
        None => Err("the clipboard is not open".to_string()),
    }
}
