mod app;
use crate::app::LodestoneLauncher;
pub use app::paths::logs_dir;
/// Measurement for the `perf` feature. The binary starts it, so it is public for `main`'s logging.
pub use app::perf;
use app::theme::{color, fonts};
use aurora_leaf::FontFamily;
use color_eyre::Result;
use tracing::info;

/// Chooses the launcher's roots: a launcher that carries a package takes its own, so its logs and files stay apart, and a
/// development run takes the folder `APP_DIRECTORY` names. Call it before [`logs_dir`] and before logging starts.
pub fn prepare() {
    app::portable::prepare();
    app::paths::use_app_directory(std::env::var_os("APP_DIRECTORY").as_deref());
}

pub fn run() -> Result<()> {
    // A launcher already running is shown instead, so this one stops before it opens a window.
    if app::single_instance::hand_over() {
        return Ok(());
    }
    if let Some(portable) = app::portable::launch() {
        info!("Starting the portable launcher for {}", portable.manifest.name);
    }
    info!("Starting Lodestone Launcher v{}#{}", LodestoneLauncher::VERSION, LodestoneLauncher::BUILD);
    if let Some(root) = app::paths::development_root() {
        info!("Development run: the launcher keeps its files under {}", root.display());
    }
    iced::application(LodestoneLauncher::boot, LodestoneLauncher::update, LodestoneLauncher::view)
        .title(LodestoneLauncher::TITLE)
        .window(LodestoneLauncher::window_settings())
        .centered()
        .theme(iced::Theme::Dark)
        .default_font(fonts::Inter::REGULAR)
        .font(FontFamily::inter().regular())
        .font(FontFamily::inter().medium())
        .font(FontFamily::inter().semi_bold())
        .font(FontFamily::inter().bold())
        .font(FontFamily::inter().extra_bold())
        .font(FontFamily::jetbrains_mono().regular())
        .font(FontFamily::jetbrains_mono().semi_bold())
        .subscription(LodestoneLauncher::subscription)
        .style(|_app, _theme| iced::theme::Style {
            text_color: color::text_default(),
            background_color: color::surface_application(),
        })
        .run()?;
    Ok(())
}
