use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;
use terminalist::badge::*;
use terminalist::terminal_badge::*;

struct App {
    should_quit: bool,
    current_page: usize,
}

impl App {
    fn new() -> App {
        App {
            should_quit: false,
            current_page: 0,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    app.should_quit = true;
                }
                KeyCode::Left => {
                    if app.current_page > 0 {
                        app.current_page -= 1;
                    }
                }
                KeyCode::Right => {
                    if app.current_page < 2 {
                        app.current_page += 1;
                    }
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    // Title
    let title = Paragraph::new("🎨 Badge Demo - Press ← → to navigate, 'q' to quit")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Content based on current page
    match app.current_page {
        0 => render_original_badges(f, chunks[1]),
        1 => render_terminal_badges(f, chunks[1]),
        2 => render_task_examples(f, chunks[1]),
        _ => {}
    }

    // Footer
    let footer_text = match app.current_page {
        0 => "Page 1/3: Original Bootstrap-style Badges",
        1 => "Page 2/3: Terminal-optimized Badges",
        2 => "Page 3/3: Task Examples",
        _ => "",
    };
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

fn render_original_badges(f: &mut Frame, area: ratatui::layout::Rect) {
    let items = vec![
        ListItem::new(Line::from(vec![
            Span::raw("Primary: "),
            create_badge("PRIMARY", BadgeStyle::Primary),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("Success: "),
            create_badge("SUCCESS", BadgeStyle::Success),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("Warning: "),
            create_badge("WARNING", BadgeStyle::Warning),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("Danger:  "),
            create_badge("DANGER", BadgeStyle::Danger),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("Info:    "),
            create_badge("INFO", BadgeStyle::Info),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("Compact: "),
            create_compact_badge("🔄", "REC", BadgeStyle::Primary),
            Span::raw(" "),
            create_compact_badge("⏰", "DUE", BadgeStyle::Danger),
        ])),
    ];

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Original Badges (Background Colors)"),
    );
    f.render_widget(list, area);
}

fn render_terminal_badges(f: &mut Frame, area: ratatui::layout::Rect) {
    let test_badges = create_test_badges();
    let items: Vec<ListItem> = test_badges
        .into_iter()
        .map(|(name, badge)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<18}: ", name), Style::default().fg(Color::Gray)),
                badge,
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Terminal-Optimized Badges (Reverse Video & Brackets)"),
    );
    f.render_widget(list, area);
}

fn render_task_examples(f: &mut Frame, area: ratatui::layout::Rect) {
    let items = vec![
        ListItem::new(Line::from(vec![
            Span::raw("Original style: Buy groceries "),
            create_compact_badge("🔄", "REC", BadgeStyle::Primary),
            Span::raw(" "),
            create_badge("2h", BadgeStyle::Warning),
            Span::raw(" "),
            create_badge("3L", BadgeStyle::Success),
        ])),
        ListItem::new(Line::from({
            let mut spans = vec![Span::raw("Terminal style: Buy groceries ")];
            spans.extend(create_terminal_task_badges(true, true, Some("2h"), 3));
            spans
        })),
        ListItem::new(Line::from(vec![
            Span::raw("Priority badges: "),
            create_priority_badge(4).unwrap_or(Span::raw("None")),
            Span::raw(" "),
            create_terminal_priority_badge(4).unwrap_or(Span::raw("None")),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("Mixed styles: Important task "),
            create_bracket_badge("URGENT", TerminalBadgeStyle::Danger),
            Span::raw(" "),
            create_paren_badge("1h", TerminalBadgeStyle::Warning),
        ])),
    ];

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Task Examples"),
    );
    f.render_widget(list, area);
}
