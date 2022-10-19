// Zestien, a hex editor by Simeon Duwel.

const BYTE_WIDTH: usize = 16;

use std::{env, fs::File, io::{BufReader, Read}};

extern crate cursive;
use cursive::views::{TextView, NamedView, PaddedView, Panel};
use cursive::{Cursive, CursiveExt};

fn nybble_to_hex(n: u8) -> char {
    if n < 10 {
        return (n + 0x30) as char;
    } else if n < 16 {
        return (n - 9 + 0x60) as char;
    }

    unreachable!("Nybbles are always 0x0F or less, received {}", n);
}


struct CharRep {
    lower: char,
    upper: char,
    ascii: char
}

impl From<u8> for CharRep {
    fn from(source: u8) -> Self {
        Self {
            lower: nybble_to_hex(source & 0x0f),
            upper: nybble_to_hex(source >> 4),
            ascii: if (source > 0x40 && source < 0x5b) || (source > 0x60 && source < 0x7b) { source as char } else { '.' } //TODO: add nerd font support for newline char
        }
    }
}

impl std::fmt::Display for CharRep {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}{}", self.upper, self.lower)
    }
}

fn main() {
    let maybe_path = &env::args().collect::<Vec<String>>()[1];
    let file = File::open(maybe_path).expect("Could not open or find the supplied file.");

    let mut buf = String::with_capacity(1 << 16);
    let _reader = BufReader::new(file).read_to_string(&mut buf);

    let data: Vec<_> = buf.as_bytes()
                          .into_iter()
                          .map(|c| CharRep::from(*c))
                          .collect();

    let mut siv = Cursive::new();
    siv.add_layer(
        Panel::new(PaddedView::lrtb(4, 4, 2, 2, NamedView::new("zestien", TextView::new(
            // data.into_iter()
            //     .enumerate()
            //     .map(|(idx, c)| format!("{}{}", c, if (idx + 1) % BYTE_WIDTH == 0 { '\n' } else { ' ' }))
            //     .collect::<Vec<String>>()
            //     .join("")

            data.chunks(BYTE_WIDTH)
                .enumerate()
                .map(|(row_idx, row)| format!(
                    "{}: {} | {}",
                    format!("{:08x}", row_idx * BYTE_WIDTH),
                    row.iter().map(|c| format!("{}", c)).collect::<Vec<_>>().join(" "),
                    row.iter().map(|c| String::from(c.ascii)).collect::<Vec<_>>().join(" ")))
                .collect::<Vec<String>>()
                .join("\n")
        ))))
    );
    siv.add_global_callback('q', |s| s.quit());
    siv.run();
}
