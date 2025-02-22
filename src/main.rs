pub(crate) use std::{
    env,
    fs::{remove_file, File},
    io::{Read, Result, Write},
    os::unix::net::{UnixListener, UnixStream},
    thread::spawn,
};

mod mdstate;
mod writeto;

const PS: usize = std::mem::size_of::<usize>();
const SOCK: &str = "/run/mdserv/mdserv.sock";

fn main() -> Result<()> {
    // Try to remove the socket file but don't really care about the outcome,
    // because the binding won't succeed if there is no privileges to write.
    match remove_file(SOCK) {
        _ => {}
    };
    let args: Vec<String> = env::args().collect();
    handle_args(args)?;
    Ok(())
}

fn stream_handler(mut stream: UnixStream) {
    let mut lbuf: [u8; PS] = [0; PS];

    // These matches are just for debugging purposes
    // will tidy up later..
    match stream.read(&mut lbuf) {
        Ok(_) => {
            let len: usize = usize::from_be_bytes(lbuf);
            let mut mdbuf: Vec<u8> = vec![0; len];

            match stream.read(&mut mdbuf) {
                Ok(_) => {
                    let parsed = mdstate::MDS::parse(mdbuf);
                    let plen: [u8; PS] = parsed.len().to_be_bytes();

                    match stream.write(&plen) {
                        Ok(_) => match stream.write(&parsed) {
                            Ok(_) => match stream.flush() {
                                Ok(_) => return,
                                Err(e) => eprintln!("Flushing wasn't successful: {e}"),
                            },

                            Err(e) => eprintln!("Couldn't write the parsed data: {e}"),
                        },

                        Err(e) => eprintln!("Couldn't write the length bytes: {e}"),
                    }
                }

                Err(e) => eprintln!("Failed to read the {len} message bytes: {e}"),
            }
        }

        Err(e) => eprintln!("Failed to read the length of the message: {e}"),
    }
}

fn handle_args(args: Vec<String>) -> Result<()> {
    if args.len() == 1 {
        eprintln!("Expected at least one argument!");
        print_help();
        return Ok(());
    }

    match args[1].as_str() {
        "help" | "--help" | "-h" | "h" | "?" => {
            print_help();
        }

        "daemon" | "d" | "--daemon" | "-d" => {
            if args.len() == 2 {
                let listener: UnixListener = UnixListener::bind(SOCK)?;

                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            spawn(|| {
                                stream_handler(stream);
                            });
                        }

                        Err(e) => eprintln!("Failed to catch the stream: {e}"),
                    }
                }
            } else {
                eprintln!("Daemon mode doesn't take arguments.");
            }
        }

        _ => match args.len() {
            2 => {
                let mut dst: String;

                if args[1].find(".md").is_some_and(|x| x == args[1].len() - 3) {
                    dst = args[1].replace(".md", ".html");
                } else {
                    dst = args[1].clone();
                    dst.push_str(".html");
                }

                parse(&args[1], &dst)?;
            }

            3 => parse(&args[1], &args[2])?,

            _ => eprintln!("Too many arguments! Expected at most 2."),
        },
    }

    Ok(())
}

/// Parse source file into destination file
fn parse<P: AsRef<std::path::Path>>(src: P, dst: P) -> Result<()> {
    let mut infile: File = File::open(src)?;
    let mut markdown: Vec<u8> = Vec::with_capacity(16 * 1024);
    infile.read_to_end(&mut markdown)?;
    let output: Vec<u8> = mdstate::MDS::parse(markdown);
    let mut outfile: File = File::create(dst)?;
    outfile.write_all(&output)?;
    println!("Target parsed!");
    Ok(())
}

fn print_help() {
    println!(
        "Usage md2htm [daemon|source file|help] [[output file]]

    Parses Markdown to HTML without adding any of the root tags.

    help, --help, h, -h, ?      Show this help and exit.

    daemon, --daemon, d, -d     Start the program in daemon mode that listens a socket in {}.
                                If given, no other arguments are expected.

    [source file]               The path of the source file containing the Markdown text.
                                Doesn't expect a file extension '.md' or anything else.

    [output file]               Optional. The path of the output file. If omitted,
                                the program uses the same path as the source file,
                                but replaces/appends the file extention to .html.
                                Doesn't expect the file extension '.html'.

    Examples:

    To parse a file named markdown.md into webpage.html, when both are in local directory:
    md2htm markdown.md webpage.html

    To parse file named markdown.md into markdown.html, when source file is in local directory:
    md2htm markdown.md

    To run this program in daemon mode, any of these commands will do:
    md2htm daemon
    md2htm --daemon
    md2htm d
    md2htm -d

    If the program doesn't have sufficient privileges to remove the socket file,
    it can be removed manually with:
    sudo rm {}

    Bugs and issues should be reported in https://github.com/rronkkeli/md2htm",
        SOCK, SOCK
    );
}
