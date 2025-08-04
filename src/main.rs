use std::fmt;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    style::{Stylize, Color},
    text::Line,
    layout::{Layout, Direction},
    prelude::{Buffer, Constraint, Alignment, Rect},
    widgets::{Widget, Block, Paragraph},
};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = MIDIKitty::new().run(terminal);
    ratatui::restore();
    result
}

#[derive(Debug, Default)]
struct Grid {
    cols: usize,
    rows: usize,
    active_cell: usize,
}

const GRID_LETTERS: [[&str; 10]; 3] = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "w", "s", "d", "f", "g", "h", "j", "k", "l"],
    ["z", "x", "c", "v", "b", "n", "m", "<", ">", "."],
];

impl Widget for &Grid {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let col_constraints = (0..self.cols).map(|_| Constraint::Length(9));
        let row_constraints = (0..self.rows).map(|_| Constraint::Length(3));
        let horizontal = Layout::horizontal(col_constraints).spacing(1);
        let vertical = Layout::vertical(row_constraints).spacing(1);

        let rows = vertical.split(area);
        let cells = rows.iter().flat_map(|&row| horizontal.split(row).to_vec());

        for (i, cell) in cells.enumerate() {
            let row = i / self.cols;
            let col = i % self.cols;

            if i == self.active_cell {
                Paragraph::new(format!("HIT"))
                    .alignment(Alignment::Center)
                    .block(Block::bordered())
                    .bg(Color::Green)
                    .render(cell, buf);
            } else {
                Paragraph::new(format!("{}", GRID_LETTERS[row][col]))
                    .alignment(Alignment::Center)
                    .block(Block::bordered())
                    .render(cell, buf);
            }
        }
    }
}

impl Grid {
    fn play(&mut self, row: usize, col: usize) {
        self.active_cell = row * self.cols + col;
    }        
}

#[derive(Clone, PartialEq, Debug, Default)]
pub enum AppMode {
    #[default] MIDI,
    Synth,
    Edit,
}

impl fmt::Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode_str = match self  {
            AppMode::MIDI => "MIDI",
            AppMode::Synth => "Synth",
            AppMode::Edit => "Synth (EDITING)",
        };
        write!(f, "{}", mode_str)
    }
}


/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct MIDIKitty {
    /// Is the application running?
    running: bool,

    // Current application mode
    mode: AppMode,

    // UI Elements`
    grid: Grid,
}

impl MIDIKitty {
    /// Construct a new instance of [`MIDIKitty`].
    pub fn new() -> Self {
        let mut app = Self::default();

        app.grid = Grid{cols: 8, rows: 3, active_cell: 0};

        app
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;

        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }

        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        let title = Line::from(format!("MIDIKitty [{}]", self.mode))
            .bold()
            .blue()
            .centered();

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(10),
                Constraint::Percentage(90)
            ])
            .split(frame.area());

        frame.render_widget(title, layout[0]);

        match self.mode {
            AppMode::MIDI => { frame.render_widget(&self.grid, layout[1]); }
            AppMode::Synth => {}
            AppMode::Edit => {}
        }
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    fn play_key(&mut self, row: usize, col: usize) {
        self.grid.play(row, col);
    }

    fn switch_mode(&mut self) {
        self.mode = match self.mode {
            AppMode::MIDI => AppMode::Synth,
            AppMode::Synth => AppMode::MIDI,
            AppMode::Edit => self.mode.clone()
        }
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc)
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Tab) => self.switch_mode(),
            (_, KeyCode::Char('q')) => self.play_key(0, 0),
            (_, KeyCode::Char('w')) => self.play_key(0, 1),
            (_, KeyCode::Char('e')) => self.play_key(0, 2),
            (_, KeyCode::Char('r')) => self.play_key(0, 3),
            (_, KeyCode::Char('t')) => self.play_key(0, 4),
            (_, KeyCode::Char('y')) => self.play_key(0, 5),
            (_, KeyCode::Char('u')) => self.play_key(0, 6),
            (_, KeyCode::Char('i')) => self.play_key(0, 7),
            (_, KeyCode::Char('a')) => self.play_key(1, 0),
            (_, KeyCode::Char('s')) => self.play_key(1, 1),
            (_, KeyCode::Char('d')) => self.play_key(1, 2),
            (_, KeyCode::Char('f')) => self.play_key(1, 3),
            (_, KeyCode::Char('g')) => self.play_key(1, 4),
            (_, KeyCode::Char('h')) => self.play_key(1, 5),
            (_, KeyCode::Char('j')) => self.play_key(1, 6),
            (_, KeyCode::Char('k')) => self.play_key(1, 7),
            (_, KeyCode::Char('z')) => self.play_key(2, 0),
            (_, KeyCode::Char('x')) => self.play_key(2, 1),
            (_, KeyCode::Char('c')) => self.play_key(2, 2),
            (_, KeyCode::Char('v')) => self.play_key(2, 3),
            (_, KeyCode::Char('b')) => self.play_key(2, 4),
            (_, KeyCode::Char('n')) => self.play_key(2, 5),
            (_, KeyCode::Char('m')) => self.play_key(2, 6),
            (_, KeyCode::Char(',')) => self.play_key(2, 7),

            // Add other key handlers here.
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
