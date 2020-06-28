use std::fs::File;
use std::path::Path;
use std::io::{Read, BufReader};
use std::io;
use byteorder::{LittleEndian, ReadBytesExt};

use clap::Clap;

#[derive(Clap)]
#[clap(author, about, version)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Lists files contained in the Pakfile lump
    #[clap(alias = "ls-pak")]
    ListPackedFiles(ListPackedFilesCommand)
}

#[derive(Clap)]
struct ListPackedFilesCommand {
    /// Source mdl
    input: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::ListPackedFiles(t) => {
            let path = Path::new(&t.input);
            let file = File::open(path)?;
            let mut reader = BufReader::new(file);

            let _ident = reader.read_u32::<LittleEndian>()?;
            let _version = reader.read_u32::<LittleEndian>()?;

            let mut pakfile_off = 0;

            for i in 0..64 {
                let off = reader.read_u32::<LittleEndian>()?;
                let _len = reader.read_u32::<LittleEndian>()?;
                let _vers = reader.read_u32::<LittleEndian>()?;
                let _lump_ident = reader.read_u32::<LittleEndian>()?;

                if i == 40 {
                    pakfile_off = off;
                }
            }
            let _map_revision = reader.read_u32::<LittleEndian>()?;

            let header_len = 1036;
            io::copy(&mut reader.by_ref().take((pakfile_off - header_len).into()), &mut io::sink())?;

            let mut archive = zip::ZipArchive::new(reader)?;
            for i in 0..archive.len() {
                let file = archive.by_index(i)?;
                let outpath = file.sanitized_name();

                if !(&*file.name()).ends_with('/') {
                    println!("{}", outpath.as_path().display());
                }
            }

            Ok(())
        }
    }
}