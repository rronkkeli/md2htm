//! This module converts markdown to html without the root elements.

use std::boxed::Box;

/// Buffer size for output data
const BUF_SIZE: usize = 16384;

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

        let mut output: Vec<u8> = Vec::with_capacity(BUF_SIZE);

        for byte in bytes {
            match byte {
                0..10 | 11..13 | 14..32 | 34..35 | 36..40 | 43..91 | 97..=255 => {
                    match state_machine.current {
                        State::None => {
                            state_machine = state_machine.rise(State::Paragraph);
                            output.push(b'<');
                            output.push(b'p');
                            output.push(b'>');
                            output.push(byte);
                        }

                        State::Code(ls, n) => {
                            if ls {
                                match n {
                                    1 => {
                                        state_machine.current = State::Code(false, n);
                                        let tag_start_code: &[u8] =
                                            b"<span class=\"code\"><code class=\"code\">";
                                        for b in tag_start_code {
                                            output.push(*b);
                                        }
                                    }

                                    3 => {
                                        state_machine.current = State::Code(false, n);
                                        let tag_start_code: &[u8] =
                                            b"<div class=\"code\"><code class=\"code\">";
                                        for b in tag_start_code {
                                            output.push(*b);
                                        }
                                    }

                                    _ => {
                                        println!("Warning: Unexpected code block state! Undefined behaviour may occur! Trying to mitigate damage by ignoring previous key..");

                                        state_machine = state_machine.fall();
                                        output.push(byte);
                                    }
                                }
                            }
                            output.push(byte);
                        }

                        State::Escape | State::Exclamation => {
                            match byte {
                                b'<' => {
                                    for b in b"&lt;" {
                                        output.push(*b);
                                    }
                                }

                                b'>' => {
                                    for b in b"&gt;" {
                                        output.push(*b);
                                    }
                                }

                                _ => output.push(byte),
                            }

                            state_machine = state_machine.fall();
                        }

                        State::Link(ref mut ld) | State::Image(ref mut ld) => match ld.status {
                            Linkstatus::Alt(0) => {
                                ld.alt.push(byte);
                            }

                            Linkstatus::Alt(1) => {
                                output.push(b'[');
                                for b in &ld.alt {
                                    output.push(*b);
                                }
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
                                for b in b"</div>" {
                                    output.push(*b);
                                }

                                for b in &buf.inner {
                                    output.push(*b);
                                }

                                state_machine = state_machine.fall();
                            } else {
                                for b in &buf.inner {
                                    output.push(*b);
                                }

                                buf.inner.clear();
                            }

                            for b in b"<p>" {
                                output.push(*b);
                            }

                            output.push(byte);
                            state_machine = state_machine.rise(State::Paragraph);
                        }

                        State::Italic(seen) => {
                            if seen {
                                for b in b"<i>" {
                                    output.push(*b);
                                }

                                state_machine.current = State::Italic(false);
                            }

                            output.push(byte);
                        }

                        State::Bold(seen) => {
                            if seen {
                                println!("Warning: Non-escaped `*` in the middle of bolded text. Parsing it as a literal..");

                                output.push(b'*');
                                state_machine.current = State::Bold(false);
                            }

                            output.push(byte);
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
                            for b in b"</div>" {
                                output.push(*b);
                            }

                            for b in &buf.inner {
                                output.push(*b);
                            }

                            state_machine = state_machine.fall();
                        }

                        state_machine = state_machine.rise(State::Exclamation);
                    }

                    _ => {
                        state_machine = state_machine.rise(State::Exclamation);
                    }
                },

                b'\\' => match state_machine.current {
                    State::Escape | State::Exclamation => {
                        output.push(byte);

                        state_machine = state_machine.fall();
                    }

                    _ => state_machine = state_machine.rise(State::Escape),
                },

                b'#' => match state_machine.current {
                    State::None => state_machine = state_machine.rise(State::Header(1, false)),

                    State::Intendation(exp, ref buf) => {
                        if exp {
                            for b in b"</div>" {
                                output.push(*b);
                            }

                            for b in &buf.inner {
                                output.push(*b);
                            }

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

                    State::Escape | State::Exclamation => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Code(ls, n) => {
                        if ls {
                            match n {
                                1 => {
                                    state_machine.current = State::Code(false, n);
                                    let tag_start_code: &[u8] =
                                        b"<span class=\"code\"><code class=\"code\">";
                                    for b in tag_start_code {
                                        output.push(*b);
                                    }
                                }

                                3 => {
                                    state_machine.current = State::Code(false, n);
                                    let tag_start_code: &[u8] =
                                        b"<div class=\"code\"><code class=\"code\">";
                                    for b in tag_start_code {
                                        output.push(*b);
                                    }
                                }

                                _ => {
                                    println!("Warning: Unexpected code block state! Undefined behaviour may occur! Trying to mitigate damage by ignoring previous key..");

                                    state_machine = state_machine.fall();
                                    output.push(byte);
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
                            for b in &ld.alt {
                                output.push(*b);
                            }
                            output.push(b']');

                            output.push(b'(');
                            for b in &ld.link {
                                output.push(*b);
                            }

                            output.push(byte);
                            state_machine = state_machine.fall();
                        }
                    },

                    _ => {
                        output.push(byte);
                    }
                },

                b' ' => {
                    match state_machine.current {
                        State::None => {
                            let tag_start_intend: &[u8] = b"<div class=\"intend\">";
                            for b in tag_start_intend {
                                output.push(*b);
                            }

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
                                        state_machine.current = State::Code(false, count);
                                        let tag_start_code: &[u8] =
                                            b"<span class=\"code\"><code class=\"code\"> ";
                                        for b in tag_start_code {
                                            output.push(*b);
                                        }
                                    }

                                    3 => {
                                        state_machine.current = State::Code(false, count);
                                        let tag_start_code: &[u8] =
                                            b"<div class=\"code\"><code class=\"code\"> ";
                                        for b in tag_start_code {
                                            output.push(*b);
                                        }
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
                            state_machine.current = State::Italic(false);
                            let tag_start_i: &[u8] = b"<i> ";
                            for b in tag_start_i {
                                output.push(*b);
                            }
                        }

                        State::Bold(true) => {
                            state_machine.current = State::Bold(false);
                            let tag_start_b: &[u8] = b"<b> ";
                            for b in tag_start_b {
                                output.push(*b);
                            }
                        }

                        State::Link(ref mut ld) => {
                            if ld.status.is_link() {
                                // Convert space into url encoded space
                                let url_enc_space: &[u8] = b"%20";
                                for b in url_enc_space {
                                    ld.link.push(*b);
                                }
                            } else {
                                if ld.status.alt_expects_url() {
                                    output.push(b'[');
                                    for b in &ld.alt {
                                        output.push(*b);
                                    }
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

                        State::Escape | State::Exclamation => {
                            output.push(byte);
                            state_machine = state_machine.fall();
                        }

                        _ => output.push(byte),
                    }
                }

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
                                    for b in b"</div>" {
                                        output.push(*b);
                                    }

                                    for b in &buf.inner {
                                        output.push(*b);
                                    }

                                    state_machine = state_machine.fall();
                                }

                                state_machine = state_machine.rise(State::Link(ld));
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
                                for b in &ld.alt {
                                    output.push(*b);
                                }

                                output.push(byte);
                                state_machine = state_machine.fall();
                            }
                        } else {
                            output.push(b'[');
                            for b in &ld.alt {
                                output.push(*b);
                            }
                            output.push(b']');

                            output.push(b'(');
                            for b in &ld.link {
                                output.push(*b);
                            }

                            output.push(byte);
                            state_machine = state_machine.fall();
                        }
                    }

                    State::Escape | State::Exclamation => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Intendation(_, buf) => {
                        for b in b"</div>" {
                            output.push(*b);
                        }

                        for b in &buf.inner {
                            output.push(*b);
                        }

                        for b in b"<p>" {
                            output.push(*b);
                        }

                        state_machine.current = State::Paragraph;

                        output.push(byte);
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
                                for b in &ld.alt {
                                    output.push(*b);
                                }

                                output.push(byte);
                                state_machine = state_machine.fall();
                            }
                        } else {
                            ld.link.push(byte);
                        }
                    }

                    State::Escape | State::Exclamation => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Intendation(_, buf) => {
                        for b in b"</div>" {
                            output.push(*b);
                        }

                        for b in &buf.inner {
                            output.push(*b);
                        }

                        for b in b"<p>" {
                            output.push(*b);
                        }

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
                            let tag_link_start: &[u8] = b"<a href=\"";
                            let tag_link_middle: &[u8] = b"\">";
                            let tag_link_end: &[u8] = b"</a>";

                            for b in tag_link_start {
                                output.push(*b);
                            }

                            for b in &ld.link {
                                output.push(*b);
                            }

                            for b in tag_link_middle {
                                output.push(*b);
                            }

                            for b in &ld.alt {
                                output.push(*b);
                            }

                            for b in tag_link_end {
                                output.push(*b);
                            }

                            state_machine = state_machine.fall();
                        } else {
                            output.push(byte);
                        }
                    }

                    State::Image(ref ld) => {
                        if ld.is_link() {
                            // Output an image
                            let tag_image_start: &[u8] = b"<img src=\"";
                            let tag_image_middle: &[u8] = b"\" alt=\"";
                            let tag_image_end: &[u8] = b"\">";

                            for b in tag_image_start {
                                output.push(*b);
                            }

                            for b in &ld.link {
                                output.push(*b);
                            }

                            for b in tag_image_middle {
                                output.push(*b);
                            }

                            for b in &ld.alt {
                                output.push(*b);
                            }

                            for b in tag_image_end {
                                output.push(*b);
                            }

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
                        for b in b"</div>" {
                            output.push(*b);
                        }

                        for b in &buf.inner {
                            output.push(*b);
                        }

                        for b in b"<p>" {
                            output.push(*b);
                        }

                        state_machine.current = State::Paragraph;

                        output.push(byte);
                    }

                    _ => output.push(byte),
                },

                b'\r' | b'\n' => match state_machine.current {
                    State::None => output.push(byte),

                    State::Header(n, p) => {
                        if !p {
                            println!("Empty header? Really??");
                        }

                        output.push(b'<');
                        output.push(b'/');
                        output.push(b'h');
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

                    State::Code(_, 0..3) => {
                        println!("Warning: You can't just put a new line in inline code! Are you daft? Look what happened!");

                        let tag_code_end: &[u8] = b"</code></span>";
                        for b in tag_code_end {
                            output.push(*b);
                        }

                        state_machine = state_machine.fall();

                        while !state_machine.is_none() {
                            if state_machine.is_paragraph() {
                                output.push(b'<');
                                output.push(b'/');
                                output.push(b'p');
                                output.push(b'>');
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
                            for b in &ld.alt {
                                output.push(*b);
                            }

                            output.push(byte);
                            state_machine = state_machine.fall();
                        } else {
                            output.push(b'[');
                            for b in &ld.alt {
                                output.push(*b);
                            }
                            output.push(b']');

                            output.push(b'(');
                            for b in &ld.link {
                                output.push(*b);
                            }

                            output.push(byte);
                            state_machine = state_machine.fall();
                        }
                    }

                    State::Intendation(_, mut buf) => {
                        buf.inner.push(byte);

                        state_machine.current = State::Intendation(true, buf);
                    }

                    _ => output.push(byte),
                },

                b'`' => match state_machine.current {
                    State::None => {
                        output.push(b'<');
                        output.push(b'/');
                        output.push(b'p');
                        output.push(b'>');

                        state_machine = state_machine
                            .rise(State::Paragraph)
                            .rise(State::Code(true, 1));
                    }

                    State::Code(ls, n) => {
                        if ls {
                            state_machine.current = State::Code(ls, n + 1);
                        } else {
                            match n {
                                // Close inline code
                                1 => {
                                    let tag_code_end: &[u8] = b"</code></span>";
                                    for b in tag_code_end {
                                        output.push(*b);
                                    }

                                    state_machine = state_machine.fall();
                                },

                                3 => {
                                    let tag_code_end: &[u8] = b"</code></div>";
                                    for b in tag_code_end {
                                        output.push(*b);
                                    }

                                    state_machine = state_machine.fall();
                                },

                                _ => println!("Warning: Predicting undefined behaviour because of unexpected code block length!"),
                            }
                        }
                    }

                    State::Escape => {
                        output.push(byte);
                        state_machine = state_machine.fall();
                    }

                    State::Intendation(exp, ref buf) => {
                        if !exp {
                            for b in b"<p>" {
                                output.push(*b);
                            }

                            state_machine = state_machine
                                .rise(State::Paragraph)
                                .rise(State::Code(true, 1));
                        } else {
                            for b in b"</div>" {
                                output.push(*b);
                            }

                            for b in &buf.inner {
                                output.push(*b);
                            }

                            for b in b"<p>" {
                                output.push(*b);
                            }

                            state_machine.current = State::Code(true, 1);
                        }
                    }

                    _ => {
                        state_machine = state_machine.rise(State::Code(true, 1));
                    }
                },

                b'*' => match state_machine.current {
                    State::None => {
                        for b in b"<p>" {
                            output.push(*b);
                        }

                        state_machine = state_machine
                            .rise(State::Paragraph)
                            .rise(State::Italic(true));
                    },

                    State::Paragraph => state_machine = state_machine.rise(State::Italic(true)),

                    State::Intendation(exp, ref buf) => {
                        if exp {
                            for b in b"</div>" {
                                output.push(*b);
                            }

                            for b in &buf.inner {
                                output.push(*b);
                            }

                            for b in b"<p>" {
                                output.push(*b);
                            }

                            state_machine.current = State::Paragraph;
                            state_machine = state_machine.rise(State::Italic(true));

                        } else {
                            for b in b"<p>" {
                                output.push(*b);
                            }

                            state_machine = state_machine
                                .rise(State::Paragraph)
                                .rise(State::Italic(true));
                        }
                    }

                    State::Escape => {
                        state_machine = state_machine.fall();

                        match state_machine.current {
                            State::None => {
                                for b in b"<p>" {
                                output.push(*b);
                                }

                                state_machine = state_machine
                                    .rise(State::Paragraph);
                            }

                            State::Intendation(exp, ref buf) => {
                                if exp {
                                    for b in b"</div>" {
                                        output.push(*b);
                                    }

                                    for b in &buf.inner {
                                        output.push(*b);
                                    }

                                    for b in b"<p>" {
                                        output.push(*b);
                                    }

                                    state_machine = state_machine
                                        .fall()
                                        .rise(State::Paragraph);

                                } else {
                                    for b in b"<p>" {
                                        output.push(*b);
                                    }

                                    state_machine = state_machine
                                        .rise(State::Paragraph);
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
                                    state_machine.current = State::Code(false, n);
                                    let tag_start_code: &[u8] =
                                        b"<span class=\"code\"><code class=\"code\">*";
                                    for b in tag_start_code {
                                        output.push(*b);
                                    }
                                }

                                3 => {
                                    state_machine.current = State::Code(false, n);
                                    let tag_start_code: &[u8] =
                                        b"<div class=\"code\"><code class=\"code\">*";
                                    for b in tag_start_code {
                                        output.push(*b);
                                    }
                                }

                                _ => {
                                    println!("Warning: Unexpected code block state! Undefined behaviour may occur! Trying to mitigate damage by ignoring previous key..");

                                    state_machine = state_machine.fall();
                                    output.push(byte);
                                }
                            }
                        } else {
                            output.push(byte);
                        }
                    }

                    State::Exclamation => state_machine.current = State::Italic(true),

                    State::Header(_, _) => state_machine = state_machine.rise(State::Italic(true)),

                    State::Italic(seen) => {
                        if seen {
                            for b in b"<b>" {
                                output.push(*b);
                            }

                            // Switch state from Italic to Bold because there were two `*` characters
                            // in a row. Swtiching instead of rising to not preserve the Italic state.
                            state_machine.current = State::Bold(false);

                        } else {
                            for b in b"</i>" {
                                output.push(*b);
                            }

                            state_machine = state_machine.fall();
                        }
                    }

                    State::Bold(seen) => {
                        if seen {
                            for b in b"</b>" {
                                output.push(*b);
                            }

                            state_machine = state_machine.fall();

                        } else {
                            state_machine.current = State::Bold(true);
                        }
                    }

                    _ => output.push(byte),
                },

                _ => {
                    output.push(byte);
                }
            }
        }

        if state_machine.is_paragraph() {
            for b in b"</p>" {
                output.push(*b);
            }
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
}
