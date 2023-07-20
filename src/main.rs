use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use crate::library::{load_cover_image, save_library_json, Book, Library};
use clap::Parser;
use iced::widget::{
	button, column, container, horizontal_space, image, row, scrollable, text,
	text_input, vertical_space, Column, Row,
};
use iced::{
	event, subscription, theme, window, Alignment, Application, Color, Command,
	Element, Event, Length, Renderer, Settings, Subscription, Theme,
};
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
	image_cache: HashMap<Uuid, image::Handle>,
	library: Library,
	library_file: PathBuf,
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
	ImageLoaded(Uuid, Result<image::Handle, String>),
	ReturnToLibrary,
	SaveLibrary,
	SaveLibraryComplete(Result<(), String>),
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
				image_cache: HashMap::new(),
				library: Library::default(),
				library_file: flags.library_file.clone(),
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

	fn theme(&self) -> Self::Theme {
		Theme::custom(theme::Palette {
			background: Color::from_rgb8(0x21, 0x21, 0x21),
			text: Color::from_rgb8(0xFF, 0xFF, 0xFF),
			// primary: Color::from_rgb8(0xFF, 0x40, 0x81),
			primary: Color::from_rgb8(0xC2, 0x18, 0x5B),
			success: Color::from_rgb8(0x00, 0xBC, 0xD4),
			danger: Color::from_rgb8(0xFF, 0xC1, 0x07),
		})
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

				let commands = self.library.get_books().iter().map(|b| {
					let path = b.get_path();
					let id = b.get_id();
					Command::perform(load_cover_image(path), move |res| {
						Message::ImageLoaded(id, res)
					})
				});
				Command::batch(commands)
			}
			Message::Loaded(Err(e)) => {
				self.state = AppState::Errored(e);
				Command::none()
			}
			Message::ImageLoaded(id, Ok(img)) => {
				self.image_cache.insert(id, img);
				Command::none()
			}
			Message::ImageLoaded(_id, Err(e)) => {
				self.state = AppState::Errored(e);
				Command::none()
			}
			Message::ImportSingleBook => {
				let path = FileDialog::new()
					.add_filter("Book", &["cbz"])
					.show_open_single_file()
					.unwrap();
				if let Some(path) = path {
					let id = self.library.add_book(&path);
					self.state = AppState::EditBook { id };

					if !self.image_cache.contains_key(&id) {
						let path =
							self.library.get_book(&id).unwrap().get_path();
						return Command::perform(
							load_cover_image(path),
							move |res| Message::ImageLoaded(id, res),
						);
					}
				}
				Command::none()
			}
			Message::ImportMultipleBooks => {
				let paths = FileDialog::new()
					.add_filter("Books", &["cbz"])
					.show_open_multiple_file()
					.unwrap();
				let ids = paths
					.iter()
					.map(|p| self.library.add_book(p))
					.filter(|id| !self.image_cache.contains_key(id))
					.collect::<Vec<Uuid>>();

				if ids.is_empty() {
					return Command::none();
				}
				let commands = ids.into_iter().map(|id| {
					let path = self.library.get_book(&id).unwrap().get_path();
					Command::perform(load_cover_image(path), move |res| {
						Message::ImageLoaded(id, res)
					})
				});
				Command::batch(commands)
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
			Message::SaveLibrary => {
				let path = self.library_file.clone();
				match self.library.to_json_bytes() {
					Ok(json) => Command::perform(
						save_library_json(json, path),
						Message::SaveLibraryComplete,
					),
					Err(e) => {
						self.state = AppState::Errored(e);
						Command::none()
					}
				}
			}
			Message::SaveLibraryComplete(Ok(_)) => {
				println!("Library saved");
				Command::none()
			}
			Message::SaveLibraryComplete(Err(e)) => {
				self.state = AppState::Errored(e);
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
			AppState::Loading => Self::loading_view(),
			AppState::Library => self.library_view(),
			AppState::EditBook { id } => self.edit_book_view(
				self.library
					.get_book(id)
					.expect("Should have found book by ID"),
			),
			AppState::Errored(e) => Self::errored_view(e),
		}
		.into()
	}
}

impl<'a> App {
	fn container(title: &str) -> Column<'a, Message> {
		column![text(title).size(50)].spacing(20).padding(20)
	}

	fn loading_view() -> Column<'a, Message> {
		Self::container("Loading").push("Loading")
	}

	fn library_view(&self) -> Column<'a, Message> {
		const BOOK_WIDTH: u16 = 200;

		let mut col = column![].spacing(20).padding([0, 20, 0, 0]);
		let chunk_size = (self.win_width / BOOK_WIDTH as u32).max(1) as usize;
		for chunk in self.library.get_books().chunks(chunk_size) {
			let mut row: Row<'a, Message> = row!().spacing(20);
			for b in chunk {
				row = row.push(
					column![
						container(self.get_image_for_book(b).width(BOOK_WIDTH))
							.center_x()
							.width(BOOK_WIDTH),
						text(b.get_title()).width(Length::Fill)
					]
					.width(Length::Fill),
				);
			}
			for _ in chunk.len()..chunk_size {
				row = row.push(horizontal_space(Length::Fill));
			}
			col = col.push(row);
		}

		Self::container("Library")
			.push(scrollable(col).height(Length::Fill))
			.push(
				row![
					button("Add book").on_press(Message::ImportSingleBook),
					button("Quick Import")
						.on_press(Message::ImportMultipleBooks),
					horizontal_space(Length::Fill),
					button("Save").on_press(Message::SaveLibrary)
				]
				.spacing(20),
			)
	}

	fn edit_book_view(&self, book: &'a Book) -> Column<'a, Message> {
		let label_size = 100;
		Self::container("Add book")
			.push(text(book.get_path_str().to_string()))
			.push(
				row![
					container(self.get_image_for_book(book).width(250))
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

	fn errored_view(e: &'a str) -> Column<'a, Message> {
		Self::container("Error").push(e)
	}

	fn get_image_for_book(&self, book: &Book) -> image::Image {
		self.image_cache
			.get(&book.get_id())
			.map(|i| image(i.clone()))
			.unwrap_or_else(|| {
				image(format!(
					"{}/images/waiting.png",
					env!("CARGO_MANIFEST_DIR")
				))
			})
	}
}
