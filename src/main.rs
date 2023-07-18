use crate::library::Library;
use clap::Parser;
use iced::widget::{column, text};
use iced::{Alignment, Application, Command, Element, Settings, Theme};

pub mod library;

fn main() -> iced::Result {
	let flags = Flags::parse();
	App::run(Settings::with_flags(flags))
}

#[derive(Debug, Parser)]
struct Flags {
	/// The location of the library file.
	#[arg(short, long, default_value_t = String::from("./library.json"))]
	library_file: String,
}

#[derive(Debug, Clone)]
enum AppState {
	Loading,
	Loaded(Library),
	Errored(String),
}

#[derive(Debug)]
struct App {
	_library_file: String,
	state: AppState,
}

#[derive(Debug, Clone)]
enum Message {
	Loaded(Result<Library, String>),
}

impl Application for App {
	type Executor = iced::executor::Default;
	type Flags = Flags;
	type Message = Message;
	type Theme = Theme;

	fn new(flags: Flags) -> (Self, Command<Message>) {
		(
			Self {
				_library_file: flags.library_file.clone(),
				state: AppState::Loading,
			},
			Command::perform(
				Library::load(flags.library_file),
				Message::Loaded,
			),
		)
	}

	fn title(&self) -> String {
		let subtitle = match &self.state {
			AppState::Loading => "Loading",
			AppState::Loaded(_) => "Loaded",
			AppState::Errored(_) => "Ooops",
		};
		format!("{subtitle} - My App")
	}

	fn update(&mut self, message: Message) -> Command<Message> {
		match message {
			Message::Loaded(Ok(data)) => {
				self.state = AppState::Loaded(data);
				Command::none()
			}
			Message::Loaded(Err(e)) => {
				self.state = AppState::Errored(e);
				Command::none()
			}
		}
	}

	fn view(&self) -> Element<Message> {
		let message = match &self.state {
			AppState::Loading => "loading...".to_owned(),
			AppState::Loaded(lib) => {
				format!("Got it: {}", lib.get_books().len())
			}
			// Interestingly enough, the emoji doesn't print on the screen
			AppState::Errored(e) => format!("⚠️  {e}"),
		};
		column![text(message).size(50)]
			.padding(50)
			.align_items(Alignment::Center)
			.into()
	}
}
