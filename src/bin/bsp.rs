use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::Hasher;
use std::io::{self, BufWriter};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use steamws::bsp::BSPHeader;
use steamws::bsp::{lump_indices::LumpIndex, BSPReader};
use strum::IntoEnumIterator;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, about, version)]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    Info(InfoCommand),

    /// Lists files contained in the Pakfile lump
    #[command(alias = "ls-pak")]
    ListPackedFiles(ListPackedFilesCommand),

    /// Separates input bsp into bsp with entity lump removed and lump file with just the entity lump, WIP!
    ExtractEntityLump(ExtractEntityLumpCommand),
}

#[derive(Args)]
struct InfoCommand {
    /// Source bsp
    input: PathBuf,

    /// Print information about lumps
    /// Includes length, offset, ident
    /// Also calculates and print lump hashes (using arbitrary hashing algorithm, just for comparison purposes)
    #[arg{short, long}]
    lumps: bool,
}

#[derive(Args)]
struct ListPackedFilesCommand {
    /// Source bsp
    input: PathBuf,

    #[arg(short, long)]
    include_size: bool,
}

#[derive(Args)]
struct ExtractEntityLumpCommand {
    /// Source bsp
    input: PathBuf,

    /// Output bsp (with entity lump removed)
    output: PathBuf,

    /// Lump file. By default <output without extension>_l_0.lmp
    lump_output: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Info(t) => {
            let path = Path::new(&t.input);
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let bsp_reader = BSPReader::from_reader(reader)?;

            println!("map version = {}", bsp_reader.header().version);
            println!("map revision = {}", bsp_reader.header().map_revision);

            if t.lumps {
                let bsp = bsp_reader.into_buffered_bsp();
                println!("===");

                for lump_index in LumpIndex::iter() {
                    let mut hasher = DefaultHasher::new();
                    hasher.write(bsp.lump_slice(lump_index));
                    let hash = hasher.finish();

                    let lump = &bsp.header.lumps[lump_index as usize];
                    println!(
                        "lump #{}: off={} len={} ident={:?} hash={:x}",
                        lump_index as usize, lump.off, lump.len, lump.ident, hash
                    );
                }
            }

            Ok(())
        }
        SubCommand::ListPackedFiles(t) => {
            let path = Path::new(&t.input);
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let bsp_reader = BSPReader::from_reader(reader)?;

            let mut archive =
                zip::ZipArchive::new(bsp_reader.into_reader_for_lump(LumpIndex::LUMP_PAKFILE)?)?;
            for i in 0..archive.len() {
                let file = archive.by_index(i)?;
                let outpath = file.enclosed_name().unwrap();

                if !(&*file.name()).ends_with('/') {
                    if t.include_size {
                        println!("{}\t{}", outpath.display(), file.size());
                    } else {
                        println!("{}", outpath.display());
                    }
                }
            }

            Ok(())
        }
        SubCommand::ExtractEntityLump(t) => {
            let path = Path::new(&t.input);
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let mut bsp = BSPReader::from_reader(reader)?.into_buffered_bsp();

            let lump_output = t.lump_output.clone().unwrap_or_else(|| {
                t.output
                    .to_str()
                    .unwrap()
                    .replace(".bsp", "_l_0.lmp")
                    .to_owned()
            });

            // create a new slice where worldspawn (i.e. entity number one) is kept in the entity lump
            let mut slice = bsp.lump_slice(LumpIndex::LUMP_ENTITIES);
            let brace_pos = slice.iter().position(|b| *b == b'}');
            let new_slice = match brace_pos {
                Some(p) => slice[0..p + 1].to_owned(),
                None => Vec::new(),
            };

            // generate extracted lump file
            let lump_header = &bsp.header.lumps[LumpIndex::LUMP_ENTITIES as usize];
            let lump_file = File::create(lump_output)?;
            let mut lump_writer = BufWriter::new(lump_file);

            lump_writer.write_u32::<LittleEndian>(4 * 5)?; // lumpOffset
            lump_writer.write_u32::<LittleEndian>(0)?; // lumpID
            lump_writer.write_u32::<LittleEndian>(lump_header.version)?; // lumpVersion
            lump_writer.write_u32::<LittleEndian>(lump_header.len)?; // lumpLength
            lump_writer.write_u32::<LittleEndian>(bsp.header.map_revision)?; // mapRevision

            io::copy(&mut slice, &mut lump_writer)?;

            // replace bsp entity lump with a stripped one
            bsp.replace_lump(LumpIndex::LUMP_ENTITIES, new_slice);

            let lump_file = File::create(t.output)?;
            let mut lump_writer = BufWriter::new(lump_file);

            bsp.write(&mut lump_writer)?;

            Ok(())
        }
    }
}
