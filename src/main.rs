// Zestien, a hex editor by Simeon Duwel.

const BYTE_WIDTH: usize = 16;

use std::{env, fs::File, io::{BufReader, Read}};

extern crate cursive;
use cursive::{views::{TextView, NamedView, PaddedView, Panel, TextContent}, utils::span::SpannedString, theme::{Style, Effect, ColorStyle, Color, BaseColor}, reexports::enumset::enum_set, event::{Event, Key}};
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

#[derive(Clone, Copy)]
struct Cursor {
    row: usize,
    col: usize,
    on_lower: bool
}

impl Cursor {
    fn new(x: usize, y: usize) -> Self {
        Cursor {
            row: y,
            col: x,
            on_lower: false
        }
    }
}

fn generate_text(data: &Vec<CharRep>, cursor: &Cursor) -> SpannedString<Style> {
    let mut spanned_string = SpannedString::new();
    for (row_idx, row) in data.chunks(BYTE_WIDTH).enumerate() {
        if row_idx != cursor.row {
            spanned_string.append_plain(format!(
                    "{}: {} | {}\n",
                    format!("{:08x}", row_idx * BYTE_WIDTH),
                    row.iter().map(|c| format!("{}", c)).collect::<Vec<_>>().join(" "),
                    row.iter().map(|c| String::from(c.ascii)).collect::<Vec<_>>().join("")));
        } else {
            spanned_string.append_plain(format!("{:08x}: ", row_idx * BYTE_WIDTH));
            for c in &row[..cursor.col] {
                spanned_string.append_plain(format!("{} ", c));
            }

            spanned_string.append_styled(
                format!("{}",  row[cursor.col].upper),
                Style {
                    effects: enum_set!(Effect::Bold),
                    color: ColorStyle::new(
                        if !cursor.on_lower { Color::Light(BaseColor::Cyan) } else { Color::Dark(BaseColor::White) },
                        Color::Dark(BaseColor::Blue)
                    )
                }
            );
            spanned_string.append_styled(
                format!("{}", row[cursor.col].lower),
                Style {
                    effects: enum_set!(Effect::Bold),
                    color: ColorStyle::new(
                        if cursor.on_lower { Color::Light(BaseColor::Cyan) } else { Color::Dark(BaseColor::White) },
                        Color::Dark(BaseColor::Blue)
                    )
                }
            );

            spanned_string.append_plain(" ");

            for c in &row[(cursor.col + 1)..] {
                spanned_string.append_plain(format!("{} ", c));
            }

            spanned_string.append_plain("| ");

            for c in &row[..cursor.col] {
                spanned_string.append_plain(format!("{}", c.ascii))
            }

            spanned_string.append_styled(
                format!("{}", row[cursor.col].ascii),
                Style {
                    effects: enum_set!(Effect::Bold),
                    color: ColorStyle::new(Color::Light(BaseColor::Cyan), Color::Dark(BaseColor::Blue))
                }
            );

            for c in &row[(cursor.col + 1)..] {
                spanned_string.append_plain(format!("{}", c.ascii))
            }

            spanned_string.append_plain("\n");
        }
    }

    spanned_string
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

    let mut cursive_cursor  = Cursor::new(3, 2);
    let cursive_content = TextContent::new(generate_text(&data, &cursive_cursor));

    let mut siv = Cursive::new();
    siv.add_layer(
        Panel::new(
            PaddedView::lrtb(4, 4, 2, 2,
                NamedView::new(
                    "zestien",
                    TextView::new_with_content(
                        cursive_content.clone()
                    )
                )
            )
        )
    );

    siv.add_global_callback(Event::Refresh, move |_| cursive_content.set_content(generate_text(&data, &cursive_cursor)));
    siv.add_global_callback(Event::Key(Key::Right), move |_| cursive_cursor.col += 1 );

    siv.run();
}
