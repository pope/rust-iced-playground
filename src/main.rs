use std::env;
use std::path::PathBuf;

use crate::library::Library;
use clap::Parser;
use iced::widget::{
	button, column, container, image, row, text, text_input, vertical_space,
	Column, Row,
};
use iced::{
	event, subscription, window, Alignment, Application, Command, Element,
	Event, Length, Renderer, Settings, Subscription, Theme,
};
use library::Book;
use native_dialog::FileDialog;
use uuid::Uuid;

pub mod library;

const INIT_WIN_HEIGHT: u32 = 768;
const INIT_WIN_WIDTH: u32 = 1024;

fn main() -> iced::Result {
	let flags = Flags::parse();
	App::run(Settings {
		window: window::Settings {
			size: (INIT_WIN_WIDTH, INIT_WIN_HEIGHT),
			..window::Settings::default()
		},
		..Settings::with_flags(flags)
	})
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
	win_height: u32,
	win_width: u32,
}

#[derive(Debug, Clone)]
enum Message {
	BookAuthorChanged { id: Uuid, author: String },
	BookTitleChanged { id: Uuid, title: String },
	ImportSingleBook,
	ImportMultipleBooks,
	Loaded(Result<Library, String>),
	ReturnToLibrary,
	WindowResized { height: u32, width: u32 },
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
				win_height: INIT_WIN_HEIGHT,
				win_width: INIT_WIN_WIDTH,
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
			Message::WindowResized { height, width } => {
				self.win_height = height;
				self.win_width = width;
				Command::none()
			}
		}
	}

	fn subscription(&self) -> Subscription<Self::Message> {
		subscription::events_with(|event, status| match (event, status) {
			(
				Event::Window(window::Event::Resized { width, height }),
				event::Status::Ignored,
			) => Some(Message::WindowResized { height, width }),
			_ => None,
		})
	}

	fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
		match &self.state {
			AppState::Loading => Self::loading(),
			AppState::Library => Self::library(&self.library, self.win_width),
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

	fn library(lib: &'a Library, win_width: u32) -> Column<'a, Message> {
		const BOOK_WIDTH: u16 = 250;
		let msg = format!("Got it: {}", lib.get_books().len());

		let mut container = Self::container("Library").push(text(msg));

		let chunk_size = (win_width / BOOK_WIDTH as u32).max(1) as usize;
		for chunk in lib.get_books().chunks(chunk_size) {
			let mut row: Row<'a, Message> = row!();
			for b in chunk {
				row = row.push(text(b.get_title()).width(Length::Fill));
			}
			container = container.push(row);
		}

		container.push(vertical_space(Length::Fill)).push(
			row![
				button("Add book").on_press(Message::ImportSingleBook),
				button("Quick Import").on_press(Message::ImportMultipleBooks)
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
