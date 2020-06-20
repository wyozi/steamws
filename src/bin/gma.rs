use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io;
use std::io::{BufRead, BufReader, Read};
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
    #[clap()]
    List(ListCommand),
    #[clap()]
    Cat(CatCommand),
    #[clap()]
    Unpack(UnpackCommand),
}

#[derive(Clap)]
struct ListCommand {
}

#[derive(Clap)]
struct CatCommand {
    pattern: String
}

#[derive(Clap)]
struct UnpackCommand {
    input: String,
    output_folder: String,
    pattern: Option<String>,
}

struct GMAFile {
    entries: Vec<GMAEntry>
}
struct GMAEntry {
    name: String,
    offset: u64,
    size: u64,
    contents: Option<Vec<u8>>
}

const SUPPORTED_GMA_VERSION: u8 = 3;

fn read_nt_string<R: Read + BufRead>(handle: &mut R) -> String {
    let mut buf = Vec::new();
    handle.read_until(0, &mut buf).unwrap();

    // don't include null byte
    return std::str::from_utf8(&buf[0..buf.len() - 1])
        .unwrap()
        .to_owned();
}

fn read_gma<R: Read + BufRead, F>(handle: &mut R, read_entry: F) -> GMAFile where
    F: Fn(&str) -> bool {
    
    let mut magic_buf = [0; 4];
    handle.read_exact(&mut magic_buf).unwrap();

    if &magic_buf != b"GMAD" {
        eprintln!("header not GMAD??");
        std::process::exit(1);
    }

    let fmt_version = handle.read_u8().unwrap();
    if fmt_version != SUPPORTED_GMA_VERSION {
        eprintln!("unsupported gma version");
        std::process::exit(1);
    }

    let _steamid = handle.read_u64::<LittleEndian>().unwrap();
    let _timestamp = handle.read_u64::<LittleEndian>().unwrap();

    let mut dumb_string = read_nt_string(handle);
    while dumb_string.len() > 0 {
        dumb_string = read_nt_string(handle);
    }

    let name = read_nt_string(handle);
    let desc = read_nt_string(handle);
    let author = read_nt_string(handle);

    let _addon_version = handle.read_u32::<LittleEndian>().unwrap();

    let mut entries = vec!();
    let mut offset = 0;

    while handle.read_u32::<LittleEndian>().unwrap() != 0 {
        let entry_name = read_nt_string(handle);
        let entry_size = handle.read_i64::<LittleEndian>().unwrap();
        let entry_crc = handle.read_u32::<LittleEndian>().unwrap();
        let entry_offset = offset;

        offset += entry_size;

        let mut entry = GMAEntry {
            name: entry_name,
            offset: entry_offset as u64,
            size: entry_size as u64,
            contents: None
        };
        entries.push(entry);
    }

    // Read file contents
    for mut e in &mut entries {
        if read_entry(&e.name) {
            let mut buf = vec![0; e.size as usize];
            handle.read_exact(&mut buf);
            e.contents = Some(buf);
        } else {
            // Pipe to sink
            io::copy(&mut handle.take(e.size), &mut io::sink());
        }
    }

    loop {
        let mut buf = [0; 1024];
        let read = handle.read(&mut buf).unwrap();
        if read == 0 {
            break; // EOF
        }
    }

    GMAFile {
        entries: entries
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::List(t) => {
            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut buf_handle = BufReader::new(handle);
        
            let gma = read_gma(&mut buf_handle, |_| false);
            for entry in gma.entries {
                println!("{}", entry.name);
            }

            Ok(())
        },
        SubCommand::Cat(t) => {
            let glob = Glob::new(&t.pattern).unwrap().compile_matcher();

            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut buf_handle = BufReader::new(handle);
        
            let stdout = io::stdout();
            let mut stdout = stdout.lock();

            let gma = read_gma(&mut buf_handle, |name| glob.is_match(name));
            for entry in gma.entries {
                if glob.is_match(&entry.name) {
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

            let does_match: Box<Fn(&str) -> bool> = match t.pattern {
                Some(src) => {
                    let glob = Glob::new(&src).unwrap().compile_matcher();
                    Box::new(move |name| glob.is_match(name))
                },
                _ => Box::new(|_| true)
            };

            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut buf_handle = BufReader::new(handle);

            let gma = read_gma(&mut buf_handle, &does_match);
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
