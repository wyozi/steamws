use std::fs::File;
use std::path::{Path, PathBuf};
use std::vec::Vec;
use std::fs;
use std::io::Read;

use clap::Clap;

#[derive(Clap)]
#[clap(author, about, version)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Lists all dependencies of this .mdl
    /// 
    /// Namely, this will return a list of paths that
    /// are required for the .mdl to function, including the
    /// .mdl file itself.
    /// 
    /// The paths are returned relative to the working directory
    #[clap(alias = "deps")]
    Dependencies(DependenciesCommand),

    /// Copies given .mdl with dependencies to target path
    /// 
    /// Maintains the folder structure
    #[clap(alias = "cp")]
    Copy(CopyCommand)
}

#[derive(Clap)]
struct DependenciesCommand {
    /// Source mdl
    input: String,
}

#[derive(Clap)]
struct CopyCommand {
    /// Source mdl
    input: String,

    /// Where mdl and dependencies will be placed.
    /// Should be the folder containing "models" and "materials"
    output_folder: String,

    /// Prints what the command would copy if executed without
    /// this flag
    #[clap(long, short = "n")]
    dry_run: bool
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Dependencies(t) => {
            let path = Path::new(&t.input);

            let mdl = steamws::mdl::MDLFile::open(path)?;
            for dep in mdl.dependencies()? {
                println!("{}", dep.to_str().unwrap());
            }

            Ok(())
        },
        SubCommand::Copy(t) => {
            let path = Path::new(&t.input);
            let out_path = Path::new(&t.output_folder);

            let mut copy_map: Vec<(PathBuf, PathBuf)> = vec!();

            let mdl = steamws::mdl::MDLFile::open(path)?;
            let assets_path = mdl.assets_path();
            for dep in &mdl.dependencies()? {
                let bare_dep = dep.strip_prefix(assets_path)?.to_path_buf();
                copy_map.push((dep.to_path_buf(), out_path.join(bare_dep)));
            }

            if t.dry_run {
                println!("Would do following copy operations (dry run): ");
                println!();
            }

            for (from, to) in copy_map {
                println!("{} -> {}", from.display(), to.display());
                if !t.dry_run {
                    fs::create_dir_all(&to.parent().unwrap())?;
                    fs::copy(&from, &to)?;
                }
            }

            Ok(())
        }
    }
}