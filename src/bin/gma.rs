use steamws::gma;

use std::io;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use clap::Clap;
use globset::Glob;

#[derive(Clap)]
#[clap(author, about, version)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Prints metadata about given gma
    #[clap()]
    Info(InfoCommand),
    /// Lists files in given gma
    #[clap()]
    List(ListCommand),
    /// Prints files in given gma
    #[clap()]
    Cat(CatCommand),
    /// Unpacks gma to a folder
    #[clap()]
    Unpack(UnpackCommand),
    /// Packs folder into a gma file
    #[clap()]
    Pack(PackCommand),
}

#[derive(Clap)]
struct InfoCommand {
    /// Source gma. Either a file path or - for stdin
    input: String
}

#[derive(Clap)]
struct ListCommand {
    /// Source gma. Either a file path or - for stdin
    input: String
}

#[derive(Clap)]
struct CatCommand {
    /// Source gma. Either a file path or - for stdin
    input: String,
    /// File pattern of files to print, e.g. "**.lua"
    pattern: Option<String>,
}

#[derive(Clap)]
struct UnpackCommand {
    /// Source gma. Either a file path or - for stdin
    input: String,
    /// Output folder for files
    output_folder: String,
    /// File pattern of files to unpack, e.g. "**.lua"
    pattern: Option<String>,
}

#[derive(Clap)]
struct PackCommand {
    /// Source folder
    folder: String,

    /// Addon title (included in the gma itself)
    #[clap(short, long)]
    title: String,

    /// Addon description (included in the gma itself)
    #[clap(short, long)]
    description: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Info(t) => {
            let gma = gma::read_gma(&t.input, |_| false);
            println!("Name: {}", gma.name);
            println!("Description: {}", gma.description);
            println!("Author: {}", gma.author);
            println!("---");
            for entry in gma.entries {
                println!("{}", entry.name);
            }

            Ok(())
        },
        SubCommand::List(t) => {
            let gma = gma::read_gma(&t.input, |_| false);
            for entry in gma.entries {
                println!("{}", entry.name);
            }

            Ok(())
        },
        SubCommand::Cat(t) => {
            if t.input != "-" {
                eprintln!("only - (stdin) argument is supported for input currently");
                std::process::exit(1);
            }

            let does_match: Box<dyn Fn(&str) -> bool> = match t.pattern {
                Some(src) => {
                    let glob = Glob::new(&src).unwrap().compile_matcher();
                    Box::new(move |name| glob.is_match(name))
                },
                _ => Box::new(|_| true)
            };

            let stdout = io::stdout();
            let mut stdout = stdout.lock();

            let gma = gma::read_gma(&t.input, &does_match);
            for entry in gma.entries {
                if does_match(&entry.name) {
                    let contents = entry.contents.unwrap();
                    io::copy(&mut &contents[..], &mut stdout).unwrap();
                }
            }

            Ok(())
        },
        SubCommand::Unpack(t) => {
            if t.input != "-" {
                eprintln!("only - (stdin) argument is supported for input currently");
                std::process::exit(1);
            }

            let output_path = Path::new(&t.output_folder);
            if !output_path.exists() {
                fs::create_dir(output_path)?;
            }

            let does_match: Box<dyn Fn(&str) -> bool> = match t.pattern {
                Some(src) => {
                    let glob = Glob::new(&src).unwrap().compile_matcher();
                    Box::new(move |name| glob.is_match(name))
                },
                _ => Box::new(|_| true)
            };

            let gma = gma::read_gma(&t.input, &does_match);
            for entry in gma.entries {
                if does_match(&entry.name) {
                    let entry_path = Path::new(&entry.name);
                    if entry_path.is_relative() {
                        let path = output_path.join(entry_path);

                        // Don't allow weird paths with parent components
                        if path.components().any(|c| c == Component::ParentDir) {
                            continue;
                        }

                        let parent = path.parent().unwrap();
                        fs::create_dir_all(parent)?;

                        let contents = entry.contents.unwrap();
                        let mut file = File::create(path).unwrap();
                        io::copy(&mut &contents[..], &mut file).unwrap();
                    }
                }
            }

            Ok(())
        },
        SubCommand::Pack(t) => {

            fn visit_dir(base_path: &Path, visit: &Path) -> Result<Vec<(String, PathBuf)>, io::Error> {
                if visit.is_dir() {
                    Ok(
                        fs::read_dir(visit)?
                        .flat_map(|entry| {
                            let okentry = entry.unwrap();
                            let path = okentry.path();
                            if path.is_dir() {
                                visit_dir(&base_path.join(okentry.file_name()), &path).unwrap()
                            } else {
                                vec!((base_path.join(okentry.file_name()).to_str().unwrap().to_owned(), path))
                            }
                        })
                        .collect()
                    )
                } else {
                    Ok(vec!())
                }
            }
            let entries = visit_dir(Path::new(""), Path::new(&t.folder))
                .unwrap()
                .iter()
                .map(|(name, path)| {
                    let mut f = File::open(&path).expect("no file found");
                    let metadata = fs::metadata(&path).expect("unable to read metadata");
                    let mut buffer = vec![0; metadata.len() as usize];
                    f.read(&mut buffer).expect("buffer overflow");

                    gma::GMAEntry {
                        name: name.to_string(),
                        size: buffer.len() as u64,
                        crc: 0,
                        contents: Some(buffer)
                    }
                })
                .collect::<Vec<_>>();

            let g = gma::GMAFile {
                name: t.title,
                description: t.description,
                author: "Author Name".to_string(),
                entries: entries
            };

            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            gma::write_gma(&g, &mut stdout)?;

            Ok(())
        }
    }
}
