// Zestien, a hex editor by Simeon Duwel.

const BYTE_WIDTH: usize = 16;
const PADDING: usize = 2;

use std::{env, fs::File, io::{BufReader, Read}};

extern crate cursive;
use cursive::views::Panel;
use cursive::utils::span::{SpannedString, SpannedStr};
use cursive::theme::{Style, Effect, ColorStyle, Color, BaseColor};
use cursive::reexports::enumset::enum_set;
use cursive::event::{Event, Key, EventResult};
use cursive::View;
use cursive::{Cursive, CursiveExt};

fn nybble_to_hex(n: u8) -> char {
    if n < 10 {
        return (n + 0x30) as char;
    } else if n < 16 {
        return (n - 9 + 0x60) as char;
    }

    unreachable!("Nybbles are always 0x0F or less, received {}", n);
}

struct CharPrintingInfo {
    lower: char,
    upper: char,
    text: char
}

impl From<Option<u8>> for CharPrintingInfo {
    fn from(val: Option<u8>) -> Self {
        if let Some(v) = val {
            CharPrintingInfo {
                lower: nybble_to_hex(v & 0xf),
                upper: nybble_to_hex(v >> 4),
                text:  if v.is_ascii_graphic() { v as char } else { '.' }
            }
        } else {
            CharPrintingInfo {
                lower: '~',
                upper: '~',
                text:  ' '
            }
        }
    }
}

impl CharPrintingInfo {
    fn byte(&self)  -> String { format!("{}{} ", self.upper, self.lower) }
    fn ascii(&self) -> String { format!("{}", self.text) }
}

struct ZestienView {
    data: Vec<Option<u8>>,
    /// The cursor points to a byte, not a nybble.
    cursor: usize,
    /// Are we editing the lower nybble of the byte the cursor is pointing to?
    on_lower_nybble: bool,
    scroll_row_offset: usize,
    visible_rows: usize,
}

impl ZestienView {
    fn new() -> Self {
        ZestienView { data: Vec::new(), cursor: 0, on_lower_nybble: false, scroll_row_offset: 0, visible_rows: 1 }
    }
    fn with_data(data: Vec<Option<u8>>) -> Self {
        ZestienView { data, cursor: 0, on_lower_nybble: false, scroll_row_offset: 0, visible_rows: 16 }
    }

    fn get_cursor_pos(&self)  -> (usize, usize) { (self.cursor % BYTE_WIDTH, self.cursor / BYTE_WIDTH) }
    fn move_cursor(&mut self, offset: isize)    {
        self.cursor = (self.cursor as isize + offset).clamp(0, (self.data.len() - 1) as isize) as usize;

        // take into account scrolling
        if self.cursor >= (BYTE_WIDTH * (self.scroll_row_offset + self.visible_rows)) { self.scroll_row_offset += 1; }
        if self.cursor < (BYTE_WIDTH * self.scroll_row_offset) { self.scroll_row_offset -= 1; }
    }
    fn nybble_move(&mut self, forward: bool)    {
        if  forward &&  self.on_lower_nybble { self.move_cursor( 1) }
        if !forward && !self.on_lower_nybble { self.move_cursor(-1) }

        // edge cases to stop bouncing at the beginning or ending of the file
        if !(!forward && !self.on_lower_nybble && self.cursor == 0) && !(forward && self.on_lower_nybble && self.cursor == self.data.len()) {
            self.on_lower_nybble = !self.on_lower_nybble;
        }
    }
    const ROW_LENGTH: usize = 8 + 2 + 3 * BYTE_WIDTH + 2 + BYTE_WIDTH;
    fn generate_text(&self, rows: usize) -> Vec<SpannedString<Style>> {
        let (c_col, c_row) = self.get_cursor_pos();

        let row_iter = self.data.chunks(BYTE_WIDTH).skip(self.scroll_row_offset).take(rows);

        let all_rows = row_iter.enumerate().map(|(screen_row_idx, row)|  {
            let row_idx = screen_row_idx + self.scroll_row_offset;

            let mut spanned_string = SpannedString::new();

            if row_idx != c_row {
                spanned_string.append_plain(format!(
                        "{}: {}| {}",
                        format!("{:08x}", row_idx * BYTE_WIDTH),
                        row.iter().map(|c| CharPrintingInfo::from(*c).byte()  ).collect::<Vec<_>>().join(""),
                        row.iter().map(|c| CharPrintingInfo::from(*c).ascii() ).collect::<Vec<_>>().join("")));
            } else {
                spanned_string.append_plain(format!("{:08x}: ", row_idx * BYTE_WIDTH));
                row[..c_col].into_iter().map(|c| CharPrintingInfo::from(*c).byte()).for_each(|s| spanned_string.append_plain(s));

                spanned_string.append_styled(
                    CharPrintingInfo::from(row[c_col]).upper.to_string(),
                    Style {
                        effects: enum_set!(Effect::Bold),
                        color: ColorStyle::new(
                            if !self.on_lower_nybble { Color::Light(BaseColor::Cyan) } else { Color::Dark(BaseColor::White) },
                            Color::Dark(BaseColor::Blue)
                        )
                    }
                );
                spanned_string.append_styled(
                    CharPrintingInfo::from(row[c_col]).lower.to_string(),
                    Style {
                        effects: enum_set!(Effect::Bold),
                        color: ColorStyle::new(
                            if self.on_lower_nybble { Color::Light(BaseColor::Cyan) } else { Color::Dark(BaseColor::White) },
                            Color::Dark(BaseColor::Blue)
                        )
                    }
                );
                spanned_string.append_plain(" ");

                row[(c_col + 1)..].into_iter().map(|c| CharPrintingInfo::from(*c).byte()).for_each(|s| spanned_string.append_plain(s));

                spanned_string.append_plain("| ");

                row[..c_col].into_iter().map(|c| CharPrintingInfo::from(*c).ascii()).for_each(|s| spanned_string.append_plain(s));

                spanned_string.append_styled(
                    CharPrintingInfo::from(row[c_col]).ascii(),
                    Style {
                        effects: enum_set!(Effect::Bold),
                        color: ColorStyle::new(Color::Light(BaseColor::Cyan), Color::Dark(BaseColor::Blue))
                    }
                );

                row[(c_col + 1)..].into_iter().map(|c| CharPrintingInfo::from(*c).ascii()).for_each(|s| spanned_string.append_plain(s));
            }
            return spanned_string;
        }).collect::<Vec<_>>();

        return all_rows;
    }

    fn edit_data(&mut self, val: u8) {
        if self.data[self.cursor].is_none() {
            self.data[self.cursor] = Some(0);
        }

        let mut original = self.data[self.cursor].unwrap();
        original &= if self.on_lower_nybble { 0xf0 } else { 0x0f };
        original |= val << if self.on_lower_nybble { 0 } else { 4 };
        self.data[self.cursor] = Some(original);

        //advance the cursor after typing something
        self.nybble_move(true);
    }
}

impl View for ZestienView {
    fn draw(&self, printer: &cursive::Printer) {
        let gen = self.generate_text(self.visible_rows);
        let window = printer.windowed(
            cursive::Rect::from_corners(
                (PADDING, PADDING),
                (PADDING + ZestienView::ROW_LENGTH, PADDING + self.visible_rows)
            )
        );

        for i in 0..self.visible_rows {
            let current_row = &gen[i];
            window.print_styled((0, i), SpannedStr::new(current_row.source(), current_row.spans_raw()));
        }
    }
    fn required_size(&mut self, _constraint: cursive::Vec2) -> cursive::Vec2 {
        cursive::Vec2::new(ZestienView::ROW_LENGTH + 2 * PADDING, self.visible_rows + 2 * PADDING)
    }
    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            // NAVIGATION
            Event::Key(Key::Right) => {
                self.nybble_move(true);
                EventResult::Ignored
            }
            Event::Key(Key::Left) => {
                self.nybble_move(false);
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

            // HEX EDITING

            Event::Char('0') => {self.edit_data(0); EventResult::Ignored}
            Event::Char('1') => {self.edit_data(1); EventResult::Ignored}
            Event::Char('2') => {self.edit_data(2); EventResult::Ignored}
            Event::Char('3') => {self.edit_data(3); EventResult::Ignored}
            Event::Char('4') => {self.edit_data(4); EventResult::Ignored}
            Event::Char('5') => {self.edit_data(5); EventResult::Ignored}
            Event::Char('6') => {self.edit_data(6); EventResult::Ignored}
            Event::Char('7') => {self.edit_data(7); EventResult::Ignored}
            Event::Char('8') => {self.edit_data(8); EventResult::Ignored}
            Event::Char('9') => {self.edit_data(9); EventResult::Ignored}
            Event::Char('a') => {self.edit_data(10); EventResult::Ignored}
            Event::Char('b') => {self.edit_data(11); EventResult::Ignored}
            Event::Char('c') => {self.edit_data(12); EventResult::Ignored}
            Event::Char('d') => {self.edit_data(13); EventResult::Ignored}
            Event::Char('e') => {self.edit_data(14); EventResult::Ignored}
            Event::Char('f') => {self.edit_data(15); EventResult::Ignored}

            // FILE HANDLING
            Event::CtrlChar('O') => unimplemented!("Opening files"),
            Event::CtrlChar('S') => unimplemented!("Saving files"),
            _ => EventResult::Ignored
        }
    }
}

fn main() {
    let maybe_path = &env::args().collect::<Vec<String>>()[1];
    let file = File::open(maybe_path).expect("Could not open or find the supplied file.");

    let mut buf = String::with_capacity(1 << 16);
    let _reader = BufReader::new(file).read_to_string(&mut buf);

    let mut data: Vec<_> = buf.as_bytes().into_iter().map(|e| Some(*e)).collect();
    let extra_chars = vec![None; 17 - ((data.len() + 1) % 16)];

    data.extend(extra_chars);
    let zestien_view = ZestienView::with_data(data);
    let mut siv = Cursive::new();
    siv.add_layer(Panel::new(zestien_view));
    siv.run();
}
