//! The setup wizard: the first-run screen that signs the player in, sets the folders the launcher keeps its files in, imports
//! the instances of other launchers and adds an instance from Discover. It shares the chrome of the sign-in gate: the top bar
//! and the status line come from the sign-in page, and the card is the gate's card. The root routes the wizard's messages and
//! runs their tasks (see `app/setup.rs`, and the import and Discover modules); this page holds the steps and what each shows.

pub mod discover;
mod done;
mod folders;
mod launchers;

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row};
use iced::{Element, Fill, Padding, alignment};

use crate::app::components::stepper::stepper;
use crate::app::components::{TextButton, scrollbar};
use crate::app::data_folder::{FolderAction, FolderKind};
use crate::app::import::Message as ImportMessage;
use crate::app::import::launcher_picker::LauncherPicker;
use crate::app::lang;
use crate::app::pages::login::{self, AuthStatus, Login, card_frame, icon};
use crate::app::paths::LauncherPaths;
use crate::app::theme::{color as palette, fonts};

/// The width of the wizard's card. The window is at least 800px wide, so the card fits beside the gate's margins.
const CARD_WIDTH: f32 = 720.0;

/// The space the scrolled body keeps clear on its right for the scrollbar, which is 16px wide.
const SCROLLBAR_GUTTER: f32 = 16.0;

/// The steps the wizard goes through, in order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Step {
    /// The player signs in: with Microsoft, offline, or in demo mode.
    #[default]
    SignIn,
    /// The folders the launcher keeps its files in. It comes before the installs, which land in them.
    Folders,
    /// The instances of the other launchers on this machine, which are copied in.
    Import,
    /// An instance added from a modpack that Discover lists.
    Discover,
    /// The summary, and the way into the launcher.
    Done,
}

impl Step {
    /// The steps the stepper lists, in order. Done follows the last of them.
    pub const LISTED: [Step; 4] = [Step::SignIn, Step::Folders, Step::Import, Step::Discover];

    /// The step after this one. Done is the last step.
    pub fn next(self) -> Step {
        match self {
            Step::SignIn => Step::Folders,
            Step::Folders => Step::Import,
            Step::Import => Step::Discover,
            Step::Discover | Step::Done => Step::Done,
        }
    }

    /// The step before this one. The sign-in is the first step, so it is its own previous step.
    pub fn prev(self) -> Step {
        match self {
            Step::SignIn | Step::Folders => Step::SignIn,
            Step::Import => Step::Folders,
            Step::Discover => Step::Import,
            Step::Done => Step::Discover,
        }
    }

    /// The stepper's active index: the position of this step among the listed ones. Done lies past the last one.
    pub fn index(self) -> usize {
        Self::LISTED.iter().position(|step| *step == self).unwrap_or(Self::LISTED.len())
    }

    /// The name the stepper shows for this step.
    fn label(self) -> &'static str {
        match self {
            Step::SignIn => lang::t("setup.step.sign_in"),
            Step::Folders => lang::t("setup.step.folders"),
            Step::Import => lang::t("setup.step.import"),
            Step::Discover => lang::t("setup.step.discover"),
            Step::Done => lang::t("setup.step.done"),
        }
    }
}

/// The wizard's state: the step on screen, what the Import and Discover steps hold, and what the import brought in.
#[derive(Debug, Clone, Default)]
pub struct Setup {
    step: Step,
    /// The Import step's picker: the launchers on this machine, and the instances the player checked.
    import: LauncherPicker,
    /// Whether the checked instances are being read and queued.
    importing: bool,
    /// The Discover step's search, and the installs it has prepared or queued.
    discover: discover::State,
    /// How many instances the Import step has copied in, for the summary.
    imported: usize,
}

/// The wizard's messages.
#[derive(Debug, Clone)]
pub enum Message {
    /// A message of the sign-in page, which the sign-in step shows. The root hands it to the sign-in page's update.
    Login(login::Message),
    /// A folder row's button: the folder opens, or it is changed, or it goes back to its default. The root carries it out.
    Folder(FolderKind, FolderAction),
    /// A message of the Import step: the launchers' picker, and the import of what it checked. See `app/import/mod.rs`.
    Import(ImportMessage),
    /// A message of the Discover step: the search, and the install of a hit. See `discover.rs`, and `app/setup.rs` for the root's side.
    Discover(discover::Message),
    /// Moves to the next step. The sign-in step moves on only once an account is stored.
    Next,
    /// Moves to the previous step.
    Back,
    /// Leaves the wizard for the sign-in page, or for the launcher when an account is stored. The setup is remembered as skipped.
    Skip,
    /// Leaves the wizard from its summary, into the launcher.
    Finish,
}

/// What the wizard shows that the root holds: the sign-in page's state, the account that plays, and the folders.
pub struct Context<'a> {
    /// The sign-in page, whose sign-in flow the sign-in step shows.
    pub login: &'a Login,
    /// The auth services' status, for the status line.
    pub auth: Option<AuthStatus>,
    /// The name of the account that plays, when one is stored.
    pub account: Option<&'a str>,
    /// The folders the launcher keeps its files in.
    pub paths: &'a LauncherPaths,
}

impl Setup {
    /// The step on screen.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Moves to the step after the current one. A sign-in that completes in the wizard moves it on this way.
    pub fn advance(&mut self) {
        self.step = self.step.next();
    }

    /// The Import step's picker, which holds the launchers and the instances checked.
    pub fn launchers(&self) -> &LauncherPicker {
        &self.import
    }

    /// The Import step's picker, to change what it holds.
    pub fn launchers_mut(&mut self) -> &mut LauncherPicker {
        &mut self.import
    }

    /// Whether the checked instances are being read and queued.
    pub fn importing(&self) -> bool {
        self.importing
    }

    /// Records whether the checked instances are being read and queued.
    pub fn set_importing(&mut self, importing: bool) {
        self.importing = importing;
    }

    /// The Discover step's search and installs.
    pub fn discover(&self) -> &discover::State {
        &self.discover
    }

    /// The Discover step's search and installs, to change them.
    pub fn discover_mut(&mut self) -> &mut discover::State {
        &mut self.discover
    }

    /// How many instances the Import step has copied in.
    pub fn imported(&self) -> usize {
        self.imported
    }

    /// Counts instances the Import step copied in.
    pub fn add_imported(&mut self, count: usize) {
        self.imported += count;
    }

    /// Applies a message that only changes what the wizard shows. The folder, import, Skip and Finish messages are the root's to
    /// carry out.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Next => self.advance(),
            Message::Back => self.step = self.step.prev(),
            Message::Login(_) | Message::Folder(..) | Message::Import(_) | Message::Discover(_) | Message::Skip | Message::Finish => {}
        }
    }

    /// The wizard's page: the sign-in gate's top bar and status line around the wizard's card.
    pub fn view<'a>(&'a self, context: Context<'a>) -> Element<'a, Message> {
        let top = context.login.top_bar().map(Message::Login);
        let status = context.login.status_bar(context.auth.clone()).map(Message::Login);
        let card = container(self.card(&context))
            .padding(Padding::new(0.0).top(12.0).bottom(12.0))
            .width(Fill)
            .height(Fill)
            .align_x(alignment::Horizontal::Center);
        column![container(top).width(Fill), card, status].width(Fill).height(Fill).into()
    }

    /// The card: the stepper on top, the step's body, and the footer with the buttons that move through the wizard.
    fn card<'a>(&'a self, context: &Context<'a>) -> Element<'a, Message> {
        let stepper = stepper::<Message>(&Step::LISTED.map(Step::label), self.step.index(), None);
        let body = container(scrollbar::scroll(self.body(context)).width(Fill))
            .padding(Padding::new(0.0).left(36.0).right(36.0).top(22.0))
            .width(Fill)
            .height(Fill);
        let content = column![stepper, body, self.footer(context)].width(Fill).height(Fill);
        card_frame(content.into(), CARD_WIDTH, Fill)
    }

    /// The step's body: its title and what it asks, and the controls it shows.
    fn body<'a>(&'a self, context: &Context<'a>) -> Element<'a, Message> {
        let (title, subtitle) = self.heading();
        let heading = column![
            Text::new(title).size(26).font(fonts::Inter::EXTRA_BOLD).color(palette::text_headline()),
            Text::new(subtitle)
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(palette::text_muted())
                .line_height(1.5),
        ]
        .spacing(6);
        let controls: Element<'a, Message> = match self.step {
            Step::SignIn => sign_in(context.login, context.account),
            Step::Folders => folders::view(context.paths),
            Step::Import => launchers::view(&self.import, self.importing),
            Step::Discover => discover::view(&self.discover).map(Message::Discover),
            Step::Done => done::view(context.account, context.paths, self.imported(), self.discover.queued.len()),
        };
        // The gutter keeps the scrollbar, which the scrolled body draws at its right edge, clear of the content.
        column![heading, Space::new().height(18), controls]
            .padding(Padding::new(0.0).right(SCROLLBAR_GUTTER))
            .width(Fill)
            .into()
    }

    /// The title and the line under it that the step's body starts with.
    fn heading(&self) -> (&'static str, &'static str) {
        match self.step {
            Step::SignIn => (lang::t("setup.sign_in.title"), lang::t("setup.sign_in.subtitle")),
            Step::Folders => (lang::t("setup.folders.title"), lang::t("setup.folders.subtitle")),
            Step::Import => (lang::t("setup.import.title"), lang::t("setup.import.subtitle")),
            Step::Discover => (lang::t("setup.discover.title"), lang::t("setup.discover.subtitle")),
            Step::Done => (lang::t("setup.done.title"), lang::t("setup.done.subtitle")),
        }
    }

    /// The footer: Skip on the left, then Back and Next on the right, with the sign-in gate's legal line under them. Next is off
    /// until an account is stored, since the sign-in step moves on only then. On the Import step, once instances are checked,
    /// Next copies them in and names how many, as the Import dialog's button does.
    fn footer<'a>(&'a self, context: &Context<'a>) -> Element<'a, Message> {
        let (next_label, next_message, next_ready): (String, Message, bool) = if self.step == Step::Done {
            (lang::t("setup.finish").to_string(), Message::Finish, true)
        } else if self.step == Step::Import && self.import.has_chosen() {
            let count = self.import.chosen_count().to_string();
            let label = lang::t_with("setup.import.submit_count", &[("count", count.as_str())]).to_string();
            (label, Message::Import(ImportMessage::Submit), !self.importing)
        } else {
            let ready = !(self.step == Step::SignIn && context.account.is_none());
            (lang::t("setup.next").to_string(), Message::Next, ready)
        };
        let next = TextButton::primary(next_label)
            .padding(Padding::new(0.0).left(18.0).right(18.0).top(9.0).bottom(9.0))
            .disabled(!next_ready)
            .on_press(next_message);
        let mut buttons = row![].spacing(10).align_y(alignment::Vertical::Center);
        if self.step != Step::Done {
            buttons = buttons.push(TextButton::ghost(lang::t("setup.skip")).on_press(Message::Skip));
        }
        buttons = buttons.push(Space::new().width(Fill));
        if self.step != Step::SignIn {
            buttons = buttons.push(TextButton::ghost(lang::t("setup.back")).on_press(Message::Back));
        }
        buttons = buttons.push(next);
        column![
            buttons,
            container(context.login.footer().map(Message::Login)).padding(Padding::new(0.0).top(8.0)),
        ]
        .spacing(6)
        .padding(Padding::new(0.0).left(36.0).right(36.0).top(14.0).bottom(22.0))
        .width(Fill)
        .into()
    }
}

/// The sign-in step's controls: the sign-in buttons until an account is stored, then the account that plays.
fn sign_in<'a>(login: &'a Login, account: Option<&'a str>) -> Element<'a, Message> {
    match account {
        Some(name) => signed_in(name),
        None => login.sign_in_area().map(Message::Login),
    }
}

/// The line that shows the stored account the wizard signed in: its name, and what Next does with it.
fn signed_in<'a>(name: &str) -> Element<'a, Message> {
    row![
        icon(Icon::lucide().check(), 18.0, palette::primary(), 1.0).map(Message::Login),
        column![
            Text::new(lang::t_with("setup.sign_in.signed_in", &[("name", name)]))
                .size(14)
                .font(fonts::Inter::SEMI_BOLD)
                .color(palette::text_headline()),
            Text::new(lang::t("setup.sign_in.signed_in_hint"))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(palette::text_muted()),
        ]
        .spacing(4),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/setup/mod.rs"]
mod tests;
