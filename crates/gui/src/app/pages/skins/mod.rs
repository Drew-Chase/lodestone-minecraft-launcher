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

use self::capes::CapeArt;
use self::catalog::{BUNDLED, Pack};
use self::library::SavedSkin;
use self::previews::{Drawn, Item, Job, Slot};
use self::viewer::{Layers, Pose, Skin};
use crate::app::pages::Page;
use crate::app::theme::effects;

/// How often the skin previews take a step through the poses' motion, which is one frame of their cache. The viewer draws its
/// own frames faster.
pub const PREVIEW_FRAME: Duration = previews::FRAME;

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

/// The cape the viewer shows with the skin, which Save applies. `Worn` is the cape the account wears, so nothing is pending.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CapePick {
    /// The cape the account wears.
    #[default]
    Worn,
    /// No cape.
    Off,
    /// The cape with this id.
    Owned(String),
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
    /// Shows the cape in the viewer, with the skin on show. Nothing is changed until Save.
    PickCape(CapePick),
    /// Asks to apply the cape the viewer shows. The root applies it.
    SaveCape,
    /// Opens the folder the saved skins live in. The root reveals it.
    OpenFolder,
    /// Opens another page, such as Settings, where a Microsoft account signs in.
    Navigate(Page),
    /// A step of the skin previews' clock, at the moment it fell due.
    Tick(Instant),
}

/// What the previews are drawn in: the pose, the layers, and the model the saved skins wear. A change to any of them makes the
/// cached frames stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Look {
    pose: Pose,
    layers: Layers,
    slim: bool,
}

/// The page's state.
pub struct Skins {
    appearance: Option<Appearance>,
    saved: Vec<SavedSkin>,
    archived: Vec<SavedSkin>,
    /// The digests the archive does not keep again: those of the saved skins and of the bundled ones.
    known: Vec<String>,
    /// The fronts of the capes the player owns, by cape id.
    cape_faces: HashMap<String, Handle>,
    /// The pictures of the capes the player owns, by cape id, which the viewer wears.
    cape_pngs: HashMap<String, Vec<u8>>,
    choice: Choice,
    /// The cape the viewer shows, which Save applies.
    cape_pick: CapePick,
    /// The skin on show, composed with the cape the viewer shows, as the viewer samples it.
    shown: Option<Arc<Skin>>,
    pose: Pose,
    layers: Layers,
    /// The model the saved skins show in, which the switch sets.
    slim: bool,
    collapsed: HashSet<Section>,
    /// Whether a change to the profile is under way. The page offers no other change until it ends.
    busy: bool,
    /// The 3D previews of the skins on the page, one frame of a loop at a time, keyed by the slot and the frame. A loop is drawn
    /// once, and then plays from these frames.
    frames: HashMap<(Slot, usize), Handle>,
    /// The frame last drawn of each skin, which shows while the frame on show is still being drawn.
    latest: HashMap<Slot, Handle>,
    /// The look the frames were drawn in. `None` once they were forgotten, until the next job names the look again.
    cached_look: Option<Look>,
    /// Bumped each time the frames are forgotten, so that a drawing made for them is dropped when it answers.
    generation: u64,
    /// Whether a drawing is running, so that the next one waits for it.
    previews_drawing: bool,
    /// When the page was last on show, which is how long the frames have gone unused (see `sweep_previews`).
    previews_used: Option<Instant>,
    /// The moment the previews' clock started, and the seconds of motion it has run.
    clock_start: Option<Instant>,
    clock: f32,
}

impl Default for Skins {
    fn default() -> Self {
        Self {
            appearance: None,
            saved: Vec::new(),
            archived: Vec::new(),
            known: Vec::new(),
            cape_faces: HashMap::new(),
            cape_pngs: HashMap::new(),
            choice: Choice::Current,
            cape_pick: CapePick::Worn,
            shown: None,
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
            frames: HashMap::new(),
            latest: HashMap::new(),
            cached_look: None,
            generation: 0,
            previews_drawing: false,
            previews_used: None,
            clock_start: None,
            clock: 0.0,
        }
    }
}

impl Skins {
    /// Takes what the folders hold. A choice that no longer names a skin goes back to the current one. The cached frames are
    /// kept while the lists hold the same skins in the same places.
    pub fn set_scan(&mut self, scan: Scan) {
        let lists_changed = !same_skins(&self.saved, &scan.saved) || !same_skins(&self.archived, &scan.archived);
        self.saved = scan.saved;
        self.archived = scan.archived;
        self.known = self.saved.iter().map(|skin| skin.digest.clone()).chain(scan.bundled).collect();
        if !self.exists(self.choice) {
            self.choose(Choice::Current);
        }
        if lists_changed {
            self.forget_frames();
        }
    }

    /// Takes the account's skin and cape, decoded for the viewer. The viewer shows the cape the account wears unless another
    /// is previewed. The cached frames show the cape the account wears, so they are forgotten when it changes.
    pub fn set_appearance(&mut self, appearance: Appearance) {
        let cape_changed = self.appearance.as_ref().is_none_or(|old| old.cape_png != appearance.cape_png);
        self.appearance = Some(appearance);
        self.refresh_shown();
        if cape_changed {
            self.forget_frames();
        }
    }

    /// Takes the capes the player owns: each one's front for the list, and its picture for the viewer.
    pub fn set_capes(&mut self, capes: Vec<CapeArt>) {
        self.cape_faces = capes.iter().map(|cape| (cape.id.clone(), cape.face.clone())).collect();
        self.cape_pngs = capes.into_iter().map(|cape| (cape.id, cape.png)).collect();
        self.refresh_shown();
    }

    /// Drops the cape the viewer shows back to the one the account wears, once a change to the profile has applied.
    pub fn reset_cape_pick(&mut self) {
        self.cape_pick = CapePick::Worn;
        self.refresh_shown();
    }

    /// The change to the account's cape that Save applies. `Some(None)` takes the cape off, `Some(Some(id))` wears the cape with
    /// that id, and `None` leaves the cape as it is. `worn` is the id of the cape the account wears.
    pub fn cape_change(&self, worn: Option<&str>) -> Option<Option<String>> {
        match &self.cape_pick {
            CapePick::Worn => None,
            CapePick::Off => worn.map(|_| None),
            CapePick::Owned(id) => (worn != Some(id.as_str())).then(|| Some(id.clone())),
        }
    }

    /// The cape the viewer shows.
    pub fn cape_pick(&self) -> &CapePick {
        &self.cape_pick
    }

    /// Whether the viewer shows a cape that differs from the one the account wears, so that Save has a change to apply.
    pub fn cape_pending(&self, worn: Option<&str>) -> bool {
        self.cape_change(worn).is_some()
    }

    /// Notes the previews' clock at `now`. The frame on show follows the clock.
    pub fn tick(&mut self, now: Instant) {
        let start = *self.clock_start.get_or_insert(now);
        self.clock = now.saturating_duration_since(start).as_secs_f32();
    }

    /// The job that draws the frame on show for the open skins that have no frame yet. `None` while a drawing runs, and when
    /// every skin of the frame is cached, which is every frame once a loop has been drawn.
    pub fn preview_job(&mut self) -> Option<Job> {
        let look = self.look();
        if self.cached_look != Some(look) {
            self.forget_frames();
            self.cached_look = Some(look);
        }
        if self.previews_drawing {
            return None;
        }
        let frame = self.frame();
        let missing: Vec<Slot> = self.open_slots().filter(|slot| !self.frames.contains_key(&(*slot, frame))).collect();
        if missing.is_empty() {
            return None;
        }
        self.previews_drawing = true;
        Some(Job {
            generation: self.generation,
            frame,
            pose: self.pose,
            time: previews::frame_time(self.pose, frame),
            layers: self.layers,
            cape: self.appearance.as_ref().and_then(|appearance| appearance.cape_png.clone()),
            items: missing.into_iter().filter_map(|slot| Some((slot, self.item(slot)?))).collect(),
        })
    }

    /// Takes the frame a drawing made, and caches each skin's preview in it. A drawing made for forgotten frames is dropped.
    pub fn set_previews(&mut self, drawn: Drawn) {
        self.previews_drawing = false;
        if drawn.generation != self.generation {
            return;
        }
        for (slot, handle) in drawn.previews {
            self.latest.insert(slot, handle.clone());
            self.frames.insert((slot, drawn.frame), handle);
        }
    }

    /// The 3D preview of the saved skin at `index`, in the frame on show.
    pub fn saved_preview(&self, index: usize) -> Option<&Handle> {
        self.preview_of(Slot::Saved(index))
    }

    /// The 3D preview of the archived skin at `index`, in the frame on show.
    pub fn archived_preview(&self, index: usize) -> Option<&Handle> {
        self.preview_of(Slot::Archived(index))
    }

    /// The 3D preview of the bundled skin at `index`, in the frame on show.
    pub fn bundled_preview(&self, index: usize) -> Option<&Handle> {
        self.preview_of(Slot::Bundled(index))
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
        match message {
            Message::Choose(choice) => self.choose(choice),
            Message::PickCape(pick) => self.pick_cape(pick),
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
            Message::Use | Message::RemoveArchived(_) | Message::SaveCape | Message::OpenFolder | Message::Navigate(_) => {}
        }
    }

    /// The page: its sections, its viewer and its controls, laid out for a page `width` pixels wide.
    pub fn view<'a>(&'a self, account: Account<'a>, width: f32) -> Element<'a, Message> {
        view::view(self, account, width)
    }

    /// Shows `choice` in the viewer, with the cape the viewer shows.
    fn choose(&mut self, choice: Choice) {
        self.choice = choice;
        self.refresh_shown();
    }

    /// Shows a cape in the viewer, on the skin on show. Nothing is saved until Save.
    fn pick_cape(&mut self, pick: CapePick) {
        self.cape_pick = pick;
        self.refresh_shown();
    }

    /// Composes the skin on show with the cape the viewer shows, for the viewer to sample. The cached frames of the grid keep
    /// the cape the account wears, so they are not touched here.
    fn refresh_shown(&mut self) {
        let skin = match self.choice {
            Choice::Current => self.appearance.as_ref().map(|appearance| appearance.skin_png.as_slice()),
            choice => self.source(choice).map(|(png, _)| png),
        };
        self.shown = skin.and_then(|png| Skin::compose(png, self.shown_cape())).map(Arc::new);
    }

    /// The picture of the cape the viewer shows: the one the account wears, none, or the one previewed.
    fn shown_cape(&self) -> Option<&[u8]> {
        match &self.cape_pick {
            CapePick::Worn => self.appearance.as_ref().and_then(|appearance| appearance.cape_png.as_deref()),
            CapePick::Off => None,
            CapePick::Owned(id) => self.cape_pngs.get(id).map(Vec::as_slice),
        }
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

    /// The look the previews are drawn in now.
    fn look(&self) -> Look {
        Look {
            pose: self.pose,
            layers: self.layers,
            slim: self.slim,
        }
    }

    /// The frame of the pose's loop on show: the frame at the clock, or the moment the pose is held at under reduced motion.
    fn frame(&self) -> usize {
        let time = if effects::ambient() { self.clock } else { self.pose.preview_time() };
        previews::frame_at(self.pose, time)
    }

    /// Forgets the cached frames. A drawing still running was made for them, so its answer is dropped.
    fn forget_frames(&mut self) {
        self.frames.clear();
        self.latest.clear();
        self.cached_look = None;
        self.generation += 1;
    }

    /// Drops the 3D previews' frames once the page has been away for [`crate::app::retention::IDLE`]. The page draws them again
    /// when it is shown. While the page is on show, its previews count as used. Returns how many frames it dropped.
    pub fn sweep_previews(&mut self, now: Instant, current: bool) -> usize {
        if current {
            self.previews_used = Some(now);
            return 0;
        }
        let away = self
            .previews_used
            .is_none_or(|used| now.saturating_duration_since(used) > crate::app::retention::IDLE);
        if !away || self.frames.is_empty() {
            return 0;
        }
        let count = self.frames.len();
        self.forget_frames();
        count
    }

    /// The slots of the skins in the open sections, in the order the page shows them.
    fn open_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        let saved = if self.collapsed(Section::Saved) { 0..0 } else { 0..self.saved.len() };
        let archived = if self.collapsed(Section::Archived) {
            0..0
        } else {
            0..self.archived.len()
        };
        let bundled = BUNDLED
            .iter()
            .enumerate()
            .filter(|(_, skin)| !self.collapsed(Section::Pack(skin.pack)))
            .map(|(index, _)| Slot::Bundled(index));
        saved.map(Slot::Saved).chain(archived.map(Slot::Archived)).chain(bundled)
    }

    /// The skin a slot names, as a drawing takes it. `None` when the page holds no skin in that slot.
    fn item(&self, slot: Slot) -> Option<Item> {
        match slot {
            Slot::Saved(index) => self.saved.get(index).map(|skin| Item {
                texels: skin.texels.clone(),
                slim: self.slim,
            }),
            Slot::Archived(index) => self.archived.get(index).map(|skin| Item {
                texels: skin.texels.clone(),
                slim: skin.slim.unwrap_or(false),
            }),
            Slot::Bundled(index) => BUNDLED.get(index).map(|skin| Item {
                texels: None,
                slim: skin.slim,
            }),
        }
    }

    /// The preview of the skin at `slot` in the frame on show, or the last frame drawn of it while the frame on show is still
    /// being drawn. Nothing shows while the cached frames belong to another look, until the next job replaces them.
    fn preview_of(&self, slot: Slot) -> Option<&Handle> {
        if self.cached_look != Some(self.look()) {
            return None;
        }
        self.frames.get(&(slot, self.frame())).or_else(|| self.latest.get(&slot))
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

    /// The skin the viewer shows: the current skin, or the chosen one, with the cape the viewer shows.
    pub fn shown_skin(&self) -> Option<Arc<Skin>> {
        self.shown.clone()
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

/// Whether two lists hold the same skins in the same places, so that the previews drawn of one hold for the other.
fn same_skins(old: &[SavedSkin], new: &[SavedSkin]) -> bool {
    old.len() == new.len() && old.iter().zip(new).all(|(old, new)| old.digest == new.digest && old.slim == new.slim)
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/skins/mod.rs"]
mod tests;
