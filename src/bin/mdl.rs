use std::fs::File;
use std::path::Path;
use std::vec::Vec;
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
    /// are required for the .mdl to function.
    /// 
    /// The paths are returned relative to the working directory
    #[clap(alias = "deps")]
    Dependencies(DependenciesCommand)
}

#[derive(Clap)]
struct DependenciesCommand {
    /// Source mdl
    input: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Dependencies(t) => {
            let path = Path::new(&t.input);

            let mdl = steamws::mdl::MDLFile::open(path)?;
            for dep in mdl.dependencies()? {
                println!("{}", dep);
            }

            Ok(())
        }
    }
}