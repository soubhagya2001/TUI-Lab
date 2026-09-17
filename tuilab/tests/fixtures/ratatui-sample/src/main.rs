//! Minimal deterministic TUI fixture for TUI Lab Phase 1.
//!
//! Screens (static strings, no clocks — snapshot-safe):
//! * list:   title `TUI-LAB-SAMPLE`, footer hints
//! * search: `/` opens `Search: <query>`, live filter
//! * detail: `ENTER` shows `Selected: <item>`
//! Quit with `q` (exit 0). `ESC` in search returns to the list.

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

/// Static catalogue. Stable strings the smoke test asserts on.
const ITEMS: &[&str] = &[
    "alpha-table",
    "beta-chair",
    "gamma-table",
    "delta-lamp",
    "epsilon-table",
];

#[derive(PartialEq)]
enum Mode {
    List,
    Search,
    Detail,
}

struct App {
    mode: Mode,
    query: String,
    state: ListState,
}

impl App {
    fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            mode: Mode::List,
            query: String::new(),
            state,
        }
    }

    /// Items visible under the current filter.
    fn visible(&self) -> Vec<&'static str> {
        ITEMS
            .iter()
            .copied()
            .filter(|item| item.contains(self.query.as_str()))
            .collect()
    }

    fn selected_item(&self) -> Option<&'static str> {
        let visible = self.visible();
        self.state.selected().and_then(|i| visible.get(i).copied())
    }

    fn move_down(&mut self) {
        let len = self.visible().len();
        if len == 0 {
            return;
        }
        let next = self.state.selected().map_or(0, |i| (i + 1) % len);
        self.state.select(Some(next));
    }

    fn move_up(&mut self) {
        let len = self.visible().len();
        if len == 0 {
            return;
        }
        let prev = self
            .state
            .selected()
            .map_or(0, |i| (i + len - 1) % len);
        self.state.select(Some(prev));
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let exit = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    exit
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui(frame, app))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        // ConPTY delivers press AND release records; react to presses only,
        // or every key would act twice.
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match app.mode {
            Mode::List => match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('/') => app.mode = Mode::Search,
                KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                KeyCode::Enter => {
                    if app.selected_item().is_some() {
                        app.mode = Mode::Detail;
                    }
                }
                _ => {}
            },
            Mode::Search => match key.code {
                KeyCode::Esc => {
                    app.mode = Mode::List;
                    app.query.clear();
                    app.state.select(Some(0));
                }
                KeyCode::Enter => {
                    if app.selected_item().is_some() {
                        app.mode = Mode::Detail;
                    }
                }
                KeyCode::Backspace => {
                    app.query.pop();
                    app.state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    app.query.push(c);
                    app.state.select(Some(0));
                }
                KeyCode::Down => app.move_down(),
                KeyCode::Up => app.move_up(),
                _ => {}
            },
            Mode::Detail => match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Esc => app.mode = Mode::List,
                _ => {}
            },
        }
    }
}

fn ui(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(frame.area());

    if app.mode == Mode::Detail {
        let item = app.selected_item().unwrap_or("?");
        let detail = Paragraph::new(format!("Selected: {item}"))
            .block(Block::default().borders(Borders::ALL).title("TUI-LAB-SAMPLE"));
        frame.render_widget(detail, chunks[0]);
    } else {
        let items: Vec<ListItem> = app
            .visible()
            .iter()
            .map(|item| ListItem::new(item.to_string()))
            .collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("TUI-LAB-SAMPLE"))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, chunks[0], &mut app.state);
    }

    let footer = match app.mode {
        Mode::Search => format!("Search: {}", app.query),
        Mode::List => "j/k or arrows move, / search, ENTER select, q quit".to_string(),
        Mode::Detail => "ESC back, q quit".to_string(),
    };
    let help =
        Paragraph::new(footer).block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(help, chunks[1]);
}
