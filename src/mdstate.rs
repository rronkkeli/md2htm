//! This module converts markdown to html without the root elements.

use crate::writeto::*;
use std::boxed::Box;

const TAG_P_O: &[u8; 3] = b"<p>";
const TAG_P_C: &[u8; 4] = b"</p>";
const TAG_CODEB_O: &[u8; 37] = b"<div class=\"code\"><code class=\"code\">";
const TAG_CODEB_C: &[u8; 13] = b"</code></div>";
const TAG_CODEI_O: &[u8; 38] = b"<span class=\"code\"><code class=\"code\">";
const TAG_CODEI_C: &[u8; 14] = b"</code></span>";
const TAG_INT_O: &[u8; 20] = b"<div class=\"intend\">";
const TAG_INT_C: &[u8; 6] = b"</div>";
const TAG_I_O: &[u8; 3] = b"<i>";
const TAG_I_C: &[u8; 4] = b"</i>";
const TAG_B_O: &[u8; 3] = b"<b>";
const TAG_B_C: &[u8; 4] = b"</b>";
const TAG_U_O: &[u8; 3] = b"<u>";
const TAG_U_C: &[u8; 4] = b"</u>";
const TAG_LI_O: &[u8; 4] = b"<li>";
const TAG_LI_C: &[u8; 5] = b"</li>";
const TAG_UL_O: &[u8; 4] = b"<ul>";
const TAG_UL_C: &[u8; 5] = b"</ul>";
const TAG_HR: &[u8; 4] = b"<hr>";


/// Markdown states
#[derive(Debug)]
enum State {
    /// The state machine hasn't encountered any keys yet
    None,
    /// Number in the Header signifies the level of the header. True implies
    /// that header start tag has been placed.
    Header(u8, bool),
    Paragraph,
    /// True if expecting a new line or space
    Intendation(bool, IntenData),
    /// True if bold state expects a closure. In other words the parser has seen first `*`
    /// character and is aticipating the next one in next byte.
    Bold(bool),
    /// True signifies that there has been a * symbol just before.
    /// Should be switched to false immediately after any other character
    /// has been identified.
    Italic(bool),
    Underscore,
    /// Counts the ` characters if they are in a sequence. True if the previous
    /// character was `, otherwise false.
    Code(bool, u8),
    Link(Linkdata),
    Exclamation,
    Image(Linkdata),
    Escape,
    /// 1st true if seen a '-' previously. 2nd true if the list tag has been placed.
    UList(bool, bool),
    LItem,
    Hor(u8),
}

#[derive(Debug)]
struct IntenData {
    inner: Vec<u8>,
}

#[derive(Debug)]
struct Linkdata {
    status: Linkstatus,
    alt: Vec<u8>,
    link: Vec<u8>,
}

#[derive(Debug)]
enum Linkstatus {
    /// 0 = `[` has been seen, 1 = `]` has been seen and `(` is being expected in next byte
    Alt(u8),
    Link,
}

impl Linkdata {
    /// Checks if the linkstatus is Alt
    fn is_alt(&self) -> bool {
        self.status.is_alt()
    }

    /// Checks if the linkstatus is Link
    fn is_link(&self) -> bool {
        self.status.is_link()
    }

    fn alt_expects_closure(&self) -> bool {
        self.status.alt_expects_closure()
    }

    fn alt_expects_url(&self) -> bool {
        self.status.alt_expects_url()
    }
}

impl Linkstatus {
    /// Checks if the linkstatus is Alt
    fn is_alt(&self) -> bool {
        match self {
            Self::Alt(_) => true,
            Self::Link => false,
        }
    }

    /// Checks if a `]` is being expected at some point
    fn alt_expects_closure(&self) -> bool {
        match self {
            Self::Alt(0) => true,
            _ => false,
        }
    }

    fn alt_expects_url(&self) -> bool {
        match self {
            Self::Alt(1) => true,
            _ => false,
        }
    }

    /// Checks if the linkstatus is Link
    fn is_link(&self) -> bool {
        match self {
            Self::Alt(_) => false,
            Self::Link => true,
        }
    }
}

/// Markdown State machine contains a linked list of current states.
/// Once a state has been handled, the state goes to previous and continues
/// handling it. States need to be ended in the reverse order they have been
/// invoked so it makes sense to trave backwards to the root state.
pub struct MDS {
    current: State,
    previous: Option<Box<Self>>,
}

impl MDS {
    pub fn parse(bytes: Vec<u8>) -> Vec<u8> {
        let mut state_machine: MDS = Self {
            current: State::None,
            previous: Option::None,
        };

        // HTML data output will be larger than Markdown data,
        // so output buffer may be larger than the input buffer.
        // This makes reallocation unlikely, resulting in faster
        // processing speed.
        let mut output: Vec<u8> = Vec::with_capacity(bytes.capacity() << 1);

        let mut line_counter: usize = 1;
        // Counts the current bytes that are not new lines or carriage returns, on the line.
        let mut column_counter: usize = 0;

        for byte in bytes {
            match byte {
                0..10 | 11..13 | 14..32 | 34..35 | 36..40 | 43..45 | 46..91 | 97..=255 => {
                    match state_machine.current {
                        State::None => {
                            state_machine = state_machine.rise(State::Paragraph);
                            output.write(TAG_P_O);
                            output.push(byte);
                        }

                        State::Code(ls, n) => {
                            if ls {
                                match n {
                                    1 => {
                                        state_machine.current = State::Code(false, n);
                                        // Open inline code span tag and code tag
                                        output.write(TAG_CODEI_O);
                                    }

                                    3 => {
                                        state_machine.current = State::Code(false, n);
                                        // Open code block div tag and code tag
                                        output.write(TAG_CODEB_O);
                                    }

                                    _ => {
                                        println!("Warning: Unexpected code block state! Undefined behaviour may occur! Trying to mitigate damage by ignoring previous key on line {} column {}..", line_counter, column_counter);
                                        state_machine = state_machine.fall();
                                    }
                                }
                            }
                            output.push(byte);
                        }

                        State::Escape => {
                            match byte {
                                b'<' => output.write(b"&lt;"),
                                b'>' => output.write(b"&gt;"),
                                _ => output.push(byte),
                            }

                            state_machine = state_machine.fall();
                        }

                        State::Exclamation => {
                            output.push(b'!');
                            output.push(byte);
                            state_machine = state_machine.fall();
                        }

                        State::Link(ref mut ld) | State::Image(ref mut ld) => match ld.status {
                            Linkstatus::Alt(0) => {
                                ld.alt.push(byte);
                            }

                            Linkstatus::Alt(1) => {
                                output.push(b'[');
                                output.write(&ld.alt);
                                output.push(b']');
                                output.push(byte);
                                state_machine = state_machine.fall();
                            }

                            Linkstatus::Link => {
                                ld.link.push(byte);
                            }

                            _ => {
                                println!("Warning: Unexpected link status. This shouldn't happen.");
                            }
                        },

                        State::Intendation(exp, ref mut buf) => {
                            if exp {
                                // Close intend div tag
                                output.write(TAG_INT_C);
                                // Write the buffer of intendation
                                output.write(&buf.inner);
                                state_machine = state_machine.fall();
                            } else {
                                output.write(&buf.inner);
                                buf.inner.clear();
                            }

                            output.write(TAG_P_O);
                            output.push(byte);
                            state_machine = state_machine.rise(State::Paragraph);
                        }

                        State::Italic(seen) => {
                            if seen {
                                // Open i tag
                                output.write(TAG_I_O);
                                state_machine.current = State::Italic(false);
                            }

                            output.push(byte);
                        }

                        State::Bold(seen) => {
                            if seen {
                                eprintln!("Warning: Non-escaped `*` in the middle of bolded on line {} column {}. Parsing it as a literal..",
                                         line_counter, column_counter);
                                output.push(b'*');
                                state_machine.current = State::Bold(false);
                            }

                            output.push(byte);
                        }

                        State::UList(seen, written) => {
                            if seen {
                                eprintln!("Unexpected character when expecting a space on line {} column {}",
                                          line_counter, column_counter);
                            }

                            if written {
                                output.write(TAG_UL_C);
                            }

                            output.write(TAG_P_C);
                            state_machine = state_machine.fall().fall();

                            match state_machine.current {
                                State::Intendation(_, ref buf) => {
                                    output.write(TAG_INT_C);
                                    output.write(&buf.inner);
                                    state_machine = state_machine.fall();
                                }
                                _ => {}
                            }

                            output.write(TAG_P_O);
                            output.push(byte);

                            state_machine = state_machine.rise(State::Paragraph);
                        }

                        _ => output.push(byte),
                    }
                }

                b'!' => match state_machine.current {
                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Exclamation | State::Link(_) | State::Image(_) | State::Code(_, _) => {
                        output.push(byte);
                    }

                    State::Intendation(exp, ref buf) => {
                        if exp {
                            // Close intend div tag
                            output.write(TAG_INT_C);
                            output.write(&buf.inner);
                            state_machine = state_machine.fall();
                        }

                        state_machine = state_machine.rise(State::Exclamation);
                    }

                    _ => {
                        state_machine = state_machine.rise(State::Exclamation);
                    }
                },

                b'\\' => match state_machine.current {
                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        state_machine = state_machine.fall().rise(State::Escape);
                    }

                    _ => state_machine = state_machine.rise(State::Escape),
                },

                b'#' => match state_machine.current {
                    State::None => state_machine = state_machine.rise(State::Header(1, false)),

                    State::Intendation(exp, ref buf) => {
                        if exp {
                            // Close intend div tag
                            output.write(TAG_INT_C);
                            output.write(&buf.inner);
                            state_machine = state_machine.fall();
                        }
                        state_machine = state_machine.rise(State::Header(1, false));
                    }

                    State::Header(n, p) => {
                        if n < 6 {
                            state_machine.current = State::Header(n + 1, p);
                        } else {
                            println!("Trying to exceed html header level 6. Ignoring excess header keys..");
                        }
                    }

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Code(ls, n) => {
                        if ls {
                            match n {
                                1 => {
                                    state_machine.current = State::Code(false, n);

                                    // Open inline code span tag and code tag
                                    output.write(TAG_CODEI_O);
                                }

                                3 => {
                                    // Open code block div tag and code tag
                                    output.write(TAG_CODEB_O);
                                    state_machine.current = State::Code(false, n);
                                }

                                _ => {
                                    println!("Warning: Unexpected code block state! Undefined behaviour may occur! Trying to mitigate damage by ignoring previous key..");

                                    output.push(byte);
                                    state_machine = state_machine.fall();
                                }
                            }
                        }
                        output.push(byte);
                    }

                    State::Link(ref mut ld) | State::Image(ref mut ld) => match ld.status {
                        Linkstatus::Alt(0) => {
                            ld.alt.push(byte);
                        }

                        Linkstatus::Link => {
                            ld.link.push(byte);
                        }

                        _ => {
                            output.push(b'[');
                            output.write(&ld.alt);
                            output.push(b']');
                            output.push(b'(');
                            output.write(&ld.link);
                            output.push(byte);
                            state_machine = state_machine.fall();
                        }
                    },

                    _ => {
                        output.push(byte);
                    }
                },

                b' ' => match state_machine.current {
                    State::None => {
                        // Open intend div tag
                        output.write(TAG_INT_O);
                        state_machine = state_machine
                            .rise(State::Intendation(false, IntenData { inner: Vec::new() }));
                    }

                    State::Header(n, p) => {
                        if !p {
                            output.push(b'<');
                            output.push(b'h');
                            output.push(n + 48);
                            output.push(b'>');

                            state_machine.current = State::Header(n, true);
                        } else {
                            output.push(byte);
                        }
                    }

                    State::Code(prev, count) => {
                        if prev {
                            match count {
                                1 => {
                                    output.write(TAG_CODEI_O);
                                    output.push(byte);
                                    state_machine.current = State::Code(false, count);
                                }

                                3 => {
                                    output.write(TAG_CODEB_O);
                                    output.push(byte);
                                    state_machine.current = State::Code(false, count);
                                }

                                _ => {
                                    // No reason to push code block if it is empty
                                    // so we jusp push the character literal to output
                                    state_machine = state_machine.fall();
                                    output.push(byte);
                                }
                            }
                        } else {
                            output.push(byte);
                        }
                    }

                    State::Italic(true) => {
                        output.write(TAG_I_O);
                        output.push(byte);
                        state_machine.current = State::Italic(false);
                    }

                    State::Bold(true) => {
                        output.write(TAG_B_O);
                        output.push(byte);
                        state_machine.current = State::Bold(false);
                    }

                    State::Link(ref mut ld) => {
                        if ld.status.is_link() {
                            // Convert space into url encoded space
                            output.write(b"%20");
                        } else {
                            if ld.status.alt_expects_url() {
                                output.push(b'[');
                                output.write(&ld.alt);
                                output.push(b']');
                                output.push(byte);

                                state_machine = state_machine.fall();
                            } else {
                                ld.alt.push(byte);
                            }
                        }
                    }

                    State::Intendation(_, b) => {
                        state_machine.current = State::Intendation(false, b);
                    }

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::UList(true, written) => {
                        if !written {
                            output.write(TAG_UL_O);
                        }

                        output.write(TAG_LI_O);
                        state_machine.current = State::UList(false, true);
                        state_machine = state_machine.rise(State::LItem);
                    }

                    State::UList(false, _) => continue,

                    _ => output.push(byte),
                },

                b'[' => match state_machine.current {
                    State::Link(ref mut ld) | State::Image(ref mut ld) => {
                        if ld.is_link() {
                            ld.link.push(byte);
                        }
                    }

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    _ => {
                        let ld: Linkdata = Linkdata {
                            status: Linkstatus::Alt(0),
                            alt: Vec::with_capacity(255),
                            link: Vec::with_capacity(255),
                        };

                        match state_machine.current {
                            State::Exclamation => state_machine.current = State::Image(ld),

                            State::Intendation(exp, ref buf) => {
                                if exp {
                                    // Close intend div tag
                                    output.write(TAG_INT_C);
                                    output.write(&buf.inner);
                                    state_machine = state_machine.fall();
                                }

                                state_machine = state_machine.rise(State::Link(ld));
                            }

                            State::UList(_, written) => {
                                if written {
                                    output.write(TAG_UL_C);
                                }
                                output.write(TAG_P_C);
                                state_machine = state_machine
                                    .fall()
                                    .fall()
                                    .rise(State::Link(ld));
                            }

                            _ => state_machine = state_machine.rise(State::Link(ld)),
                        }
                    }
                },

                b'(' => match state_machine.current {
                    State::Link(ref mut ld) | State::Image(ref mut ld) => {
                        if ld.is_alt() {
                            if ld.alt_expects_url() {
                                ld.status = Linkstatus::Link;
                            } else {
                                // Fall back from link/image and write the alt data as is
                                output.push(b'[');
                                output.write(&ld.alt);
                                output.push(byte);
                                state_machine = state_machine.fall();
                            }
                        } else {
                            output.push(b'[');
                            output.write(&ld.alt);
                            output.push(b']');
                            output.push(b'(');
                            output.write(&ld.link);
                            output.push(byte);
                            state_machine = state_machine.fall();
                        }
                    }

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Intendation(_, buf) => {
                        // Close intend div tag
                        output.write(TAG_INT_C);
                        output.write(&buf.inner);
                        // Open p tag
                        output.write(TAG_P_O);
                        output.push(byte);
                        state_machine.current = State::Paragraph;
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        state_machine = state_machine.fall();

                        match state_machine.current {
                            State::Link(ref mut ld) | State::Image(ref mut ld) => {
                                if ld.is_alt() {
                                    if ld.alt_expects_url() {
                                        ld.status = Linkstatus::Link;
                                    } else {
                                        // Fall back from link/image and write the alt data as is
                                        output.push(b'[');
                                        output.write(&ld.alt);
                                        output.push(byte);
                                        state_machine = state_machine.fall();
                                    }
                                } else {
                                    output.push(b'[');
                                    output.write(&ld.alt);
                                    output.push(b']');
                                    output.push(b'(');
                                    output.write(&ld.link);
                                    output.push(byte);
                                    state_machine = state_machine.fall();
                                }
                            }

                            _ => output.push(byte),
                        }
                    }

                    State::UList(_, written) => {
                        if written {
                            // Start a new paragraph and end the list
                            output.write(TAG_UL_C);
                        }

                        output.write(TAG_P_C);
                        output.write(TAG_P_O);
                        output.push(byte);
                        state_machine = state_machine
                            .fall();
                    }

                    _ => {
                        output.push(byte);
                    }
                },

                b']' => match state_machine.current {
                    State::Link(ref mut ld) | State::Image(ref mut ld) => {
                        if ld.status.is_alt() {
                            if ld.alt_expects_closure() {
                                ld.status = Linkstatus::Alt(1);
                            } else {
                                // Fall back from link and write the alt data as is
                                output.write(&ld.alt);
                                output.push(byte);
                                state_machine = state_machine.fall();
                            }
                        } else {
                            ld.link.push(byte);
                        }
                    }

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        state_machine = state_machine.fall();

                        match state_machine.current {
                            State::Link(ref mut ld) | State::Image(ref mut ld) => {
                                if ld.status.is_alt() {
                                    if ld.alt_expects_closure() {
                                        ld.status = Linkstatus::Alt(1);
                                    } else {
                                        // Fall back from link and write the alt data as is
                                        output.write(&ld.alt);
                                        output.push(byte);
                                        state_machine = state_machine.fall();
                                    }
                                } else {
                                    ld.link.push(byte);
                                }
                            }

                            _ => output.push(byte),
                        }
                    }

                    State::Intendation(_, buf) => {
                        // Close intendation div tag
                        output.write(TAG_INT_C);
                        output.write(&buf.inner);
                        // Open p tag
                        output.write(TAG_P_O);
                        state_machine.current = State::Paragraph;
                        output.push(byte);
                    }

                    _ => {
                        output.push(byte);
                    }
                },

                b')' => match state_machine.current {
                    State::Link(ref ld) => {
                        if ld.is_link() {
                            // Output an link
                            output.write(b"<a href=\"");
                            output.write(&ld.link);
                            output.write(b"\">");
                            output.write(&ld.alt);
                            output.write(b"</a>");
                            state_machine = state_machine.fall();
                        } else {
                            output.push(byte);
                        }
                    }

                    State::Image(ref ld) => {
                        if ld.is_link() {
                            // Output an image
                            output.write(b"<img src=\"");
                            output.write(&ld.link);
                            output.write(b"\" alt=\"");
                            output.write(&ld.alt);
                            output.write(b"\">");
                            state_machine = state_machine.fall();
                        } else {
                            output.push(byte);
                        }
                    }

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Intendation(_, buf) => {
                        // Close intend div tag
                        output.write(TAG_INT_C);
                        output.write(&buf.inner);
                        // Open p tag
                        output.write(TAG_P_O);
                        output.push(byte);
                        state_machine.current = State::Paragraph;
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        state_machine = state_machine.fall();

                        match state_machine.current {
                            State::Link(ref ld) => {
                                if ld.is_link() {
                                    // Output an link
                                    output.write(b"<a href=\"");
                                    output.write(&ld.link);
                                    output.write(b"\">");
                                    output.write(&ld.alt);
                                    output.write(b"</a>");
                                    state_machine = state_machine.fall();
                                } else {
                                    output.push(byte);
                                }
                            }

                            State::Image(ref ld) => {
                                if ld.is_link() {
                                    // Output an image
                                    output.write(b"<img src=\"");
                                    output.write(&ld.link);
                                    output.write(b"\" alt=\"");
                                    output.write(&ld.alt);
                                    output.write(b"\">");
                                    state_machine = state_machine.fall();
                                } else {
                                    output.push(byte);
                                }
                            }

                            _ => output.push(byte),
                        }
                    }

                    _ => output.push(byte),
                },

                b'\r' | b'\n' => {
                    column_counter = 0;
                    if byte == b'\n' {
                        line_counter += 1;
                    }

                    match state_machine.current {
                        State::None => output.push(byte),

                        State::Header(n, p) => {
                            if !p {
                                println!("Empty header? Really??");
                            }

                            output.write(b"</h");
                            output.push(n + 48);
                            output.push(b'>');
                            output.push(byte);

                            state_machine = state_machine.fall();
                        }

                        State::Paragraph => {
                            output.push(b'<');
                            output.push(b'/');
                            output.push(b'p');
                            output.push(b'>');

                            state_machine = state_machine.fall();

                            match state_machine.current {
                                State::Intendation(_, mut buf) => {
                                    buf.inner.push(byte);
                                    state_machine.current = State::Intendation(true, buf);
                                }

                                _ => output.push(byte),
                            }
                        }

                        State::Code(seen, count) => {
                            if count == 1 {
                                eprintln!("Unexpected new line in the middle of inline code.");
                                // Close code block span tag and code tag
                                output.write(TAG_CODEI_C);

                                state_machine = state_machine.fall();

                                while !state_machine.is_none() {
                                    if state_machine.is_paragraph() {
                                        output.write(TAG_P_C);
                                    }

                                    state_machine = state_machine.fall();
                                }
                            } else if count == 2 {
                                if seen {
                                    eprintln!("Unexpected number of code block keys. Maybe you meant to write 3?");
                                }

                                state_machine = state_machine.fall();
                            }

                            output.push(byte);
                        }

                        State::Escape => {
                            output.push(byte);
                            state_machine = state_machine.fall();
                        }

                        State::Link(ref ld) | State::Image(ref ld) => {
                            println!("Warning: New lines in links and images are not supported. This may cripple your text.");
                            if ld.is_alt() {
                                output.push(b'[');
                                output.write(&ld.alt);
                                output.push(byte);
                                state_machine = state_machine.fall();
                            } else {
                                output.push(b'[');
                                output.write(&ld.alt);
                                output.push(b']');
                                output.push(b'(');
                                output.write(&ld.link);
                                output.push(byte);
                                state_machine = state_machine.fall();
                            }
                        }

                        State::Intendation(_, mut buf) => {
                            buf.inner.push(byte);
                            state_machine.current = State::Intendation(true, buf);
                        }

                        State::Exclamation => {
                            output.push(b'!');
                            state_machine = state_machine.fall();

                            loop {
                                match state_machine.current {
                                    State::Paragraph => output.write(TAG_P_C),
                                    State::Header(n, _) => {
                                        output.write(b"</h");
                                        output.push(n + 48);
                                        output.push(b'>');
                                    }
                                    State::Intendation(_, mut buf) => {
                                        buf.inner.push(byte);
                                        state_machine.current = State::Intendation(true, buf);
                                        break;
                                    }
                                    _ => {
                                        output.push(byte);
                                        break;
                                    }
                                }

                                state_machine = state_machine.fall();
                            }
                        }

                        State::LItem => {
                            output.write(TAG_LI_C);
                            output.push(byte);
                            state_machine = state_machine.fall();
                        }

                        State::UList(true, _) => {
                            output.push(byte);
                            output.write(TAG_P_C);
                            state_machine = state_machine.fall().fall();
                        }

                        State::Hor(3..) => {
                            output.write(TAG_HR);
                            output.push(byte);
                            state_machine = state_machine.fall().fall();
                        }

                        _ => output.push(byte),
                    }
                }

                b'`' => match state_machine.current {
                    State::None => {
                        output.write(TAG_P_O);
                        println!("Code key increment to 1");
                        state_machine = state_machine
                            .rise(State::Paragraph)
                            .rise(State::Code(true, 1));
                    }

                    State::Code(ls, n) => {
                        let x = n + 1;
                        if ls {
                            state_machine.current = State::Code(ls, x);
                             if x == 6 {
                                // Close code blog div tag and code tag
                                output.write(TAG_CODEB_C);
                                state_machine = state_machine.fall();
                            }
                        } else {
                            if x == 2 {
                                // Close code blog span tag and code tag
                                output.write(TAG_CODEI_C);
                                state_machine = state_machine.fall();

                            } else {
                                state_machine.current = State::Code(true, x);
                            }
                        }
                    }

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Intendation(exp, ref buf) => {
                        if !exp {
                            // Open p tag
                            output.write(TAG_P_O);
                            state_machine = state_machine
                                .rise(State::Paragraph)
                                .rise(State::Code(true, 1));
                        } else {
                            // Close intend div tag
                            output.write(TAG_INT_C);
                            output.write(&buf.inner);
                            // Open p tag
                            output.write(TAG_P_O);
                            state_machine.current = State::Code(true, 1);
                        }
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        state_machine.current = State::Code(true, 1);
                    }

                    State::Italic(true) => {
                        output.write(TAG_I_O);
                        state_machine.current = State::Italic(false);
                        state_machine = state_machine.rise(State::Code(true, 1));
                    }

                    State::Bold(seen) => {
                        if seen {
                            println!("Warning: Non-escaped `*` in the middle of bolded text. Parsing it as a literal..");
                            output.push(b'*');
                            state_machine.current = State::Bold(false);
                        }
                        state_machine = state_machine.rise(State::Code(true, 1));
                    }

                    _ => {
                        state_machine = state_machine.rise(State::Code(true, 1));
                    }
                },

                b'*' => match state_machine.current {
                    State::None => {
                        // Open p tag
                        output.write(TAG_P_O);
                        state_machine = state_machine
                            .rise(State::Paragraph)
                            .rise(State::Italic(true));
                    }

                    State::Paragraph => state_machine = state_machine.rise(State::Italic(true)),

                    State::Intendation(exp, ref buf) => {
                        if exp {
                            // Close intend div tag
                            output.write(TAG_INT_C);
                            output.write(&buf.inner);
                            // Open p tag
                            output.write(TAG_P_O);
                            state_machine = state_machine
                                .fall()
                                .rise(State::Paragraph)
                                .rise(State::Italic(true));
                        } else {
                            // Open p tag
                            output.write(TAG_P_O);
                            state_machine = state_machine
                                .rise(State::Paragraph)
                                .rise(State::Italic(true));
                        }
                    }

                    State::Escape => {
                        state_machine = state_machine.fall();

                        match state_machine.current {
                            State::None => {
                                // Open p tag
                                output.write(TAG_P_O);
                                state_machine = state_machine.rise(State::Paragraph);
                            }

                            State::Intendation(exp, ref buf) => {
                                if exp {
                                    // Close intend div tag
                                    output.write(TAG_INT_C);
                                    output.write(&buf.inner);
                                    // Open p tag
                                    output.write(TAG_P_O);
                                    state_machine = state_machine.fall().rise(State::Paragraph);
                                } else {
                                    // Open p tag
                                    output.write(TAG_P_O);
                                    state_machine = state_machine.rise(State::Paragraph);
                                }
                            }

                            _ => {}
                        }

                        output.push(byte);
                    }

                    State::Code(ls, n) => {
                        if ls {
                            match n {
                                1 => {
                                    output.write(TAG_CODEI_O);
                                    output.push(byte);
                                    state_machine.current = State::Code(false, n);
                                }

                                3 => {
                                    output.write(TAG_CODEB_O);
                                    output.push(byte);
                                    state_machine.current = State::Code(false, n);
                                }

                                _ => {
                                    println!("Warning: Unexpected code block state! Undefined behaviour may occur! Trying to mitigate damage by ignoring previous key..");
                                    output.push(byte);
                                    state_machine = state_machine.fall();
                                }
                            }
                        } else {
                            output.push(byte);
                        }
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        state_machine.current = State::Italic(true);
                    }

                    State::Header(_, _) => state_machine = state_machine.rise(State::Italic(true)),

                    State::Italic(seen) => {
                        if seen {
                            // Open b tag
                            output.write(TAG_B_O);
                            // Switch state from Italic to Bold because there were two `*` characters
                            // in a row. Swtiching instead of rising to not preserve the Italic state.
                            state_machine.current = State::Bold(false);
                        } else {
                            // Close i tag
                            output.write(TAG_I_C);
                            state_machine = state_machine.fall();
                        }
                    }

                    State::Bold(seen) => {
                        if seen {
                            // Close b tag
                            output.write(TAG_B_C);
                            state_machine = state_machine.fall();
                        } else {
                            state_machine.current = State::Bold(true);
                        }
                    }

                    State::Underscore => {
                        state_machine = state_machine.rise(State::Italic(true));
                    }

                    _ => output.push(byte),
                },

                b'_' => match state_machine.current {
                    State::None => {
                        output.write(TAG_P_O);
                        state_machine =
                            state_machine.rise(State::Paragraph).rise(State::Underscore);
                    }

                    State::Paragraph | State::Header(_, _) => {
                        state_machine = state_machine.rise(State::Underscore)
                    }

                    State::Intendation(exp, ref buf) => {
                        if exp {
                            output.write(TAG_INT_C);
                            output.write(&buf.inner);
                            output.write(TAG_P_O);
                            output.write(TAG_U_O);
                            state_machine = state_machine
                                .fall()
                                .rise(State::Paragraph)
                                .rise(State::Underscore);
                        } else {
                            output.write(TAG_U_O);
                            state_machine = state_machine.rise(State::Underscore);
                        }
                    }

                    State::Bold(seen) => {
                        if seen {
                            println!("Warning: Non-escaped `*` in the middle of bolded text. Parsing it as a literal..");
                            output.push(b'*');
                            state_machine.current = State::Bold(false);
                        }
                        output.write(TAG_U_O);
                        state_machine = state_machine.rise(State::Underscore);
                    }

                    State::Italic(seen) => {
                        if seen {
                            output.write(TAG_I_O);
                            state_machine = state_machine.rise(State::Italic(false));
                        }
                        output.write(TAG_U_O);
                        state_machine = state_machine.rise(State::Underscore);
                    }

                    State::Underscore => {
                        output.write(TAG_U_C);
                        state_machine = state_machine.fall();
                    }

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        state_machine = state_machine.fall().rise(State::Underscore);
                    }

                    State::Link(ref mut ld) | State::Image(ref mut ld) => {
                        if ld.is_alt() {
                            ld.alt.push(byte);
                        } else {
                            ld.link.push(byte);
                        }
                    }

                    _ => output.push(byte),
                },

                b'-' => match state_machine.current {
                    State::None => {
                        output.write(TAG_P_O);
                        state_machine = state_machine
                            .rise(State::Paragraph)
                            .rise(State::UList(true, false));
                    }

                    State::Intendation(exp, ref mut buf) => {
                        if exp {
                            output.write(TAG_INT_C);
                            output.write(&buf.inner);
                            state_machine = state_machine.fall();
                        } else {
                            output.write(&buf.inner);
                            buf.inner.clear();
                        }

                        output.write(TAG_P_O);
                        state_machine = state_machine
                            .rise(State::Paragraph)
                            .rise(State::UList(true, false));
                    }

                    State::UList(true, false) => state_machine.current = State::Hor(2),

                    State::UList(true, true) => {
                        output.write(TAG_UL_C);
                        state_machine = state_machine
                            .fall()
                            .rise(State::Hor(2));
                    }

                    State::UList(false, p) => state_machine.current = State::UList(true, p),

                    State::Hor(n) => state_machine.current = State::Hor(n+1),

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Exclamation => {
                        output.push(b'!');
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Link(ref mut ld) | State::Image(ref mut ld) => {
                        if ld.is_alt() {
                            ld.alt.push(byte);
                        } else {
                            ld.link.push(byte);
                        }
                    }

                    _ => output.push(byte),
                }

                _ => output.push(byte),
            }

            column_counter += 1;
        }

        if state_machine.is_ulist() {
            // Close ul tag
            output.write(TAG_UL_C);
            state_machine = state_machine.fall();
        }

        if state_machine.is_paragraph() {
            // Close p tag
            output.write(TAG_P_C);
            state_machine = state_machine.fall();
        }

        if state_machine.is_intend() {
            // Close intend div tag
            output.write(TAG_INT_C);
        }

        output
    }

    /// Switches the state to previous state discarding the current state
    /// and consuming the current self value.
    fn fall(self) -> Self {
        #[cfg(debug_assertions)]
        println!("Falling from state {:?}", &self.current);

        if self.previous.is_some() {
            *self.previous.unwrap()
        } else {
            println!("Warning: Already in root state! Cannot fall back.");
            self
        }
    }

    fn rise(self, top: State) -> Self {
        #[cfg(debug_assertions)]
        println!("Rising from state {:?} to state {:?}", &self.current, &top);

        Self {
            current: top,
            previous: Some(Box::new(self)),
        }
    }

    fn is_none(&self) -> bool {
        match self.current {
            State::None => true,
            _ => false,
        }
    }

    fn is_paragraph(&self) -> bool {
        match self.current {
            State::Paragraph => true,
            _ => false,
        }
    }

    fn is_ulist(&self) -> bool {
        match self.current {
            State::UList(_, true) => true,
            _ => false,
        }
    }

    fn is_intend(&self) -> bool {
        match self.current {
            State::Intendation(_, _) => true,
            _ => false,
        }
    }
}
