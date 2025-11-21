mod app;
mod config;
mod dataset;
mod image;
mod state;
mod ui;

use iced::{application, Size};

use app::{TaggerState, APP_TITLE, WINDOW_SIZE};
use config::AppConfig;

fn main() -> iced::Result {
    let config = AppConfig::discover().unwrap_or_else(|err| {
        eprintln!("{APP_TITLE} unable to start: {err:#}");
        std::process::exit(1);
    });

    let initial_config = config.clone();

    application(
        |state: &TaggerState| state.title(),
        TaggerState::update,
        TaggerState::view,
    )
    .theme(|state| state.theme())
    .subscription(|_| TaggerState::subscription())
    .window_size(Size::new(WINDOW_SIZE.0, WINDOW_SIZE.1))
    .centered()
    .run_with(move || TaggerState::boot(initial_config.clone()))
}
