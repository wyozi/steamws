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
    /// Lists material paths used by this .mdl
    /// 
    /// This is very much a best-effort function.
    /// We try to clean up the data and so on, but for
    /// instance all returned materials may not exist.
    #[clap()]
    Materials(MaterialsCommand)
}

#[derive(Clap)]
struct MaterialsCommand {
    /// Source mdl
    input: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Materials(t) => {
            let path = Path::new(&t.input);
            let mut file = File::open(path)?;

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            
            let mdl = steamws::mdl::read(&mut buf)?;
            for mat in mdl.materials {
                let cleaned_up = mat.replace("\\", "/");
                println!("{}", cleaned_up);
            }

            Ok(())
        }
    }
}