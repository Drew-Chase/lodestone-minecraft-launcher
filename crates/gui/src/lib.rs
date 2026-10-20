mod app;
use crate::app::LodestoneLauncher;
use app::theme::{color, fonts};
use aurora_leaf::FontFamily;
use color_eyre::Result;
use tracing::info;

pub fn run() -> Result<()> {
	info!("Starting Lodestone Launcher v{}#{}", LodestoneLauncher::VERSION, LodestoneLauncher::BUILD);
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
			text_color: color::Text::DEFAULT,
			background_color: color::Surface::APPLICATION,
		})
		.run()?;
	Ok(())
}