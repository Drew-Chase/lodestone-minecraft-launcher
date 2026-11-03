//! The Skins page: the player's skin and cape in 3D, the skins to choose from, and the capes the player owns.
//!
//! The page holds what it shows. The root reads the folders and the profile and hands the results in, and it carries out
//! what the page asks for, since uploads, cape changes and moves to the trash are the root's to start and confirm.

pub mod archive;
pub mod capes;
pub mod catalog;
pub mod library;
pub mod previews;
mod view;
pub mod viewer;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::Element;
use iced::widget::image::Handle;
use lodestone_minecraft_authentication::Cape;

use self::catalog::{BUNDLED, Pack};
use self::library::SavedSkin;
use self::previews::{Item, Job, Previews};
use self::viewer::{Layers, Pose, Skin};
use crate::app::pages::Page;
use crate::app::theme::effects;

/// How often the skin previews take a step through the poses' motion. The viewer draws its own frames faster.
pub const PREVIEW_FRAME: Duration = Duration::from_millis(66);

/// The skin and cape the account wears, and the model it wears them in.
#[derive(Clone)]
pub struct Appearance {
    pub skin_png: Vec<u8>,
    pub cape_png: Option<Vec<u8>>,
    pub slim: bool,
}

impl fmt::Debug for Appearance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The bytes are left out, as the skins' are.
        formatter
            .debug_struct("Appearance")
            .field("slim", &self.slim)
            .field("cape", &self.cape_png.is_some())
            .finish()
    }
}

/// What the folders hold, read off the UI thread.
#[derive(Debug, Clone)]
pub struct Scan {
    pub saved: Vec<SavedSkin>,
    pub archived: Vec<SavedSkin>,
    /// The digests of the bundled skins. The archive does not keep a skin that the bundle already has.
    pub bundled: Vec<String>,
}

/// A skin the grid shows, as the page refers to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// The skin the account wears.
    Current,
    /// A saved skin, by its place in the saved list.
    Saved(usize),
    /// An archived skin, by its place in the archive.
    Archived(usize),
    /// A bundled skin, by its place in the catalog.
    Bundled(usize),
}

/// The sections of the page. Each pack of bundled skins is a section of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Saved,
    Archived,
    Pack(Pack),
}

/// One of the overlay layers the viewer draws over the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Hat,
    Jacket,
    Sleeves,
    Pants,
}

/// A skin the page would upload: its name for the question that confirms it, its PNG, and its model.
pub struct Chosen {
    pub name: String,
    pub png: Vec<u8>,
    pub slim: bool,
}

/// The account the page shows: whether it signed in with Microsoft, which may change its skin and cape, and the capes it owns.
#[derive(Debug, Clone, Copy)]
pub struct Account<'a> {
    pub microsoft: bool,
    pub capes: &'a [Cape],
}

/// Messages the page emits and handles. The root handles the ones that start an upload, a cape change, a move to the trash,
/// a reveal or a navigation.
#[derive(Debug, Clone)]
pub enum Message {
    /// Shows a skin in the viewer.
    Choose(Choice),
    /// Changes the pose.
    Pose(Pose),
    /// Shows or hides an overlay layer.
    Layer(Layer),
    /// Chooses the classic or the slim model, for a saved skin.
    Slim(bool),
    /// Opens or shuts a section.
    ToggleSection(Section),
    /// Asks to use the chosen skin. The root confirms it, then uploads it.
    Use,
    /// Asks to move the archived skin at this place to the trash. The root confirms it.
    RemoveArchived(usize),
    /// Asks to wear the cape with this id, or to take the cape off when `None`. The root confirms it.
    Cape(Option<String>),
    /// Opens the folder the saved skins live in. The root reveals it.
    OpenFolder,
    /// Opens another page, such as Settings, where a Microsoft account signs in.
    Navigate(Page),
    /// A step of the skin previews' clock, at the moment it fell due.
    Tick(Instant),
}

/// The page's state.
pub struct Skins {
    appearance: Option<Appearance>,
    /// The skin the account wears, composed with its cape, as the viewer samples it.
    current: Option<Arc<Skin>>,
    saved: Vec<SavedSkin>,
    archived: Vec<SavedSkin>,
    /// The digests the archive does not keep again: those of the saved skins and of the bundled ones.
    known: Vec<String>,
    /// The fronts of the capes the player owns, by cape id.
    cape_faces: HashMap<String, Handle>,
    choice: Choice,
    /// The chosen skin as the viewer shows it, with the cape the account wears. `None` while the current skin is chosen.
    preview: Option<Arc<Skin>>,
    pose: Pose,
    layers: Layers,
    /// The model the saved skins show in, which the switch sets.
    slim: bool,
    collapsed: HashSet<Section>,
    /// Whether a change to the profile is under way. The page offers no other change until it ends.
    busy: bool,
    /// The 3D previews of the saved, archived and bundled skins, drawn in the pose on show.
    previews: Previews,
    /// Whether the previews must be drawn again, and the generation of the last drawing asked for, so that an answer to an
    /// older drawing is dropped.
    previews_stale: bool,
    /// The generation of the last drawing asked for, and of the last one shown.
    preview_generation: u64,
    drawn_generation: u64,
    /// Whether a drawing is running, so that the next one waits for it.
    previews_busy: bool,
    /// The moment the previews' clock started, the seconds of motion it has run, and whether a frame of it has fallen due.
    clock_start: Option<Instant>,
    clock: f32,
    frame_due: bool,
}

impl Default for Skins {
    fn default() -> Self {
        Self {
            appearance: None,
            current: None,
            saved: Vec::new(),
            archived: Vec::new(),
            known: Vec::new(),
            cape_faces: HashMap::new(),
            choice: Choice::Current,
            preview: None,
            pose: Pose::default(),
            layers: Layers::default(),
            slim: false,
            // The packs start folded away, so the page opens on the saved skins and the vanilla defaults.
            collapsed: Pack::ALL
                .iter()
                .filter(|pack| **pack != Pack::Defaults)
                .map(|pack| Section::Pack(*pack))
                .collect(),
            busy: false,
            previews: Previews::default(),
            previews_stale: false,
            preview_generation: 0,
            drawn_generation: 0,
            previews_busy: false,
            clock_start: None,
            clock: 0.0,
            frame_due: false,
        }
    }
}

impl Skins {
    /// Takes what the folders hold. A choice that no longer names a skin goes back to the current one.
    pub fn set_scan(&mut self, scan: Scan) {
        self.saved = scan.saved;
        self.archived = scan.archived;
        self.known = self.saved.iter().map(|skin| skin.digest.clone()).chain(scan.bundled).collect();
        if !self.exists(self.choice) {
            self.choose(Choice::Current);
        }
        self.previews_stale = true;
    }

    /// Takes the account's skin and cape, decoded for the viewer. The chosen skin is shown with the cape the account wears.
    pub fn set_appearance(&mut self, appearance: Appearance) {
        self.current = Skin::compose(&appearance.skin_png, appearance.cape_png.as_deref()).map(Arc::new);
        self.appearance = Some(appearance);
        self.choose(self.choice);
        self.previews_stale = true;
    }

    /// Notes a frame of the poses' motion, at `now`. The previews are drawn again at the next opportunity.
    pub fn tick(&mut self, now: Instant) {
        let start = *self.clock_start.get_or_insert(now);
        self.clock = now.saturating_duration_since(start).as_secs_f32();
        self.frame_due = true;
    }

    /// The job that draws the previews again: when the list, the pose, the layers, the account or the open sections changed, or
    /// a frame is due. Only one drawing runs at a time, so `None` while one is running or nothing has changed. Under reduced
    /// motion the poses are held, so a frame is never due.
    pub fn preview_job(&mut self) -> Option<Job> {
        if self.previews_busy || !(self.previews_stale || self.frame_due) {
            return None;
        }
        self.previews_stale = false;
        self.frame_due = false;
        self.previews_busy = true;
        self.preview_generation += 1;
        let time = if effects::ambient() {
            self.clock
        } else {
            self.pose.preview_time()
        };
        let saved_open = !self.collapsed(Section::Saved);
        let archived_open = !self.collapsed(Section::Archived);
        Some(Job {
            generation: self.preview_generation,
            pose: self.pose,
            time,
            layers: self.layers,
            cape: self.appearance.as_ref().and_then(|appearance| appearance.cape_png.clone()),
            saved_len: self.saved.len(),
            archived_len: self.archived.len(),
            saved: if saved_open {
                self.saved
                    .iter()
                    .enumerate()
                    .map(|(index, skin)| {
                        (
                            index,
                            Item {
                                texels: skin.texels.clone(),
                                slim: self.slim,
                            },
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            },
            archived: if archived_open {
                self.archived
                    .iter()
                    .enumerate()
                    .map(|(index, skin)| {
                        (
                            index,
                            Item {
                                texels: skin.texels.clone(),
                                slim: skin.slim.unwrap_or(false),
                            },
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            },
            bundled: BUNDLED
                .iter()
                .enumerate()
                .filter(|(_, skin)| !self.collapsed(Section::Pack(skin.pack)))
                .map(|(index, skin)| (index, skin.slim))
                .collect(),
        })
    }

    /// Takes the previews a drawing made. Drawings finish in the order they were asked for, so each one replaces the last.
    pub fn set_previews(&mut self, generation: u64, previews: Previews) {
        self.previews_busy = false;
        if generation > self.drawn_generation {
            self.drawn_generation = generation;
            self.previews = previews;
        }
    }

    /// The 3D preview of the saved skin at `index`, when it has been drawn.
    pub fn saved_preview(&self, index: usize) -> Option<&Handle> {
        self.previews.saved.get(index).and_then(Option::as_ref)
    }

    /// The 3D preview of the archived skin at `index`, when it has been drawn.
    pub fn archived_preview(&self, index: usize) -> Option<&Handle> {
        self.previews.archived.get(index).and_then(Option::as_ref)
    }

    /// The 3D preview of the bundled skin at `index`, when it has been drawn.
    pub fn bundled_preview(&self, index: usize) -> Option<&Handle> {
        self.previews.bundled.get(index).and_then(Option::as_ref)
    }

    /// Takes the fronts of the capes the player owns, by cape id.
    pub fn set_cape_faces(&mut self, faces: Vec<(String, Handle)>) {
        self.cape_faces = faces.into_iter().collect();
    }

    /// Marks a change to the profile as under way, or finished.
    pub fn set_busy(&mut self, busy: bool) {
        self.busy = busy;
    }

    /// The account's skin and cape, as the page holds them.
    pub fn appearance(&self) -> Option<&Appearance> {
        self.appearance.as_ref()
    }

    /// The digests the archive does not keep again.
    pub fn known(&self) -> &[String] {
        &self.known
    }

    /// The file an archived skin lives in.
    pub fn archived_path(&self, index: usize) -> Option<PathBuf> {
        self.archived.get(index).map(|skin| skin.path.clone())
    }

    /// The skin the chosen tile stands for, as an upload sends it. `None` while the current skin is chosen.
    pub fn chosen(&self) -> Option<Chosen> {
        let (name, png, slim) = match self.choice {
            Choice::Current => return None,
            Choice::Saved(index) => {
                let skin = self.saved.get(index)?;
                (skin.name.clone(), skin.png.clone(), self.slim)
            }
            Choice::Archived(index) => {
                let skin = self.archived.get(index)?;
                (skin.name.clone(), skin.png.clone(), skin.slim.unwrap_or(self.slim))
            }
            Choice::Bundled(index) => {
                let skin = BUNDLED.get(index)?;
                (skin.name.to_owned(), skin.png.to_vec(), skin.slim)
            }
        };
        Some(Chosen { name, png, slim })
    }

    /// Applies the page's own changes. The changes that start a request pass through untouched, for the root.
    pub fn update(&mut self, message: Message) {
        // The previews show the pose, the layers and the model of the saved skins, so changing any of them draws them again.
        let restyles = matches!(
            message,
            Message::Pose(_) | Message::Layer(_) | Message::Slim(_) | Message::ToggleSection(_)
        );
        match message {
            Message::Choose(choice) => self.choose(choice),
            Message::Tick(now) => self.tick(now),
            Message::Pose(pose) => self.pose = pose,
            Message::Layer(layer) => match layer {
                Layer::Hat => self.layers.hat = !self.layers.hat,
                Layer::Jacket => self.layers.jacket = !self.layers.jacket,
                Layer::Sleeves => self.layers.sleeves = !self.layers.sleeves,
                Layer::Pants => self.layers.pants = !self.layers.pants,
            },
            Message::Slim(slim) => self.slim = slim,
            Message::ToggleSection(section) => {
                if !self.collapsed.remove(&section) {
                    self.collapsed.insert(section);
                }
            }
            Message::Use | Message::RemoveArchived(_) | Message::Cape(_) | Message::OpenFolder | Message::Navigate(_) => {}
        }
        if restyles {
            self.previews_stale = true;
        }
    }

    /// The page: its sections, its viewer and its controls, laid out for a page `width` pixels wide.
    pub fn view<'a>(&'a self, account: Account<'a>, width: f32) -> Element<'a, Message> {
        view::view(self, account, width)
    }

    /// Shows `choice` in the viewer, with the cape the account wears.
    fn choose(&mut self, choice: Choice) {
        self.choice = choice;
        let cape = self.appearance.as_ref().and_then(|appearance| appearance.cape_png.as_deref());
        self.preview = match self.source(choice) {
            Some((png, _)) => Skin::compose(png, cape).map(Arc::new),
            None => None,
        };
    }

    /// Whether `choice` names a skin the page holds.
    fn exists(&self, choice: Choice) -> bool {
        match choice {
            Choice::Current => true,
            Choice::Saved(index) => index < self.saved.len(),
            Choice::Archived(index) => index < self.archived.len(),
            Choice::Bundled(index) => index < BUNDLED.len(),
        }
    }

    /// The PNG a choice stands for, and the model it knows, when it knows one. The saved skins know none: the switch picks.
    fn source(&self, choice: Choice) -> Option<(&[u8], Option<bool>)> {
        match choice {
            Choice::Current => None,
            Choice::Saved(index) => self.saved.get(index).map(|skin| (skin.png.as_slice(), None)),
            Choice::Archived(index) => self.archived.get(index).map(|skin| (skin.png.as_slice(), skin.slim)),
            Choice::Bundled(index) => BUNDLED.get(index).map(|skin| (skin.png, Some(skin.slim))),
        }
    }

    /// The choice the grid highlights.
    pub fn choice(&self) -> Choice {
        self.choice
    }

    /// The pose the viewer shows.
    pub fn pose(&self) -> Pose {
        self.pose
    }

    /// The layers the viewer shows.
    pub fn layers(&self) -> Layers {
        self.layers
    }

    /// The model the saved skins show in.
    pub fn slim(&self) -> bool {
        self.slim
    }

    /// Whether a change to the profile is under way.
    pub fn busy(&self) -> bool {
        self.busy
    }

    /// Whether a section is folded away.
    pub fn collapsed(&self, section: Section) -> bool {
        self.collapsed.contains(&section)
    }

    /// The saved skins, in name order.
    pub fn saved(&self) -> &[SavedSkin] {
        &self.saved
    }

    /// The archived skins, newest first.
    pub fn archived(&self) -> &[SavedSkin] {
        &self.archived
    }

    /// The front of the cape with `id`, when it has been read.
    pub fn cape_face(&self, id: &str) -> Option<&Handle> {
        self.cape_faces.get(id)
    }

    /// The skin the viewer shows: the current skin, or the chosen one.
    pub fn shown_skin(&self) -> Option<Arc<Skin>> {
        match self.choice {
            Choice::Current => self.current.clone(),
            _ => self.preview.clone(),
        }
    }

    /// The model the viewer shows: the account's own, for the current skin, or the chosen skin's.
    pub fn shown_slim(&self) -> bool {
        match self.choice {
            Choice::Current => self.appearance.as_ref().is_some_and(|appearance| appearance.slim),
            choice => self.source(choice).and_then(|(_, slim)| slim).unwrap_or(self.slim),
        }
    }

    /// The saved or archived skin the choice names, for its size and colour. `None` for the current skin and the bundled ones.
    pub fn shown_saved(&self) -> Option<&SavedSkin> {
        match self.choice {
            Choice::Saved(index) => self.saved.get(index),
            Choice::Archived(index) => self.archived.get(index),
            Choice::Current | Choice::Bundled(_) => None,
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/skins/mod.rs"]
mod tests;
