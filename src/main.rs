use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use crate::library::{load_cover_image, load_images, BookRef, Library};
use clap::Parser;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
	button, column, container, horizontal_space, image, row, scrollable, svg,
	text, text_input, vertical_space, Column, Row,
};
use iced::{
	event, keyboard, subscription, theme, window, Alignment, Application,
	Color, Command, ContentFit, Element, Event, Length, Renderer, Settings,
	Subscription, Theme,
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
	BookDetails {
		book: BookRef,
	},
	EditBook {
		book: BookRef,
	},
	Errored(String),
	Library,
	Loading,
	Viewer {
		book: BookRef,
		cur: usize,
		images: Vec<image::Handle>,
	},
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
	AdvancePage(bool),
	BookAuthorChanged(BookRef, String),
	BookImagesLoaded(BookRef, Result<Vec<image::Handle>, String>),
	BookTitleChanged(BookRef, String),
	CoverImageLoaded(BookRef, Result<image::Handle, String>),
	ImportMultipleBooks,
	ImportSingleBook,
	Loaded(Result<Library, String>),
	OpenBookDetails(BookRef),
	OpenBookViewer(BookRef),
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
			AppState::BookDetails { book } => {
				book.read().unwrap().get_title().to_string()
			}
			AppState::EditBook { .. } => "Add Book".into(),
			AppState::Errored(_) => "Ooops".into(),
			AppState::Library => "Library".into(),
			AppState::Loading => "Loading".into(),
			AppState::Viewer { book, .. } => {
				book.read().unwrap().get_title().to_string()
			}
		};
		format!("{subtitle} - My App")
	}

	fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
		match message {
			Message::AdvancePage(go_forward) => match &mut self.state {
				AppState::Viewer { cur, images, .. } => {
					if !go_forward && *cur > 0 {
						*cur -= 1;
					} else if go_forward && *cur < images.len() - 1 {
						*cur += 1;
					}
					Command::none()
				}
				_ => Command::none(),
			},
			Message::BookAuthorChanged(book, author) => {
				book.write().unwrap().set_author(author);
				Command::none()
			}
			Message::BookImagesLoaded(book, Ok(images)) => {
				match &mut self.state {
					AppState::Viewer {
						book: current_book,
						cur,
						images: current_images,
					} if *current_book.read().unwrap()
						== *book.read().unwrap() =>
					{
						*cur = 0;
						*current_images = images;
						Command::none()
					}
					_ => Command::none(),
				}
			}
			Message::BookImagesLoaded(_book, Err(e)) => {
				self.state = AppState::Errored(e);
				Command::none()
			}
			Message::BookTitleChanged(book, title) => {
				book.write().unwrap().set_title(title);
				Command::none()
			}
			Message::CoverImageLoaded(book, Ok(img)) => {
				let id = { book.read().unwrap().get_id() };
				self.image_cache.insert(id, img);
				Command::none()
			}
			Message::CoverImageLoaded(_book, Err(e)) => {
				self.state = AppState::Errored(e);
				Command::none()
			}
			Message::ImportMultipleBooks => {
				let paths = FileDialog::new()
					.add_filter("Books", &["cbz"])
					.show_open_multiple_file()
					.unwrap();
				let books = paths
					.iter()
					.map(|p| self.library.add_book(p))
					.filter(|b| {
						!self
							.image_cache
							.contains_key(&b.read().unwrap().get_id())
					})
					.collect::<Vec<BookRef>>();

				if books.is_empty() {
					return Command::none();
				}
				let commands = books.into_iter().map(|book| {
					let path = {
						let book = book.read().unwrap();
						book.get_path()
					};
					Command::perform(load_cover_image(path), move |res| {
						Message::CoverImageLoaded(book, res)
					})
				});
				Command::batch(commands)
			}
			Message::ImportSingleBook => {
				let path = FileDialog::new()
					.add_filter("Book", &["cbz"])
					.show_open_single_file()
					.unwrap();
				if let Some(path) = path {
					let book = self.library.add_book(&path);
					self.state = AppState::EditBook {
						book: Arc::clone(&book),
					};

					let (id, path) = {
						let book = book.read().unwrap();
						(book.get_id(), book.get_path())
					};
					if !self.image_cache.contains_key(&id) {
						return Command::perform(
							load_cover_image(path),
							move |res| Message::CoverImageLoaded(book, res),
						);
					}
				}
				Command::none()
			}
			Message::Loaded(Ok(library)) => {
				self.library = library;
				self.state = AppState::Library;

				let commands = self.library.get_books().iter().map(|book| {
					let path = {
						let book = book.read().unwrap();
						book.get_path()
					};
					let book = Arc::clone(book);
					Command::perform(load_cover_image(path), move |res| {
						Message::CoverImageLoaded(book, res)
					})
				});
				Command::batch(commands)
			}
			Message::Loaded(Err(e)) => {
				self.state = AppState::Errored(e);
				Command::none()
			}
			Message::OpenBookDetails(book) => {
				self.state = AppState::BookDetails { book };
				Command::none()
			}
			Message::OpenBookViewer(book) => {
				let path = book.read().unwrap().get_path();
				self.state = AppState::Viewer {
					book: Arc::clone(&book),
					cur: 0,
					images: Vec::new(),
				};
				Command::perform(load_images(path), move |res| {
					Message::BookImagesLoaded(book, res)
				})
			}
			Message::ReturnToLibrary => {
				self.state = AppState::Library;
				Command::none()
			}
			Message::SaveLibrary => Command::perform(
				self.library.clone().save(self.library_file.clone()),
				Message::SaveLibraryComplete,
			),
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
			(
				Event::Keyboard(keyboard::Event::KeyPressed {
					key_code, ..
				}),
				event::Status::Ignored,
			) => match key_code {
				keyboard::KeyCode::Left => Some(Message::AdvancePage(false)),
				keyboard::KeyCode::Right => Some(Message::AdvancePage(true)),
				_ => None,
			},
			_ => None,
		})
	}

	fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
		match &self.state {
			AppState::BookDetails { book } => {
				self.book_details_view(Arc::clone(book)).into()
			}
			AppState::EditBook { book } => {
				self.book_details_view(Arc::clone(book)).into()
			}
			AppState::Errored(e) => Self::errored_view(e).into(),
			AppState::Library => self.library_view().into(),
			AppState::Loading => Self::loading_view().into(),
			AppState::Viewer { book, cur, images } => {
				let img = images.get(*cur);
				self.viewer_view(Arc::clone(book), img).into()
			}
		}
	}
}

impl<'a> App {
	fn container(title: &str) -> Column<'a, Message> {
		column![text(title).size(50)].spacing(20).padding(20)
	}

	fn book_details_view(&self, book: BookRef) -> Row<'a, Message> {
		let mut book_list = column![].spacing(10).padding([0, 20, 0, 0]);
		for book in self.library.get_books() {
			let title = {
				let book = book.read().unwrap();
				book.get_title().to_string()
			};
			let msg = Message::OpenBookDetails(Arc::clone(book));
			book_list = book_list.push(
				button(
					row![
						container(self.get_image_for_book(book).width(50))
							.center_x()
							.width(50),
						text(title).size(16).width(Length::Fill)
					]
					.align_items(Alignment::Center)
					.spacing(6)
					.padding(0),
				)
				.padding(0)
				.on_press(msg)
				.style(theme::Button::Text)
				.width(Length::Fill),
			);
		}

		let label_size = 100;
		let (author, path, title) = {
			let book = book.read().unwrap();
			(
				book.get_author().to_string(),
				book.get_path_str().to_string(),
				book.get_title().to_string(),
			)
		};
		let a_book = Arc::clone(&book);
		let t_book = Arc::clone(&book);
		row![
			scrollable(book_list).width(250),
			Self::container("Book details")
				.push(
					row![
						button(
							container(
								self.get_image_for_book(&book).width(200)
							)
							.center_x()
						)
						.on_press(Message::OpenBookViewer(Arc::clone(&book)))
						.style(theme::Button::Text),
						column![
							row![
								text("Title").width(label_size),
								text_input("Enter a title...", &title)
									.on_input(move |title| {
										let book = t_book.clone();
										Message::BookTitleChanged(book, title)
									})
							]
							.spacing(20)
							.align_items(Alignment::Center),
							row![
								text("Author").width(label_size),
								text_input("Enter an author...", &author)
									.on_input(move |author| {
										let book = a_book.clone();
										Message::BookAuthorChanged(book, author)
									})
							]
							.spacing(20)
							.align_items(Alignment::Center),
							row![text("Path").width(label_size), text(path)]
								.spacing(20)
								.align_items(Alignment::Center)
						]
						.spacing(20),
					]
					.spacing(20),
				)
				.push(vertical_space(Length::Fill))
				.push(button("Back").on_press(Message::ReturnToLibrary))
				.width(Length::Fill)
		]
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
				let title = {
					let book = b.read().unwrap();
					book.get_title().to_string()
				};
				let msg = Message::OpenBookDetails(Arc::clone(b));
				row = row.push(
					button(column![
						container(self.get_image_for_book(b).width(BOOK_WIDTH))
							.center_x()
							.width(BOOK_WIDTH),
						text(title).width(Length::Fill)
					])
					.padding(0)
					.on_press(msg)
					.style(theme::Button::Text)
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

	fn errored_view(e: &'a str) -> Column<'a, Message> {
		Self::container("Error").push(e)
	}

	fn viewer_view(
		&self,
		book: BookRef,
		img: Option<&'a image::Handle>,
	) -> Column<'a, Message> {
		let back_msg = Message::OpenBookDetails(book);
		column![
			row![
				button(
					container(
						svg(svg::Handle::from_path(format!(
							"{}/images/nav_before.svg",
							env!("CARGO_MANIFEST_DIR")
						)))
						.style(theme::Svg::custom_fn(|theme| {
							svg::Appearance {
								color: Some(theme.palette().text),
							}
						}))
						.width(48)
						.height(48)
					)
					.width(Length::Fill)
					.height(Length::Fill)
					.align_x(Horizontal::Left)
					.align_y(Vertical::Center)
				)
				.style(theme::Button::Text)
				.width(Length::Fill)
				.on_press(Message::AdvancePage(false)),
				img.map(|img| image(img.clone()))
					.unwrap_or_else(|| {
						image(format!(
							"{}/images/waiting.png",
							env!("CARGO_MANIFEST_DIR")
						))
					})
					.content_fit(ContentFit::ScaleDown),
				button(
					container(
						svg(svg::Handle::from_path(format!(
							"{}/images/nav_next.svg",
							env!("CARGO_MANIFEST_DIR")
						)))
						.style(theme::Svg::custom_fn(|theme| {
							svg::Appearance {
								color: Some(theme.palette().text),
							}
						}))
						.width(48)
						.height(48)
					)
					.width(Length::Fill)
					.height(Length::Fill)
					.align_x(Horizontal::Right)
					.align_y(Vertical::Center)
				)
				.style(theme::Button::Text)
				.width(Length::Fill)
				.on_press(Message::AdvancePage(true)),
			]
			.height(Length::Fill),
			button("Back").on_press(back_msg)
		]
		.spacing(20)
		.padding(20)
		.width(Length::Fill)
	}

	fn get_image_for_book(&self, book: &BookRef) -> image::Image {
		let id = { book.read().unwrap().get_id() };
		self.image_cache
			.get(&id)
			.map(|i| image(i.clone()))
			.unwrap_or_else(|| {
				image(format!(
					"{}/images/waiting.png",
					env!("CARGO_MANIFEST_DIR")
				))
			})
	}
}
