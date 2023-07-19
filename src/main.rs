use std::env;
use std::path::PathBuf;

use crate::library::Library;
use clap::Parser;
use iced::widget::{
	button, column, container, image, row, text, text_input, vertical_space,
	Column,
};
use iced::{
	Alignment, Application, Command, Element, Length, Renderer, Settings, Theme,
};
use library::Book;
use native_dialog::FileDialog;
use uuid::Uuid;

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
	Library,
	EditBook { id: Uuid },
	Errored(String),
}

#[derive(Debug)]
struct App {
	library: Library,
	_library_file: PathBuf,
	state: AppState,
}

#[derive(Debug, Clone)]
enum Message {
	BookAuthorChanged { id: Uuid, author: String },
	BookTitleChanged { id: Uuid, title: String },
	ImportSingleBook,
	ImportMultipleBooks,
	Loaded(Result<Library, String>),
	ReturnToLibrary,
}

impl Application for App {
	type Executor = iced::executor::Default;
	type Flags = Flags;
	type Message = Message;
	type Theme = Theme;

	fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
		(
			Self {
				library: Library::default(),
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
			AppState::Library => "Library",
			AppState::EditBook { .. } => "Add Book",
			AppState::Errored(_) => "Ooops",
		};
		format!("{subtitle} - My App")
	}

	fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
		match message {
			Message::Loaded(Ok(library)) => {
				self.library = library;
				self.state = AppState::Library;
				Command::none()
			}
			Message::Loaded(Err(e)) => {
				self.state = AppState::Errored(e);
				Command::none()
			}
			Message::ImportSingleBook => {
				let path = FileDialog::new()
					.set_location("~/Desktop")
					.add_filter("Book", &["cbz"])
					.show_open_single_file()
					.unwrap();
				if let Some(path) = path {
					let id = self.library.add_book(&path);
					self.state = AppState::EditBook { id };
				}
				Command::none()
			}
			Message::ImportMultipleBooks => {
				let paths = FileDialog::new()
					.set_location("~/Desktop")
					.add_filter("Books", &["cbz"])
					.show_open_multiple_file()
					.unwrap();
				paths.iter().for_each(|p| {
					self.library.add_book(p);
				});
				Command::none()
			}
			Message::BookTitleChanged { id, title } => {
				if let Some(book) = self.library.get_book_mut(&id) {
					book.set_title(title);
				}
				Command::none()
			}
			Message::BookAuthorChanged { id, author } => {
				if let Some(book) = self.library.get_book_mut(&id) {
					book.set_author(author);
				}
				Command::none()
			}
			Message::ReturnToLibrary => {
				self.state = AppState::Library;
				Command::none()
			}
		}
	}

	fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
		match &self.state {
			AppState::Loading => Self::loading(),
			AppState::Library => Self::library(&self.library),
			AppState::EditBook { id } => Self::edit_book(
				self.library
					.get_book(id)
					.expect("Should have found book by ID"),
			),
			AppState::Errored(e) => Self::errored(e),
		}
		.into()
	}
}

impl<'a> App {
	fn container(title: &str) -> Column<'a, Message> {
		column![text(title).size(50)].spacing(20).padding(20)
	}

	fn loading() -> Column<'a, Message> {
		Self::container("Loading").push("Loading")
	}

	fn library(lib: &'a Library) -> Column<'a, Message> {
		let msg = format!("Got it: {}", lib.get_books().len());
		Self::container("Library")
			.push(text(msg))
			.push(vertical_space(Length::Fill))
			.push(
				row![
					button("Add book").on_press(Message::ImportSingleBook),
					button("Quick Import")
						.on_press(Message::ImportMultipleBooks)
				]
				.spacing(20),
			)
	}

	fn edit_book(book: &'a Book) -> Column<'a, Message> {
		let label_size = 100;
		Self::container("Add book")
			.push(text(book.get_path_str().to_string()))
			.push(
				row![
					// TODO(pope): Get rid of this unwrap.
					container(image(book.load_image().unwrap()).width(250))
						.center_x(),
					column![
						row![
							text("Title").width(label_size),
							text_input("Enter a title...", book.get_title())
								.on_input(|title| {
									Message::BookTitleChanged {
										id: book.get_id(),
										title,
									}
								})
						]
						.spacing(20)
						.align_items(Alignment::Center),
						row![
							text("Author").width(label_size),
							text_input("Enter an author...", book.get_author())
								.on_input(|author| {
									Message::BookAuthorChanged {
										id: book.get_id(),
										author,
									}
								})
						]
						.spacing(20)
						.align_items(Alignment::Center),
					]
					.spacing(20),
				]
				.spacing(20),
			)
			.push(vertical_space(Length::Fill))
			.push(button("Back").on_press(Message::ReturnToLibrary))
	}

	fn errored(e: &'a str) -> Column<'a, Message> {
		Self::container("Error").push(e)
	}
}
