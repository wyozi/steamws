use steamws::gma;

use std::io;
use std::fs;
use std::fs::File;
use std::path::{Component, Path};
use clap::Clap;
use globset::Glob;

#[derive(Clap)]
#[clap()]
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
    }
}
