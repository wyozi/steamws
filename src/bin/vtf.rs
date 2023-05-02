use std::fs::File;
use std::path::Path;
use std::vec::Vec;
use std::io::Read;

use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(author, about, version)]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Prints metadata about given vtf
    #[command()]
    Info(InfoCommand)
}

#[derive(Args)]
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