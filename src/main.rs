use std::env;
use std::path::PathBuf;

use crate::library::Library;
use clap::Parser;
use iced::widget::{button, column, text};
use iced::{Alignment, Application, Command, Element, Settings, Theme};
use native_dialog::FileDialog;

pub mod library;

fn main() -> iced::Result {
	let flags = Flags::parse();
	App::run(Settings::with_flags(flags))
}

fn default_library_path() -> PathBuf {
	let mut path = env::current_dir().expect("Should have a current directory");
	path.push("library.json");
	path
}

#[derive(Debug, Parser)]
struct Flags {
	/// The location of the library file.
	#[arg(short, long, default_value = default_library_path().into_os_string())]
	library_file: PathBuf,
}

#[derive(Debug, Clone)]
enum AppState {
	Loading,
	Loaded(Library),
	Errored(String),
}

#[derive(Debug)]
struct App {
	_library_file: PathBuf,
	state: AppState,
}

#[derive(Debug, Clone)]
enum Message {
	Loaded(Result<Library, String>),
	AddBook,
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
			Message::AddBook => {
				println!("Add book");
				let path = FileDialog::new()
					.set_location("~/Desktop")
					.add_filter("Book", &["cbz"])
					.show_open_single_file()
					.unwrap();
				println!("path {:?}", path);
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
		column![
			text(message).size(50),
			button("Add book").on_press(Message::AddBook)
		]
		.padding(50)
		.align_items(Alignment::Center)
		.into()
	}
}
