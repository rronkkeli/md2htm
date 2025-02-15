use std::env;
use std::fs::File;
use std::io::Result;
use std::io::{Read, Write};

mod mdstate;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Something is missing..");
        println!("Try: md2htm [markdown file] [output file]");
    } else {
        let mut md: File = File::open(&args[1])?;
        let mut bytes: Vec<u8> = Vec::with_capacity(16384);
        md.read_to_end(&mut bytes)?;
        let html_bytes: Vec<u8> = mdstate::MDS::parse(bytes);
        let mut html: File = File::create(&args[2])?;
        html.write_all(&html_bytes)?;
        println!("HTML encoded the MD data");
    }

    Ok(())
}
