use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io;
use std::io::{BufRead, BufReader, Read};
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
    pattern: String,
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

fn main() {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::List(t) => {
            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut buf_handle = BufReader::new(handle);
        
            let f = read_gma(&mut buf_handle, |_| false);
            for entry in f.entries {
                println!("{}", entry.name);
            }
        },
        SubCommand::Cat(t) => {
            let glob = Glob::new(&t.pattern).unwrap().compile_matcher();

            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut buf_handle = BufReader::new(handle);
        
            let stdout = io::stdout();
            let mut stdout = stdout.lock();

            let f = read_gma(&mut buf_handle, |name| glob.is_match(name));
            for entry in f.entries {
                if glob.is_match(&entry.name) {
                    let contents = entry.contents.unwrap();
                    io::copy(&mut &contents[..], &mut stdout).unwrap();
                }
            }
        },
        SubCommand::Unpack(t) => {
            println!("{}", t.pattern);
        },
    }
}
