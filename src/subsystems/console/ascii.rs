use core::fmt;

use crate::{
    subsystems::display::text::TextScreen,
    util::grid::{GridPosition, GridSize},
};

use super::{Console, ConsoleColors};

const TAB_WIDTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsciiConsoleError<E> {
    Screen(E),
    CursorOutOfBounds,
}

pub struct AsciiConsole<S>
where
    S: TextScreen,
{
    screen: S,
    size: GridSize,
    cursor: GridPosition,
    colors: ConsoleColors,
    wrap_pending: bool,
}

impl<S> AsciiConsole<S>
where
    S: TextScreen,
{
    /// 화면을 사용하는 ASCII 콘솔을 생성합니다.
    ///
    /// 열이나 행이 하나도 없는 화면에는 콘솔을 생성할 수 없습니다.
    pub fn new(screen: S, colors: ConsoleColors) -> Option<Self> {
        let size = screen.size();
        if size.columns == 0 || size.rows == 0 {
            return None;
        }

        Some(Self {
            screen,
            size,
            cursor: GridPosition::new(0, 0),
            colors,
            wrap_pending: false,
        })
    }

    pub fn with_default_colors(screen: S) -> Option<Self> {
        Self::new(screen, ConsoleColors::default())
    }

    pub const fn screen(&self) -> &S {
        &self.screen
    }

    pub fn screen_mut(&mut self) -> &mut S {
        &mut self.screen
    }

    pub fn into_screen(self) -> S {
        self.screen
    }

    fn next_line(&mut self) {
        self.cursor.column = 0;
        self.wrap_pending = false;

        if self.cursor.row == self.size.rows - 1 {
            self.screen.scroll(1, self.colors.background);
        } else {
            self.cursor.row += 1;
        }
    }

    fn carriage_return(&mut self) {
        self.cursor.column = 0;
        self.wrap_pending = false;
    }

    fn backspace(&mut self) {
        if self.wrap_pending {
            self.wrap_pending = false;
        } else if self.cursor.column > 0 {
            self.cursor.column -= 1;
        }
    }

    fn write_printable(&mut self, ch: char) -> Result<(), AsciiConsoleError<S::Error>> {
        if self.wrap_pending {
            self.next_line();
        }

        self.screen
            .put_char(
                self.cursor,
                ch,
                self.colors.foreground,
                self.colors.background,
            )
            .map_err(AsciiConsoleError::Screen)?;

        if self.cursor.column == self.size.columns - 1 {
            self.wrap_pending = true;
        } else {
            self.cursor.column += 1;
        }

        Ok(())
    }

    fn write_tab(&mut self) -> Result<(), AsciiConsoleError<S::Error>> {
        if self.wrap_pending {
            self.next_line();
        }

        let spaces = TAB_WIDTH - (self.cursor.column % TAB_WIDTH);
        for _ in 0..spaces {
            self.write_printable(' ')?;
        }
        Ok(())
    }
}

impl<S> Console for AsciiConsole<S>
where
    S: TextScreen,
{
    type Error = AsciiConsoleError<S::Error>;

    fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error> {
        match byte {
            b'\n' => self.next_line(),
            b'\r' => self.carriage_return(),
            b'\t' => return self.write_tab(),
            0x08 => self.backspace(),
            0x20..=0x7e => return self.write_printable(char::from(byte)),
            0x80..=0xff => return self.write_printable('?'),
            _ => {}
        }

        Ok(())
    }

    fn size(&self) -> GridSize {
        self.size
    }

    fn cursor_position(&self) -> GridPosition {
        self.cursor
    }

    fn set_cursor_position(&mut self, position: GridPosition) -> Result<(), Self::Error> {
        if !self.size.contains(position) {
            return Err(AsciiConsoleError::CursorOutOfBounds);
        }

        self.cursor = position;
        self.wrap_pending = false;
        Ok(())
    }

    fn colors(&self) -> ConsoleColors {
        self.colors
    }

    fn set_colors(&mut self, colors: ConsoleColors) {
        self.colors = colors;
    }

    fn clear(&mut self) {
        self.screen.clear(self.colors.background);
        self.cursor = GridPosition::new(0, 0);
        self.wrap_pending = false;
    }
}

impl<S> fmt::Write for AsciiConsole<S>
where
    S: TextScreen,
{
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.write_bytes(text.as_bytes()).map_err(|_| fmt::Error)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        subsystems::{
            console::{Console, ConsoleColors},
            display::text::TextScreen,
        },
        util::{
            color::Color,
            grid::{GridPosition, GridSize},
        },
    };

    use super::{AsciiConsole, AsciiConsoleError};

    const COLUMNS: usize = 4;
    const ROWS: usize = 2;

    struct TestScreen {
        cells: [[char; COLUMNS]; ROWS],
        clear_color: Option<Color>,
        put_count: usize,
    }

    impl TestScreen {
        fn new() -> Self {
            Self {
                cells: [[' '; COLUMNS]; ROWS],
                clear_color: None,
                put_count: 0,
            }
        }
    }

    impl TextScreen for TestScreen {
        type Error = ();

        fn size(&self) -> GridSize {
            GridSize::new(COLUMNS, ROWS)
        }

        fn put_char(
            &mut self,
            position: GridPosition,
            ch: char,
            _foreground: Color,
            _background: Color,
        ) -> Result<(), Self::Error> {
            if !self.size().contains(position) {
                return Err(());
            }
            self.cells[position.row][position.column] = ch;
            self.put_count += 1;
            Ok(())
        }

        fn fill_cell(&mut self, position: GridPosition, _color: Color) -> Result<(), Self::Error> {
            if !self.size().contains(position) {
                return Err(());
            }
            self.cells[position.row][position.column] = ' ';
            Ok(())
        }

        fn clear(&mut self, color: Color) {
            self.cells = [[' '; COLUMNS]; ROWS];
            self.clear_color = Some(color);
        }

        fn scroll(&mut self, rows: isize, _erase_color: Color) {
            assert_eq!(rows, 1);
            self.cells[0] = self.cells[1];
            self.cells[1] = [' '; COLUMNS];
        }
    }

    fn console() -> AsciiConsole<TestScreen> {
        AsciiConsole::with_default_colors(TestScreen::new()).expect("test screen is non-empty")
    }

    #[test]
    fn wraps_and_scrolls_only_when_the_next_character_arrives() {
        let mut console = console();

        console.write_bytes(b"abcdefgh").unwrap();
        assert_eq!(
            console.screen().cells,
            [['a', 'b', 'c', 'd'], ['e', 'f', 'g', 'h']]
        );
        assert_eq!(console.cursor_position(), GridPosition::new(3, 1));

        console.write_byte(b'i').unwrap();
        assert_eq!(
            console.screen().cells,
            [['e', 'f', 'g', 'h'], ['i', ' ', ' ', ' ']]
        );
        assert_eq!(console.cursor_position(), GridPosition::new(1, 1));
    }

    #[test]
    fn handles_newline_carriage_return_tab_and_backspace() {
        let mut console = console();

        console.write_bytes(b"ab\x08X\nY\rZ\t").unwrap();

        assert_eq!(
            console.screen().cells,
            [['a', 'X', ' ', ' '], ['Z', ' ', ' ', ' ',]]
        );
        assert_eq!(console.cursor_position(), GridPosition::new(3, 1));
    }

    #[test]
    fn backspace_moves_without_erasing_the_previous_cell() {
        let mut console = console();

        console.write_bytes(b"abc\x08").unwrap();

        assert_eq!(console.screen().cells[0], ['a', 'b', 'c', ' ']);
        assert_eq!(console.cursor_position(), GridPosition::new(2, 0));
    }

    #[test]
    fn backspace_cancels_pending_wrap_without_erasing_the_last_cell() {
        let mut console = console();

        console.write_bytes(b"abcd\x08").unwrap();

        assert_eq!(console.screen().cells[0], ['a', 'b', 'c', 'd']);
        assert_eq!(console.cursor_position(), GridPosition::new(3, 0));
        console.write_byte(b'X').unwrap();
        assert_eq!(console.screen().cells[0], ['a', 'b', 'c', 'X']);
    }

    #[test]
    fn tab_after_pending_wrap_uses_the_new_line_column() {
        let mut console = console();

        console.write_bytes(b"abcd\t").unwrap();

        assert_eq!(console.screen().put_count, 8);
        assert_eq!(console.cursor_position(), GridPosition::new(3, 1));
    }

    #[test]
    fn rejects_an_out_of_bounds_cursor_position() {
        let mut console = console();

        assert_eq!(
            console.set_cursor_position(GridPosition::new(COLUMNS, 0)),
            Err(AsciiConsoleError::CursorOutOfBounds),
        );
        assert_eq!(console.cursor_position(), GridPosition::new(0, 0));
    }

    #[test]
    fn clear_uses_the_current_background_and_resets_the_cursor() {
        let mut console = console();
        let colors = ConsoleColors::new(Color::GREEN, Color::BLUE);
        console.set_colors(colors);
        console.write_bytes(b"abc").unwrap();

        console.clear();

        assert_eq!(console.colors(), colors);
        assert_eq!(console.cursor_position(), GridPosition::new(0, 0));
        assert_eq!(console.screen().clear_color, Some(Color::BLUE));
        assert_eq!(console.screen().cells, [[' '; COLUMNS]; ROWS]);
    }

    #[test]
    fn replaces_non_ascii_bytes_and_ignores_other_controls() {
        let mut console = console();

        console.write_bytes(&[0x01, 0x80, b'A', 0x7f]).unwrap();

        assert_eq!(console.screen().cells[0], ['?', 'A', ' ', ' ']);
    }
}
