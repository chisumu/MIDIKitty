use std::collections::HashMap;
use std::fmt;

use color_eyre::{Result, eyre::eyre};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use midikitty::engine::Synth;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Direction, Layout},
    prelude::{Alignment, Buffer, Constraint, Rect},
    style::{Color, Stylize},
    text::Line,
    widgets::{Block, Paragraph, Widget},
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
    pad_state: Vec<PadState>,
    pad_config: Vec<PadConfig>,
}

#[derive(Debug, Default, Clone)]
struct PadState {
    active: bool,
}

#[derive(Debug, Default, Clone)]
struct PadConfig {
    note: u32,
    velocity: u32,
}

// TODO: Make this just a keyboard layout, then subselect a portion
// to use for the grid depeneding on the number of rows/columns
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
            let g = self.grid_index(row, col);

            if self.pad_state[g].active {
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
    pub fn new(rows: usize, cols: usize) -> Self {
        let mut app = Self::default();

        app.rows = rows;
        app.cols = cols;
        app.pad_state = vec![PadState::default(); rows * cols];
        app.pad_config = vec![PadConfig::default(); rows * cols];

        app
    }

    fn grid_index(&self, row: usize, col: usize) -> usize {
        row * self.cols + col
    }

    fn play(&mut self, row: usize, col: usize) {
        let g = self.grid_index(row, col);

        // TODO: #5 Unset after some timeout instead of on press
        for i in 0..(self.rows * self.cols) {
            if i == g {
                self.pad_state[i].active = true;
            } else {
                self.pad_state[i].active = false;
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub enum AppMode {
    #[default]
    MIDI,
    Synth,
    Edit,
}

impl fmt::Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode_str = match self {
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

    keymap: HashMap<KeyCode, (usize, usize)>,

    engine: Synth,
}

impl MIDIKitty {
    // Grid Keymap

    /// Construct a new instance of [`MIDIKitty`].
    pub fn new() -> Self {
        let mut app = Self::default();

        app.grid = Grid::new(3, 8);
        app.keymap = HashMap::from([
            (KeyCode::Char('q'), (0, 0)),
            (KeyCode::Char('w'), (0, 1)),
            (KeyCode::Char('e'), (0, 2)),
            (KeyCode::Char('r'), (0, 3)),
            (KeyCode::Char('t'), (0, 4)),
            (KeyCode::Char('y'), (0, 5)),
            (KeyCode::Char('u'), (0, 6)),
            (KeyCode::Char('i'), (0, 7)),
            (KeyCode::Char('a'), (1, 0)),
            (KeyCode::Char('s'), (1, 1)),
            (KeyCode::Char('d'), (1, 2)),
            (KeyCode::Char('f'), (1, 3)),
            (KeyCode::Char('g'), (1, 4)),
            (KeyCode::Char('h'), (1, 5)),
            (KeyCode::Char('j'), (1, 6)),
            (KeyCode::Char('k'), (1, 7)),
            (KeyCode::Char('z'), (2, 0)),
            (KeyCode::Char('x'), (2, 1)),
            (KeyCode::Char('c'), (2, 2)),
            (KeyCode::Char('v'), (2, 3)),
            (KeyCode::Char('b'), (2, 4)),
            (KeyCode::Char('n'), (2, 5)),
            (KeyCode::Char('m'), (2, 6)),
            (KeyCode::Char(','), (2, 7)),
        ]);

        app
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        self.connect()?;

        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }

        Ok(())
    }

    fn connect(&mut self) -> Result<()> {
        self.engine.connect().map_err(|_| eyre!("cannot connect"))?;
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
            .constraints(vec![Constraint::Percentage(10), Constraint::Percentage(90)])
            .split(frame.area());

        frame.render_widget(title, layout[0]);

        match self.mode {
            AppMode::MIDI | AppMode::Synth => {
                frame.render_widget(&self.grid, layout[1]);
            }
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
        self.engine.play(self.note_number(row, col));
    }

    fn note_number(&self, row: usize, col: usize) -> u8 {
        36 + (row * self.grid.cols + col) as u8
    }

    fn switch_mode(&mut self) {
        self.mode = match self.mode {
            AppMode::MIDI => AppMode::Synth,
            AppMode::Synth => AppMode::MIDI,
            AppMode::Edit => self.mode.clone(),
        }
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        let mapped_key = self.keymap.get(&key.code);

        if key.modifiers.is_empty() && mapped_key.is_some() {
            let mapped_grid = mapped_key.unwrap();
            self.play_key(mapped_grid.0, mapped_grid.1);
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Esc)
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Tab) => self.switch_mode(),

            // Add other key handlers here.
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
