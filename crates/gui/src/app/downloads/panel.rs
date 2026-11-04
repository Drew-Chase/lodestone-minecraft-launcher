//! The Downloads popover: a header with the running count and the pause-all and settings buttons, the
//! overall progress while installs run, the jobs in three sections (downloading, waiting, completed),
//! and a footer with the day's and the week's totals.

use aurora_leaf::Icon;
use iced::widget::{Column, Space, Text, column, container, row, stack, svg};
use iced::{Background, Border, Color, Element, Fill, Padding, Shadow, Vector, alignment};
use iced_color_extension::rgb;

use super::{Downloads, Job, JobKind, JobState, Message, ModJob, ReinstallJob, Summary, icon_of};
use crate::app::components::badge::Badge;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::{self, IconButton, IconButtonStyle};
use crate::app::components::popover::{Placement, Popover};
use crate::app::components::{ProgressBar, TextButton, instance_icon, platform_logo, scrollbar};
use crate::app::export::format_label;
use crate::app::export::job::ExportJob;
use crate::app::install::download::Transfer;
use crate::app::install::{InstallProgress, InstallRequest};
use crate::app::lang;
use crate::app::pages::instance::format::format_bytes;
use crate::app::theme::{color, fonts, utility};

/// The popover's width.
const WIDTH: f32 = 380.0;
/// The tallest the job list grows before it scrolls.
const LIST_HEIGHT: f32 = 420.0;

/// The sidebar's Downloads trigger: the download icon, tinted and numbered while installs have not
/// ended. The popover opens at `placement`: beside the rail, or above the floating bar.
pub fn trigger(downloads: &Downloads, placement: Placement) -> Element<'static, Message> {
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
        .placement(placement)
        .into()
}

/// The count on the trigger's top-right corner.
fn count_chip(count: usize) -> Element<'static, Message> {
    let chip = container(
        Text::new(count.to_string())
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::primary_on()),
    )
    .padding(Padding::new(0.0).left(5).right(5).top(1).bottom(1))
    .style(|_| container::Style {
        background: Some(Background::Color(color::primary())),
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
    let ink = if active { color::primary() } else { color::text_muted() };
    IconButtonStyle {
        foreground: ink,
        foreground_hover: if active { color::primary() } else { color::text_headline() },
        background: if active { color::primary_tint() } else { Color::TRANSPARENT },
        border: Color::TRANSPARENT,
        background_hover: rgb!(255, 255, 255, 8%),
        border_hover: Color::TRANSPARENT,
        background_disabled: Color::TRANSPARENT,
        foreground_disabled: color::text_disabled(),
        background_pressed: rgb!(255, 255, 255, 12%),
        foreground_pressed: ink,
    }
}

/// The popover's panel for `downloads`.
pub fn view(downloads: &Downloads) -> Element<'static, Message> {
    let mut body = Column::new().spacing(12).width(Fill).push(header(downloads));
    if downloads.installs_running() > 0 {
        body = body.push(overall(downloads.summary()));
    }
    body = body.push(rule()).push(jobs_list(downloads)).push(rule()).push(footer(downloads));
    container(body)
        .padding(14)
        .width(WIDTH)
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg3())),
            border: Border {
                radius: 14.0.into(),
                width: 1.0,
                color: color::surface_line_strong(),
            },
            shadow: Shadow {
                color: rgb!(0, 0, 0, 45%),
                offset: Vector::new(0.0, 10.0),
                blur_radius: crate::app::theme::blur(28.0),
            },
            ..container::Style::default()
        })
        .into()
}

/// A one-pixel hairline across the popover, between its sections.
fn rule() -> Element<'static, Message> {
    container(Space::new().width(Fill).height(1))
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_line())),
            ..container::Style::default()
        })
        .into()
}

/// The title, the running count, and the pause-all and settings buttons.
fn header(downloads: &Downloads) -> Element<'static, Message> {
    let pausable = downloads.has_pausable();
    let mut actions = row![].spacing(6).align_y(alignment::Vertical::Center);
    if pausable || downloads.any_paused() {
        let (icon, label) = if pausable {
            (Icon::lucide().pause(), lang::t("downloads.pause_all"))
        } else {
            (Icon::lucide().play(), lang::t("downloads.resume_all"))
        };
        actions = actions.push(small_button(icon, Message::TogglePauseAll, label, icon_button::secondary_button_style()));
    }
    actions = actions.push(small_button(
        Icon::lucide().settings(),
        Message::OpenSettings,
        lang::t("downloads.settings"),
        icon_button::secondary_button_style(),
    ));

    let mut title = row![
        utility::icon(Icon::lucide().download(), 14.0, Some((color::primary(), color::primary()))),
        Text::new(lang::t("downloads.title"))
            .size(14)
            .font(fonts::Inter::BOLD)
            .color(color::text_headline()),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    let running = downloads.running();
    if running > 0 {
        title = title.push(Badge::new(
            lang::t_with("downloads.active", &[("count", running.to_string().as_str())]),
            color::primary(),
            color::primary_tint(),
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
                .color(color::text_headline()),
            Space::new().width(Fill),
            Text::new(rate_line(summary.per_second, summary.eta_seconds()))
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::primary()),
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
                .color(color::text_muted()),
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
            list = list.push(running_row(job));
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
    container(scrollbar::scroll_panel(list).width(Fill)).max_height(LIST_HEIGHT).into()
}

/// One running job, as the running section lists it: an install's card, or a mod's.
fn running_row(job: &Job) -> Element<'static, Message> {
    match &job.kind {
        JobKind::Install(request) => running_job(job, request),
        JobKind::Mod(mod_job) => running_mod(job, mod_job),
        JobKind::Export(export) => running_export(job, export),
        JobKind::Reinstall(reinstall) => running_reinstall(job, reinstall),
    }
}

/// One running reinstall: its mark, the version being fetched again, and an indeterminate bar, since it reports no
/// progress.
fn running_reinstall(job: &Job, reinstall: &ReinstallJob) -> Element<'static, Message> {
    running_card(
        column![
            row![
                art(job, 40.0),
                column![
                    title(&reinstall.name),
                    mono_line(format!("{} · {}", reinstall.mc_version, lang::t("downloads.reinstall.running")))
                ]
                .spacing(2)
                .width(Fill),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
            ProgressBar::new(0.0).indeterminate(true).width(Fill),
        ]
        .spacing(8),
    )
}

/// One running export: its mark, name, format and stage, a cancel button, and a bar with the share written.
fn running_export(job: &Job, export: &ExportJob) -> Element<'static, Message> {
    let progress = export.progress;
    let fraction = progress.and_then(|progress| progress.fraction());
    let stage = progress.map_or_else(
        || lang::t("export.stage.starting").to_owned(),
        |progress| progress.stage.label().to_owned(),
    );
    running_card(
        column![
            row![
                art(job, 40.0),
                column![title(&export.name), mono_line(format!("{} · {}", format_label(export.format), stage))]
                    .spacing(2)
                    .width(Fill),
                small_button(
                    Icon::lucide().x(),
                    Message::Cancel(job.id),
                    lang::t("downloads.cancel"),
                    icon_button::danger_button_style(),
                ),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
            row![
                ProgressBar::new(fraction.unwrap_or(0.0)).indeterminate(fraction.is_none()).width(Fill),
                Text::new(fraction.map_or_else(String::new, percent_label))
                    .size(11)
                    .font(fonts::JetBrainsMono::SEMI_BOLD)
                    .color(color::text_headline()),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(8),
    )
}

/// One running install, in a tinted card: its art, name and buttons, its bar with the share done at the
/// right, and its rate and time left.
fn running_job(job: &Job, request: &InstallRequest) -> Element<'static, Message> {
    let progress = job.progress.as_ref();
    let fraction = progress.and_then(|progress| progress.fraction);
    let stage = progress.map_or_else(|| lang::t("downloads.starting").to_owned(), |progress| progress.stage.label().to_owned());
    let transfer = progress.map(|progress| progress.transfer).unwrap_or_default();
    running_card(
        column![
            row![
                art(job, 40.0),
                column![title(&request.name), running_subtitle(request, &stage)].spacing(2).width(Fill),
                small_button(
                    Icon::lucide().pause(),
                    Message::Pause(job.id),
                    lang::t("downloads.pause"),
                    icon_button::secondary_button_style(),
                ),
                small_button(
                    Icon::lucide().x(),
                    Message::Cancel(job.id),
                    lang::t("downloads.cancel"),
                    icon_button::danger_button_style(),
                ),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
            row![
                ProgressBar::new(fraction.unwrap_or(0.0)).indeterminate(fraction.is_none()).width(Fill),
                Text::new(fraction.map_or_else(String::new, percent_label))
                    .size(11)
                    .font(fonts::JetBrainsMono::SEMI_BOLD)
                    .color(color::text_headline()),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
            Text::new(rate_line(transfer.per_second, transfer_eta(transfer)))
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::primary()),
        ]
        .spacing(8),
    )
}

/// One mod being added or replaced. A mod reports no progress, so its bar runs without a share, and it has no buttons,
/// since it cannot be paused or cancelled.
fn running_mod(job: &Job, mod_job: &ModJob) -> Element<'static, Message> {
    let verb = if mod_job.replacing {
        lang::t("downloads.mod.replacing")
    } else {
        lang::t("downloads.mod.adding")
    };
    running_card(
        column![
            row![
                art(job, 40.0),
                column![title(&mod_job.name), mono_line(format!("{} · {}", mod_job.instance, verb))]
                    .spacing(2)
                    .width(Fill),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
            ProgressBar::new(0.0).indeterminate(true).width(Fill),
        ]
        .spacing(8),
    )
}

/// The tinted card a running job sits in.
fn running_card(content: impl Into<Element<'static, Message>>) -> Element<'static, Message> {
    container(content)
        .padding(10)
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(34, 255, 132, 4%))),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: Color { a: 0.35, ..color::primary() },
            },
            ..container::Style::default()
        })
        .into()
}

/// A job's picture: the instance's icon for an install, or the platform's mark for a mod.
fn art(job: &Job, size: f32) -> Element<'static, Message> {
    match &job.kind {
        JobKind::Install(request) => instance_icon::icon(&icon_of(request), size),
        JobKind::Mod(mod_job) => platform_logo::logo(mod_job.source.logo(), size),
        JobKind::Export(_) => export_mark(size),
        JobKind::Reinstall(_) => glyph_mark(Icon::lucide().refresh_cw(), size),
    }
}

/// An export's mark, where an install shows its art: the package glyph on a green tile.
fn export_mark(size: f32) -> Element<'static, Message> {
    glyph_mark(Icon::lucide().package(), size)
}

/// A job's mark: a glyph on the primary tint.
fn glyph_mark(glyph_svg: &'static str, size: f32) -> Element<'static, Message> {
    let glyph = svg(svg::Handle::from_memory(glyph_svg.as_bytes()))
        .width(size * 0.5)
        .height(size * 0.5)
        .style(|_, _| svg::Style {
            color: Some(color::primary()),
        });
    container(glyph)
        .width(size)
        .height(size)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(color::primary_tint())),
            border: Border {
                radius: (size * 0.25).into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// A running install's loader, version and stage, in the mono line under its name.
fn running_subtitle(request: &InstallRequest, stage: &str) -> Text<'static> {
    mono_line(format!("{} · {} · {}", request.loader.label(), request.mc_version, stage))
}

/// A line of mono text under a job's name.
fn mono_line(line: String) -> Text<'static> {
    Text::new(line).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted())
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
                icon_button::secondary_button_style(),
            )),
        ),
        _ => (format!("#{}", place.unwrap_or_default()), None),
    };
    let mut items = row![
        art(job, 36.0),
        column![title(job.name()), subtitle(job)].spacing(2).width(Fill),
        Text::new(status).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
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
            icon_button::danger_button_style(),
        ))
        .into()
}

/// One install that has ended: its name, and whether it is done or why it failed.
fn ended_job(job: &Job) -> Element<'static, Message> {
    let (status, ink) = match &job.state {
        JobState::Failed(error) => (lang::t_with("downloads.failed", &[("error", error.as_str())]), color::pink()),
        _ => (lang::t("downloads.done").to_owned(), color::text_muted()),
    };
    let mut items = row![
        art(job, 36.0),
        column![title(job.name()), subtitle(job)].spacing(2).width(Fill),
        Text::new(status).size(11).font(fonts::Inter::REGULAR).color(ink),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);
    if matches!(job.kind, JobKind::Export(_)) && job.state == JobState::Done {
        items = items.push(small_button(
            Icon::lucide().folder(),
            Message::RevealExport(job.id),
            lang::t("export.show"),
            icon_button::secondary_button_style(),
        ));
    }
    items.into()
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
        .color(color::text_muted()),
        Space::new().width(Fill),
        clear,
    ]
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A section's heading: its label, and a hairline after it.
fn section(label: String) -> Element<'static, Message> {
    row![
        Text::new(label).size(10).font(fonts::JetBrainsMono::SEMI_BOLD).color(color::text_muted()),
        container(Space::new().width(Fill).height(1)).style(|_| container::Style {
            background: Some(Background::Color(color::surface_line())),
            ..container::Style::default()
        }),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A job's name, in the popover's row title style.
fn title(name: &str) -> Text<'static> {
    Text::new(name.to_owned()).size(13).font(fonts::Inter::BOLD).color(color::text_headline())
}

/// A job's line under its name: an install's loader and Minecraft version, or the instance a mod goes into.
fn subtitle(job: &Job) -> Text<'static> {
    match &job.kind {
        JobKind::Install(request) => mono_line(format!("{} · {}", request.loader.label(), request.mc_version)),
        JobKind::Mod(mod_job) => mono_line(mod_job.instance.clone()),
        JobKind::Export(export) => mono_line(format_label(export.format).to_owned()),
        JobKind::Reinstall(reinstall) => mono_line(reinstall.mc_version.clone()),
    }
}

/// An icon-only button with a hover hint that names it.
fn small_button(icon: &'static str, message: Message, label: &'static str, style: icon_button::IconButtonStyle) -> Element<'static, Message> {
    hint(
        IconButton::new(icon)
            .width(28)
            .height(28)
            .icon_size(12.0)
            .radius(10.0)
            .style(style)
            .on_press(message),
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
