//! The Downloads popover: a header with the running count and the pause-all and settings buttons, the
//! overall progress while installs run, the jobs in three sections (downloading, waiting, completed),
//! and a footer with the day's and the week's totals.

use aurora_leaf::Icon;
use iced::widget::{Column, Space, Text, column, container, row, stack};
use iced::{Background, Border, Color, Element, Fill, Padding, Shadow, Vector, alignment};
use iced_color_extension::rgb;

use super::{Downloads, Job, JobState, Message, Summary};
use crate::app::components::badge::Badge;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::{self, IconButton, IconButtonStyle};
use crate::app::components::popover::{Placement, Popover};
use crate::app::components::{ProgressBar, TextButton, instance_icon, scrollbar};
use crate::app::install::InstallProgress;
use crate::app::install::download::Transfer;
use crate::app::lang;
use crate::app::pages::instance::format::format_bytes;
use crate::app::theme::{color, fonts, utility};

/// The popover's width.
const WIDTH: f32 = 340.0;
/// The tallest the job list grows before it scrolls.
const LIST_HEIGHT: f32 = 420.0;

/// The sidebar's Downloads trigger: the download icon, tinted and numbered while installs have not
/// ended. The popover opens beside it.
pub fn trigger(downloads: &Downloads) -> Element<'static, Message> {
    let count = downloads.badge();
    let active = count > 0;
    let button = hint(
        IconButton::new(Icon::lucide().download())
            .size(36)
            .radius(12.0)
            .icon_size(16.0)
            .style(trigger_style(active))
            .on_press(Message::TogglePanel),
        lang::t("downloads.title"),
    );
    let face: Element<'static, Message> = if active { stack![button, count_chip(count)].into() } else { button };
    // The trigger spans the rail, so the panel opens clear of it rather than over its edge.
    let face = container(face).width(Fill).center_x(Fill);
    // The panel is only built while it shows, since the sidebar redraws with every frame.
    let panel: Element<'static, Message> = if downloads.is_open() { view(downloads) } else { Space::new().into() };
    Popover::new(face, panel)
        .open(downloads.is_open())
        .on_dismiss(Message::DismissPanel)
        .placement(Placement::Beside)
        .into()
}

/// The count on the trigger's top-right corner.
fn count_chip(count: usize) -> Element<'static, Message> {
    let chip = container(
        Text::new(count.to_string())
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::Primary::ON),
    )
    .padding(Padding::new(0.0).left(5).right(5).top(1).bottom(1))
    .style(|_| container::Style {
        background: Some(Background::Color(color::Primary::DEFAULT)),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    });
    container(chip)
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Top)
        .into()
}

/// The trigger's look: primary ink on a primary tint while installs run, muted otherwise.
fn trigger_style(active: bool) -> IconButtonStyle {
    let ink = if active { color::Primary::DEFAULT } else { color::Text::MUTED };
    IconButtonStyle {
        foreground: ink,
        foreground_hover: if active { color::Primary::DEFAULT } else { color::Text::HEADLINE },
        background: if active { color::Primary::TINT } else { Color::TRANSPARENT },
        border: Color::TRANSPARENT,
        background_hover: rgb!(255, 255, 255, 8%),
        border_hover: Color::TRANSPARENT,
        background_disabled: Color::TRANSPARENT,
        foreground_disabled: color::Text::DISABLED,
        background_pressed: rgb!(255, 255, 255, 12%),
        foreground_pressed: ink,
    }
}

/// The popover's panel for `downloads`.
pub fn view(downloads: &Downloads) -> Element<'static, Message> {
    let mut body = Column::new().spacing(12).width(Fill).push(header(downloads));
    if downloads.running() > 0 {
        body = body.push(overall(downloads.summary()));
    }
    body = body.push(jobs_list(downloads));
    body = body.push(footer(downloads));
    container(body)
        .padding(14)
        .width(WIDTH)
        .style(|_| container::Style {
            background: Some(Background::Color(color::Surface::BG3)),
            border: Border {
                radius: 14.0.into(),
                width: 1.0,
                color: color::Surface::LINE_STRONG,
            },
            shadow: Shadow {
                color: rgb!(0, 0, 0, 45%),
                offset: Vector::new(0.0, 10.0),
                blur_radius: 28.0,
            },
            ..container::Style::default()
        })
        .into()
}

/// The title, the running count, and the pause-all and settings buttons.
fn header(downloads: &Downloads) -> Element<'static, Message> {
    let pausable = downloads
        .jobs()
        .iter()
        .any(|job| matches!(job.state, JobState::Queued | JobState::Running));
    let mut actions = row![].spacing(6).align_y(alignment::Vertical::Center);
    if pausable || downloads.any_paused() {
        let (icon, label) = if pausable {
            (Icon::lucide().pause(), lang::t("downloads.pause_all"))
        } else {
            (Icon::lucide().play(), lang::t("downloads.resume_all"))
        };
        actions = actions.push(small_button(icon, Message::TogglePauseAll, label, icon_button::SECONDARY_BUTTON_STYLE));
    }
    actions = actions.push(small_button(
        Icon::lucide().settings(),
        Message::OpenSettings,
        lang::t("downloads.settings"),
        icon_button::SECONDARY_BUTTON_STYLE,
    ));

    let mut title = row![
        utility::icon(Icon::lucide().download(), 14.0, Some((color::Primary::DEFAULT, color::Primary::DEFAULT))),
        Text::new(lang::t("downloads.title"))
            .size(14)
            .font(fonts::Inter::BOLD)
            .color(color::Text::HEADLINE),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    let running = downloads.running();
    if running > 0 {
        title = title.push(Badge::new(
            lang::t_with("downloads.active", &[("count", running.to_string().as_str())]),
            color::Primary::DEFAULT,
            color::Primary::TINT,
        ));
    }
    row![title, Space::new().width(Fill), actions].align_y(alignment::Vertical::Center).into()
}

/// The overall share of the running installs' files, and their rate and time left.
fn overall(summary: Summary) -> Element<'static, Message> {
    let percent = summary.fraction.map_or_else(String::new, percent_label);
    column![
        row![
            Text::new(percent)
                .size(12)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::Text::HEADLINE),
            Space::new().width(Fill),
            Text::new(rate_line(summary.per_second, summary.eta_seconds()))
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Primary::DEFAULT),
        ]
        .align_y(alignment::Vertical::Center),
        ProgressBar::new(summary.fraction.unwrap_or(0.0))
            .indeterminate(summary.fraction.is_none())
            .width(Fill),
    ]
    .spacing(6)
    .into()
}

/// The jobs, in the sections the popover shows them under. Nothing shows while there are none.
fn jobs_list(downloads: &Downloads) -> Element<'static, Message> {
    let jobs = downloads.jobs();
    if jobs.is_empty() {
        return container(
            Text::new(lang::t("downloads.empty"))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
        )
        .padding(8)
        .into();
    }
    let running: Vec<&Job> = jobs.iter().filter(|job| job.state == JobState::Running).collect();
    let waiting: Vec<&Job> = jobs
        .iter()
        .filter(|job| matches!(job.state, JobState::Queued | JobState::Paused))
        .collect();
    let ended: Vec<&Job> = jobs
        .iter()
        .filter(|job| matches!(job.state, JobState::Done | JobState::Failed(_)))
        .collect();

    let mut list = Column::new().spacing(10).width(Fill);
    if !running.is_empty() {
        list = list.push(section(lang::t_with(
            "downloads.section.downloading",
            &[("count", running.len().to_string().as_str())],
        )));
        for job in running {
            list = list.push(running_job(job));
        }
    }
    if !waiting.is_empty() {
        list = list.push(section(lang::t_with(
            "downloads.section.queued",
            &[("count", waiting.len().to_string().as_str())],
        )));
        let mut position = 0;
        for job in waiting {
            let place = match job.state {
                JobState::Queued => {
                    position += 1;
                    Some(position)
                }
                _ => None,
            };
            list = list.push(waiting_job(job, place));
        }
    }
    if !ended.is_empty() {
        list = list.push(section(lang::t("downloads.section.completed").to_owned()));
        for job in ended {
            list = list.push(ended_job(job));
        }
    }
    container(scrollbar::scroll(list).width(Fill)).max_height(LIST_HEIGHT).into()
}

/// One running install: its art, name and buttons, then its bar, stage and rate.
fn running_job(job: &Job) -> Element<'static, Message> {
    let progress = job.progress.as_ref();
    let fraction = progress.and_then(|progress| progress.fraction);
    let stage = progress.map_or_else(|| lang::t("downloads.starting").to_owned(), |progress| progress.stage.label().to_owned());
    let transfer = progress.map(|progress| progress.transfer).unwrap_or_default();
    let eta = transfer_eta(transfer);
    let detail = match fraction {
        Some(fraction) if transfer.per_second == 0 => percent_label(fraction),
        _ => rate_line(transfer.per_second, eta),
    };
    column![
        row![
            instance_icon::icon(&job.icon(), 40.0),
            column![title(&job.request.name), subtitle(job)].spacing(2).width(Fill),
            small_button(
                Icon::lucide().pause(),
                Message::Pause(job.id),
                lang::t("downloads.pause"),
                icon_button::SECONDARY_BUTTON_STYLE,
            ),
            small_button(
                Icon::lucide().x(),
                Message::Cancel(job.id),
                lang::t("downloads.cancel"),
                icon_button::DANGER_BUTTON_STYLE,
            ),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
        ProgressBar::new(fraction.unwrap_or(0.0)).indeterminate(fraction.is_none()).width(Fill),
        row![
            Text::new(stage).size(11).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
            Space::new().width(Fill),
            Text::new(detail).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::Text::MUTED),
        ]
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(8)
    .into()
}

/// One queued or paused install. A queued one shows its place in the queue; a paused one can resume.
fn waiting_job(job: &Job, place: Option<usize>) -> Element<'static, Message> {
    let (status, resume) = match job.state {
        JobState::Paused => (
            lang::t("downloads.paused").to_owned(),
            Some(small_button(
                Icon::lucide().play(),
                Message::Resume(job.id),
                lang::t("downloads.resume"),
                icon_button::SECONDARY_BUTTON_STYLE,
            )),
        ),
        _ => (format!("#{}", place.unwrap_or_default()), None),
    };
    let mut items = row![
        instance_icon::icon(&job.icon(), 36.0),
        column![title(&job.request.name), subtitle(job)].spacing(2).width(Fill),
        Text::new(status).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::Text::MUTED),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);
    if let Some(resume) = resume {
        items = items.push(resume);
    }
    items
        .push(small_button(
            Icon::lucide().x(),
            Message::Cancel(job.id),
            lang::t("downloads.cancel"),
            icon_button::DANGER_BUTTON_STYLE,
        ))
        .into()
}

/// One install that has ended: its name, and whether it is done or why it failed.
fn ended_job(job: &Job) -> Element<'static, Message> {
    let (status, ink) = match &job.state {
        JobState::Failed(error) => (lang::t_with("downloads.failed", &[("error", error.as_str())]), color::Accents::PINK),
        _ => (lang::t("downloads.done").to_owned(), color::Text::MUTED),
    };
    row![
        instance_icon::icon(&job.icon(), 36.0),
        column![title(&job.request.name), subtitle(job)].spacing(2).width(Fill),
        Text::new(status).size(11).font(fonts::Inter::REGULAR).color(ink),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// The footer: the day's and the week's totals, and the button that clears the jobs that have ended.
fn footer(downloads: &Downloads) -> Element<'static, Message> {
    let totals = downloads.totals();
    let ended = downloads
        .jobs()
        .iter()
        .any(|job| matches!(job.state, JobState::Done | JobState::Failed(_)));
    let clear: Element<'static, Message> = if ended {
        TextButton::ghost(lang::t("downloads.clear_completed"))
            .on_press(Message::ClearCompleted)
            .into()
    } else {
        TextButton::ghost(lang::t("downloads.clear_completed")).disabled(true).into()
    };
    row![
        Text::new(lang::t_with(
            "downloads.totals",
            &[
                ("today", format_bytes(totals.today).as_str()),
                ("week", format_bytes(totals.week).as_str()),
            ],
        ))
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::Text::MUTED),
        Space::new().width(Fill),
        clear,
    ]
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A section's heading: its label, and a hairline after it.
fn section(label: String) -> Element<'static, Message> {
    row![
        Text::new(label).size(10).font(fonts::JetBrainsMono::SEMI_BOLD).color(color::Text::MUTED),
        container(Space::new().width(Fill).height(1)).style(|_| container::Style {
            background: Some(Background::Color(color::Surface::LINE)),
            ..container::Style::default()
        }),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A job's name, in the popover's row title style.
fn title(name: &str) -> Text<'static> {
    Text::new(name.to_owned()).size(13).font(fonts::Inter::BOLD).color(color::Text::HEADLINE)
}

/// A job's loader and Minecraft version, in the mono line under its name.
fn subtitle(job: &Job) -> Text<'static> {
    Text::new(format!("{} · {}", job.request.loader.label(), job.request.mc_version))
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::Text::MUTED)
}

/// An icon-only button with a hover hint that names it.
fn small_button(icon: &'static str, message: Message, label: &'static str, style: icon_button::IconButtonStyle) -> Element<'static, Message> {
    hint(
        IconButton::new(icon).width(28).height(28).icon_size(12.0).style(style).on_press(message),
        label,
    )
}

/// `fraction` as a whole percentage, clamped to 100.
fn percent_label(fraction: f32) -> String {
    format!("{:.0}%", (fraction * 100.0).min(100.0))
}

/// The seconds a transfer has left at its current rate, when the rate is known.
fn transfer_eta(transfer: Transfer) -> Option<u64> {
    (transfer.per_second > 0).then(|| transfer.total.saturating_sub(transfer.done) / transfer.per_second)
}

/// The line under an install's bar: what the install is doing, and how fast it is moving when that is known.
pub fn progress_detail(progress: Option<&InstallProgress>) -> String {
    let Some(progress) = progress else {
        return lang::t("downloads.starting").to_owned();
    };
    let rate = rate_line(progress.transfer.per_second, transfer_eta(progress.transfer));
    if rate.is_empty() {
        return progress.detail.clone();
    }
    lang::t_with(
        "downloads.detail_with_rate",
        &[("detail", progress.detail.as_str()), ("rate", rate.as_str())],
    )
}

/// The download rate, and the time left when it is known. Empty while the rate is unknown.
fn rate_line(per_second: u64, eta: Option<u64>) -> String {
    if per_second == 0 {
        return String::new();
    }
    let rate = lang::t_with("downloads.per_second", &[("rate", format_bytes(per_second).as_str())]);
    match eta {
        Some(seconds) => lang::t_with("downloads.rate_eta", &[("rate", rate.as_str()), ("eta", eta_label(seconds).as_str())]),
        None => lang::t_with("downloads.rate", &[("rate", rate.as_str())]),
    }
}

/// A time left, as seconds, minutes and seconds, or hours and minutes.
fn eta_label(seconds: u64) -> String {
    if seconds < 60 {
        lang::t_with("downloads.eta_seconds", &[("seconds", seconds.to_string().as_str())])
    } else if seconds < 3600 {
        lang::t_with(
            "downloads.eta_minutes",
            &[
                ("minutes", (seconds / 60).to_string().as_str()),
                ("seconds", (seconds % 60).to_string().as_str()),
            ],
        )
    } else {
        lang::t_with(
            "downloads.eta_hours",
            &[
                ("hours", (seconds / 3600).to_string().as_str()),
                ("minutes", ((seconds % 3600) / 60).to_string().as_str()),
            ],
        )
    }
}
