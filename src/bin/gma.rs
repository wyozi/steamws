use similar::{capture_diff_slices, ChangeTag};
use steamws::gma;

use clap::{Args, Parser, Subcommand};
use globset::Glob;
use sha2::{Digest, Sha256};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::hash::Hasher;
use std::io;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[derive(Parser)]
#[command(author, about, version)]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Prints metadata about given gma
    Info(InfoCommand),
    /// Lists files in given gma
    #[command(alias = "ls")]
    List(ListCommand),
    /// Prints files in given gma
    Cat(CatCommand),
    /// Unpacks gma to a folder
    Unpack(UnpackCommand),
    /// Packs folder into a gma file
    Pack(PackCommand),
    /// Diffs gma against a folder
    Diff(DiffCommand),
}

#[derive(Args)]
struct InfoCommand {
    /// Source gma. Either a file path or - for stdin
    input: String,
}

#[derive(Args)]
struct ListCommand {
    /// Source gma. Either a file path or - for stdin
    input: String,

    /// Sorts the output and includes extra metadata
    #[arg(short)]
    long_format: bool,
}

#[derive(Args)]
struct CatCommand {
    /// Source gma. Either a file path or - for stdin
    input: String,
    /// File pattern of files to print, e.g. "**.lua"
    pattern: Option<String>,
}

#[derive(Args)]
struct UnpackCommand {
    /// Source gma. Either a file path or - for stdin
    input: String,
    /// Output folder for files
    output_folder: String,
    /// File pattern of files to unpack, e.g. "**.lua"
    pattern: Option<String>,
}

#[derive(Args)]
struct PackCommand {
    /// Source folder
    folder: String,

    /// Addon title (included in the gma itself)
    #[arg(short, long)]
    title: Option<String>,

    /// Addon description (included in the gma itself)
    ///
    /// Note that by convention GMAD places a JSON with metadata
    /// in the description string. You should probably not use this
    /// flag unless you know what you're doing
    #[arg(short, long)]
    description: Option<String>,
}

#[derive(Args)]
struct DiffCommand {
    /// Source gma. Either a file path or - for stdin
    input: String,

    /// What to diff against
    target: String,

    /// Flips "old" and "new", so that the input is considered new and target old
    #[arg(short, long)]
    invert: bool,
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
            if t.long_format {
                let gma = gma::read_gma(&t.input, |name| {
                    cfg!(feature = "vtf") && name.ends_with(".vtf")
                });

                let mut entries = gma.entries;
                entries.sort_by(|a, b| a.size.cmp(&b.size));

                for entry in entries {
                    print!("{:8}", steamws::human_readable_size(entry.size));
                    print!("{:40}", entry.name);

                    #[cfg(feature = "vtf")]
                    if entry.name.ends_with(".vtf") && entry.contents.is_some() {
                        let mut contents = entry.contents.unwrap();

                        if let Ok(vtf) = steamws::vtf::from_bytes(&mut contents) {
                            let header = vtf.header;

                            //println!("{:?}", header);
                            print!(
                                "[VTF{}.{}: Highres {:>4}x{:<4} {:10} Lowres {:>4}x{:<4} {:10}]",
                                header.version[0],
                                header.version[1],
                                header.width,
                                header.height,
                                header.highres_image_format,
                                header.lowres_image_width,
                                header.lowres_image_height,
                                header.lowres_image_format,
                            );
                        }
                    }

                    println!();
                }
            } else {
                let gma = gma::read_gma(&t.input, |_| false);

                for entry in gma.entries {
                    println!("{}", entry.name);
                }
            }

            Ok(())
        }
        SubCommand::Cat(t) => {
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

            let addon_json = vec![
                Path::new(&t.folder).join("addon.json"),
                Path::new(&t.folder).join("steamws_addon.json"),
            ]
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
                name: t
                    .title
                    .or_else(|| addon_json.as_ref().map(|a| a.title.clone()))
                    .expect("missing addon title"),
                description: t.description.unwrap_or_else(|| match addon_json {
                    Some(a) => {
                        serde_json::to_string_pretty(&gma::GMADescriptionJson::from_addon(&a))
                            .unwrap()
                    }
                    _ => "{}".to_string(),
                }),
                author: "Author Name".to_string(),
                entries: entries,
            };

            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            gma::write_gma(&g, &mut stdout)?;

            Ok(())
        }
        SubCommand::Diff(t) => {
            let gma = gma::read_gma(&t.input, |_| true);

            #[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
            struct DiffItem {
                name: String,
                hash: String,
            }

            let mut a_items: Vec<DiffItem> = gma
                .entries
                .into_iter()
                .map(|e| {
                    let mut hasher = Sha256::new();
                    hasher.update(&e.contents.unwrap());
                    let hash = hasher.finalize();

                    DiffItem {
                        name: e.name,
                        hash: format!("{:x}", hash),
                    }
                })
                .collect();
            a_items.sort_by(|a, b| a.name.cmp(&b.name));

            let mut b_items: Vec<DiffItem> = Vec::new();

            let iter_start = Path::new(&t.target);
            for entry in walkdir::WalkDir::new(iter_start)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| !e.file_type().is_dir())
            {
                let rel_path = entry.path().strip_prefix(iter_start)?;
                let f_name = String::from(rel_path.to_str().unwrap());

                let mut file = File::open(entry.path()).expect("Unable to open file");

                let mut hasher = Sha256::new();
                io::copy(&mut file, &mut hasher).unwrap();
                let hash = hasher.finalize();

                b_items.push(DiffItem {
                    name: f_name,
                    hash: format!("{:x}", hash),
                });
            }

            b_items.sort_by(|a, b| a.name.cmp(&b.name));

            let ops = capture_diff_slices(similar::Algorithm::Myers, &a_items, &b_items);
            let changes: Vec<_> = ops
                .iter()
                .flat_map(|x| x.iter_changes(&a_items, &b_items))
                .map(|x| (x.tag(), x.value()))
                .collect();

            for change in changes {
                use colored::*;
                let text = match change.0 {
                    ChangeTag::Delete => format!("{}{:?}", "-", change.1).red(),
                    ChangeTag::Insert => format!("{}{:?}", "+", change.1).green(),
                    ChangeTag::Equal => format!("{:?}", change.1).dimmed(),
                };
                println!("{}", text);
            }

            Ok(())
        }
    }
}
