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
    /// Prints metadata about given vtf
    #[clap()]
    Info(InfoCommand)
}

#[derive(Clap)]
struct InfoCommand {
    /// Source vtf
    input: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Info(t) => {
            let path = Path::new(&t.input);
            let mut file = File::open(path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            let vtf = vtf::from_bytes(&mut buf)?;
            println!("{:#?}", vtf.header);

            Ok(())
        }
    }
}