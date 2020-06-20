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

const SUPPORTED_GMA_VERSION: u8 = 3;

fn read_nt_string<R: Read + BufRead>(handle: &mut R) -> String {
    let mut buf = Vec::new();
    handle.read_until(0, &mut buf).unwrap();

    // don't include null byte
    return std::str::from_utf8(&buf[0..buf.len() - 1])
        .unwrap()
        .to_owned();
}

fn read_gma<R: Read + BufRead>(handle: &mut R) {
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
    println!("{}", name);

    let _addon_version = handle.read_u32::<LittleEndian>().unwrap();

    let mut offset = 0;

    while handle.read_u32::<LittleEndian>().unwrap() != 0 {
        let entry_name = read_nt_string(handle);
        let entry_size = handle.read_i64::<LittleEndian>().unwrap();
        let entry_crc = handle.read_u32::<LittleEndian>().unwrap();
        let entry_offset = offset;

        offset += entry_size;

        println!("{} at {}", entry_name, entry_offset);
    }

    loop {
        let mut buf = [0; 1024];
        let read = handle.read(&mut buf).unwrap();
        if read == 0 {
            break; // EOF
        }
    }
}

fn main() {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::List(t) => {
            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut buf_handle = BufReader::new(handle);
        
            read_gma(&mut buf_handle);
        },
        SubCommand::Cat(t) => {
            let glob = Glob::new(&t.pattern).unwrap().compile_matcher();
            println!("{} = {}", "my/addon/test.lua", glob.is_match("my/addon/test.lua"));
        },
        SubCommand::Unpack(t) => {
            println!("{}", t.pattern);
        },
    }
}
