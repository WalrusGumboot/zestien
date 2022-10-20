// Zestien, a hex editor by Simeon Duwel.

const BYTE_WIDTH: usize = 16;

use std::{env, fs::File, io::{BufReader, Read}};

extern crate cursive;
use cursive::views::{NamedView, PaddedView, Panel};
use cursive::utils::span::{SpannedString, SpannedStr};
use cursive::theme::{Style, Effect, ColorStyle, Color, BaseColor};
use cursive::reexports::enumset::enum_set;
use cursive::event::{Event, Key, EventResult};
use cursive::View;
use cursive::{Cursive, CursiveExt, XY};

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
            ascii: if (source as char).is_ascii_graphic() { source as char } else { '.' } //TODO: add nerd font support for newline char
        }
    }
}

impl std::fmt::Display for CharRep {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}{}", self.upper, self.lower)
    }
}


struct ZestienView {
    data: Vec<CharRep>,
    cursor: usize,
    scroll_row_offset: usize,
    visible_rows: usize,
    padding: usize
}

impl ZestienView {
    fn get_cursor_pos(&self)  -> (usize, usize)  { (self.cursor % BYTE_WIDTH, self.cursor / BYTE_WIDTH) }
    fn cursor_on_lower(&self) -> bool            { self.cursor % 2 == 0 }
    fn move_cursor(&mut self, offset: isize)     { self.cursor = (self.cursor as isize + offset).clamp(0, self.data.len() as isize) as usize }

    const ROW_LENGTH: usize = 8 + 2 + 3 * BYTE_WIDTH + 2 + BYTE_WIDTH;
    fn generate_text(&self, rows: usize) -> Vec<SpannedString<Style>> {
        let (c_col, c_row) = self.get_cursor_pos();
        let c_lower = self.cursor_on_lower();

        let row_iter = self.data.chunks(BYTE_WIDTH).skip(self.scroll_row_offset).take(rows);

        let all_rows = row_iter.enumerate().map(|(screen_row_idx, row)|  {
            let row_idx = screen_row_idx + self.scroll_row_offset;

            let mut spanned_string = SpannedString::new();

            if row_idx != c_row {
                spanned_string.append_plain(format!(
                        "{}: {} | {}\n",
                        format!("{:08x}", row_idx * BYTE_WIDTH),
                        row.iter().map(|c| format!("{}", c)).collect::<Vec<_>>().join(" "),
                        row.iter().map(|c| String::from(c.ascii)).collect::<Vec<_>>().join("")));
            } else {
                spanned_string.append_plain(format!("{:08x}: ", row_idx * BYTE_WIDTH));
                for c in &row[..c_col] {
                    spanned_string.append_plain(format!("{} ", c));
                }
                spanned_string.append_styled(
                    format!("{}",  row[c_col].upper),
                    Style {
                        effects: enum_set!(Effect::Bold),
                        color: ColorStyle::new(
                            if !c_lower { Color::Light(BaseColor::Cyan) } else { Color::Dark(BaseColor::White) },
                            Color::Dark(BaseColor::Blue)
                        )
                    }
                );
                spanned_string.append_styled(
                    format!("{}", row[c_col].lower),
                    Style {
                        effects: enum_set!(Effect::Bold),
                        color: ColorStyle::new(
                            if c_lower { Color::Light(BaseColor::Cyan) } else { Color::Dark(BaseColor::White) },
                            Color::Dark(BaseColor::Blue)
                        )
                    }
                );
                spanned_string.append_plain(" ");
                for c in &row[(c_col + 1)..] {
                    spanned_string.append_plain(format!("{} ", c));
                }
                spanned_string.append_plain("| ");
                for c in &row[..c_col] {
                    spanned_string.append_plain(format!("{}", c.ascii))
                }
                spanned_string.append_styled(
                    format!("{}", row[c_col].ascii),
                    Style {
                        effects: enum_set!(Effect::Bold),
                        color: ColorStyle::new(Color::Light(BaseColor::Cyan), Color::Dark(BaseColor::Blue))
                    }
                );
                for c in &row[(c_col + 1)..] {
                    spanned_string.append_plain(format!("{}", c.ascii))
                }
                spanned_string.append_plain("\n");

            }
            return spanned_string;
        }).collect::<Vec<_>>();

        return all_rows;
    }
}

impl View for ZestienView {
    fn draw(&self, printer: &cursive::Printer) {
        let gen = self.generate_text(self.visible_rows);
        let mut window = printer.windowed(cursive::Rect::from_corners((self.padding, self.padding), (self.padding + ZestienView::ROW_LENGTH, self.padding + self.visible_rows)));

        for i in 0..self.visible_rows {
            let current_row = &gen[i];
            window.print_styled(XY::new(0, i), SpannedStr::new(current_row.source(), current_row.spans_raw()));
        }
    }
    fn required_size(&mut self, _constraint: cursive::Vec2) -> cursive::Vec2 {
        cursive::Vec2::new(ZestienView::ROW_LENGTH + 2 * self.padding, self.visible_rows + 2 * self.padding)
    }
    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Right) => {
                self.move_cursor(1);
                EventResult::Ignored
            }
            Event::Key(Key::Left) => {
                self.move_cursor(-1);
                EventResult::Ignored
            }
            Event::Key(Key::Up) => {
                self.move_cursor(-16); // TODO: take BYTE_WIDTH into account
                EventResult::Ignored
            }
            Event::Key(Key::Down) => {
                self.move_cursor(16);
                EventResult::Ignored
            }
            _ => EventResult::Ignored
        }
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

    let zestien_view = ZestienView { data, cursor: 0, scroll_row_offset: 0, visible_rows: 10, padding: 2 };

    let mut siv = Cursive::new();
    siv.add_layer(Panel::new(zestien_view));

    siv.run();
}
