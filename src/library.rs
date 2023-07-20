use iced::widget::image;
use serde::{Deserialize, Serialize};
use std::{
	fs::File,
	io::Read,
	path::{Path, PathBuf},
};
use uuid::Uuid;
use zip::ZipArchive;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Book {
	id: Uuid,
	author: Option<String>,
	path: PathBuf,
	tags: Vec<String>,
	title: Option<String>,
	// #[serde(skip)]
	// cover: Option<image::Handle>,
}

impl Book {
	fn new(path: &Path) -> Self {
		Self {
			id: Uuid::new_v4(),
			author: None,
			path: path.to_path_buf(),
			tags: Vec::new(),
			title: None,
			// cover: None,
		}
	}

	pub fn get_id(&self) -> Uuid {
		self.id
	}

	pub fn get_path_str(&self) -> &str {
		self.path.to_str().unwrap_or_default()
	}

	pub fn get_path(&self) -> PathBuf {
		self.path.clone()
	}

	pub fn get_title(&self) -> &str {
		self.title
			.as_ref()
			.map(|t| t.as_ref())
			.or_else(|| self.path.file_stem().and_then(|stem| stem.to_str()))
			.unwrap_or_default()
	}

	pub fn set_title(&mut self, title: String) {
		self.title = Some(title);
	}

	pub fn get_author(&self) -> &str {
		self.author.as_ref().map(|a| a.as_ref()).unwrap_or_default()
	}

	pub fn set_author(&mut self, author: String) {
		self.author = Some(author);
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Library {
	version: String,
	books: Vec<Book>,
}

impl Library {
	pub async fn load(path: PathBuf) -> Result<Self, String> {
		if let Ok(b) = tokio::fs::read(&path).await {
			return serde_json::from_slice(&b).map_err(|err| {
				let msg = "Unable to parse JSON file";
				eprintln!("{msg}: {err}");
				msg.to_owned()
			});
		}

		let lib = Self::default();
		let b = serde_json::to_vec_pretty(&lib).map_err(|err| {
			let msg = "Unable to serialize library";
			eprintln!("{msg}: {err}");
			msg
		})?;
		match tokio::fs::write(&path, &b).await {
			Ok(_) => Ok(lib),
			Err(err) => {
				let msg = "Unable to save library file";
				eprintln!("{msg}: {err}");
				Err(msg.to_owned())
			}
		}
	}

	pub fn get_books(&self) -> &Vec<Book> {
		&self.books
	}

	pub fn add_book(&mut self, path: &Path) -> Uuid {
		let b = Book::new(path);
		let id = b.id;
		self.books.push(b);
		id
	}

	pub fn get_book(&self, id: &Uuid) -> Option<&Book> {
		self.books.iter().find(|b| b.id == *id)
	}

	pub fn get_book_mut(&mut self, id: &Uuid) -> Option<&mut Book> {
		self.books.iter_mut().find(|b| b.id == *id)
	}
}

impl Default for Library {
	fn default() -> Self {
		Self {
			version: "1.0".to_owned(),
			books: Vec::new(),
		}
	}
}

pub async fn load_cover_image(path: PathBuf) -> Result<image::Handle, String> {
	let zipfile = File::open(path).map_err(|_| "Failed to read cbz file")?;
	let mut archive =
		ZipArchive::new(zipfile).map_err(|_| "Unable to process cbz file")?;

	let first = archive
		.file_names()
		.filter(|f| {
			f.ends_with(".jpeg") || f.ends_with(".jpg") || f.ends_with(".png")
		})
		.reduce(|res, f| if f < res { f } else { res })
		.ok_or("Unable to find an image in the cbz file")?
		.to_owned();

	let mut img_file = archive.by_name(&first).unwrap();
	let mut b = Vec::new();
	img_file
		.read_to_end(&mut b)
		.map_err(|_| "Unable to read bytes")?;

	let img = ::image::load_from_memory(&b)
		.map_err(|_| "Unable to processes image")?;
	let img = img.resize(250, 350, ::image::imageops::FilterType::Triangle);
	Ok(image::Handle::from_pixels(
		img.width(),
		img.height(),
		img.into_rgba8().to_vec(),
	))
}
