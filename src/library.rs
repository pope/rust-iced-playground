use serde::{Deserialize, Serialize};
use std::fs::File;
use zip::ZipArchive;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Book {
	author: Option<String>,
	path: String,
	tags: Vec<String>,
	title: Option<String>,
	// #[serde(skip)]
	// cover: Vec<u8>,
}

impl Book {
	fn new(path: &str) -> Self {
		Self {
			author: None,
			path: path.to_owned(),
			tags: Vec::new(),
			title: None,
			// cover: Vec::new(),
		}
	}
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Library {
	books: Vec<Book>,
}

impl Library {
	pub async fn load(path: String) -> Result<Self, String> {
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

	pub fn add_book(&mut self, path: &str) {
		let b = Book::new(path);
		self.books.push(b);
	}
}

#[derive(Debug, Clone)]
pub struct Data {
	pub name: String,
	pub size: u64,
}

impl Data {
	pub async fn load(fname: String) -> Result<Self, String> {
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

		Ok(Self {
			name: first,
			size: yeah.size(),
		})
	}
}
