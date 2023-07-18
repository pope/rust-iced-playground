use clap::Parser;
use iced::widget::{column, text};
use iced::{Alignment, Application, Command, Element, Settings, Theme};
use std::fs::File;
use zip::ZipArchive;

fn main() -> iced::Result {
	let flags = Flags::parse();
	App::run(Settings::with_flags(flags))
}

#[derive(Parser)]
struct Flags {
	/// The location of a .cbz file.
	cbz_file: String,
}

#[derive(Debug)]
enum App {
	Loading,
	Loaded(Data),
	Errored(String),
}

#[derive(Debug, Clone)]
enum Message {
	Loaded(Result<Data, String>),
}

impl Application for App {
	type Executor = iced::executor::Default;
	type Flags = Flags;
	type Message = Message;
	type Theme = Theme;

	fn new(flags: Flags) -> (App, Command<Message>) {
		(
			App::Loading,
			Command::perform(Data::load(flags.cbz_file), Message::Loaded),
		)
	}

	fn title(&self) -> String {
		let subtitle = match self {
			App::Loading => "Loading",
			App::Loaded(_) => "Loaded",
			App::Errored(_) => "Ooops",
		};
		format!("{subtitle} - My App")
	}

	fn update(&mut self, message: Message) -> Command<Message> {
		match message {
			Message::Loaded(Ok(data)) => {
				*self = App::Loaded(data);
				Command::none()
			}
			Message::Loaded(Err(e)) => {
				*self = App::Errored(e);
				Command::none()
			}
		}
	}

	fn view(&self) -> Element<Message> {
		let message = match self {
			App::Loading => "loading...".to_owned(),
			App::Loaded(data) => format!("{} - {}b big", data.name, data.size),
			// Interestingly enough, the emoji doesn't print on the screen
			App::Errored(e) => format!("⚠️  {e}"),
		};
		column![text(message).size(50)]
			.padding(50)
			.align_items(Alignment::Center)
			.into()
	}
}

#[derive(Debug, Clone)]
struct Data {
	name: String,
	size: u64,
}

impl Data {
	async fn load(fname: String) -> Result<Self, String> {
		let zipfile =
			File::open(fname).map_err(|_| "Failed to read cbz file")?;
		let mut archive = ZipArchive::new(zipfile)
			.map_err(|_| "Unable to process cbz file")?;

		let first = archive
			.file_names()
			.filter(|f| f.ends_with(".jpeg"))
			.reduce(|res, f| if f < res { f } else { res })
			.ok_or("Unable to find an image in the cbz file")?
			.to_owned();

		let yeah = archive.by_name(&first).unwrap();

		Ok(Data {
			name: first,
			size: yeah.size(),
		})
	}
}
