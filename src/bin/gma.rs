use steamws::gma;

use clap::Clap;
use globset::Glob;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

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
    #[clap(alias = "ls")]
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
    input: String,
}

#[derive(Clap)]
struct ListCommand {
    /// Source gma. Either a file path or - for stdin
    input: String,

    /// Include size of each entry in the listing
    #[clap(short)]
    long_format: bool
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
    title: Option<String>,

    /// Addon description (included in the gma itself)
    ///
    /// Note that by convention GMAD places a JSON with metadata
    /// in the description string. You should probably not use this
    /// flag unless you know what you're doing
    #[clap(short, long)]
    description: Option<String>,
}

fn human_readable_filesize(size: u64) -> String {
    if size < 1000 {
        format!("{}B", size)
    } else if size < 1_000_000 {
        format!("{:.2}K", size as f64 / 1_000f64)
    } else {
        format!("{:.2}M", size as f64 / 1_000_000f64)
    }
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
        }
        SubCommand::List(t) => {
            let gma = gma::read_gma(&t.input, |_| false);

            let mut entries = gma.entries;
            entries.sort_by(|a, b| a.size.cmp(&b.size));

            for entry in entries {
                if t.long_format {
                    println!("{:8} {}", human_readable_filesize(entry.size), entry.name);
                } else {
                    println!("{}", entry.name);
                }
            }

            Ok(())
        }
        SubCommand::Cat(t) => {
            if t.input != "-" {
                eprintln!("only - (stdin) argument is supported for input currently");
                std::process::exit(1);
            }

            let does_match: Box<dyn Fn(&str) -> bool> = match t.pattern {
                Some(src) => {
                    let glob = Glob::new(&src).unwrap().compile_matcher();
                    Box::new(move |name| glob.is_match(name))
                }
                _ => Box::new(|_| true),
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
        }
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
                }
                _ => Box::new(|_| true),
            };

            let gma_file = gma::read_gma(&t.input, &does_match);
            for entry in &gma_file.entries {
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

                        let contents = entry.contents.as_ref().unwrap();
                        let mut file = File::create(path).unwrap();
                        io::copy(&mut &contents[..], &mut file).unwrap();
                    }
                }
            }

            if let Some(addon_json) = gma::AddonJson::from_gma_file(&gma_file) {
                let json = serde_json::to_string_pretty(&addon_json).unwrap();
                std::fs::write(output_path.join("steamws_addon.json"), json)?;
            }

            Ok(())
        }
        SubCommand::Pack(t) => {
            fn visit_dir(
                base_path: &Path,
                visit: &Path,
            ) -> Result<Vec<(String, PathBuf)>, io::Error> {
                if visit.is_dir() {
                    Ok(fs::read_dir(visit)?
                        .flat_map(|entry| {
                            let okentry = entry.unwrap();
                            let path = okentry.path();
                            if path.is_dir() {
                                visit_dir(&base_path.join(okentry.file_name()), &path).unwrap()
                            } else {
                                vec![(
                                    base_path
                                        .join(okentry.file_name())
                                        .to_str()
                                        .unwrap()
                                        .to_owned(),
                                    path,
                                )]
                            }
                        })
                        .collect())
                } else {
                    Ok(vec![])
                }
            }

            let addon_json =
                vec!(
                    Path::new(&t.folder).join("addon.json"),
                    Path::new(&t.folder).join("steamws_addon.json")
                )
                .iter()
                .filter_map(|p| {
                    if p.exists() {
                        gma::AddonJson::from_file(&p)
                    } else {
                        None
                    }
                })
                .next();

            let entries = visit_dir(Path::new(""), Path::new(&t.folder))
                .unwrap()
                .iter()
                .filter(|(name, _)| name != "addon.json" && name != "steamws_addon.json")
                .map(|(name, path)| {
                    let mut f = File::open(&path).expect("no file found");
                    let metadata = fs::metadata(&path).expect("unable to read metadata");
                    let mut buffer = vec![0; metadata.len() as usize];
                    f.read(&mut buffer).expect("buffer overflow");

                    gma::GMAEntry {
                        name: name.to_string(),
                        size: buffer.len() as u64,
                        crc: 0,
                        contents: Some(buffer),
                    }
                })
                .collect::<Vec<_>>();

            let g = gma::GMAFile {
                name: t.title.or_else(|| {
                    addon_json.as_ref().map(|a| a.title.clone())
                }).expect("missing addon title"),
                description: t.description.unwrap_or_else(|| {
                    match addon_json {
                        Some(a) => serde_json::to_string_pretty(&gma::GMADescriptionJson::from_addon(&a)).unwrap(),
                        _ => "{}".to_string()
                    }
                }),
                author: "Author Name".to_string(),
                entries: entries,
            };

            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            gma::write_gma(&g, &mut stdout)?;

            Ok(())
        }
    }
}
