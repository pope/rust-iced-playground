use clap::Parser;
use std::fs::File;
use std::path::Path;
use zip::ZipArchive;

#[derive(Parser)]
struct Args {
	/// The location of a .cbz file.
	cbz_file: String,
}

fn main() {
	let args = Args::parse();
	let fname = Path::new(&args.cbz_file);
	let zipfile = File::open(fname).unwrap();
	let mut archive = ZipArchive::new(zipfile).unwrap();

	let first = archive
		.file_names()
		.filter(|f| f.ends_with(".jpeg"))
		.reduce(|res, f| if f < res { f } else { res })
		.unwrap()
		.to_owned();
	println!("First image: {}", first);

	let yeah = archive.by_name(&first).unwrap();
	println!("It is {}b big", yeah.size());
}
