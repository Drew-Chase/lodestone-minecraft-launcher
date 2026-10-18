mod app;
use crate::app::LodestoneLauncher;
use app::theme::color;
use aurora_leaf::FontFamily;
use color_eyre::Result;
use tracing::info;

pub fn run() -> Result<()> {
	info!("Starting Lodestone Launcher v{}#{}", LodestoneLauncher::VERSION, LodestoneLauncher::BUILD);
	iced::application(LodestoneLauncher::boot, LodestoneLauncher::update, LodestoneLauncher::view)
		.title(LodestoneLauncher::TITLE)
		.window(LodestoneLauncher::window_settings())
		.centered()
		.theme(iced::Theme::Light)
		.font(FontFamily::roboto().bold())
		.font(FontFamily::roboto().medium())
		.font(FontFamily::roboto().regular())
		.subscription(LodestoneLauncher::subscription)
		.style(|_app, _theme| iced::theme::Style {
			text_color: color::Text::DEFAULT,
			background_color: color::Surface::APPLICATION,
		})
		.run()?;
	Ok(())
}