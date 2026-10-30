// Terminal UI: draws the game with crossterm and handles keyboard input.
// The whole screen is redrawn from game state on every event, which keeps
// rendering simple and makes terminal resizes work for free.

use std::io::{self, Stdout, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};

use super::config::{Config, CHAR_SPACING, MIN_TERMINAL_WIDTH, WORD_LENGTH};
use super::gamestate::{GameData, GameStatus, KeyBoardHelperState};
use super::stats::{GameStats, StatsManager};
use super::words;

const KEYBOARD_ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
const TITLE: &str = env!("CARGO_PKG_NAME");

#[derive(Clone, Copy, PartialEq, Eq)]
enum TileStyle {
    Blank,                       // empty box waiting for a letter
    Typed,                       // letter typed in the current row
    Key,                         // keyboard key not yet used
    Scored(KeyBoardHelperState), // GRAY / AMBER / GREEN
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MessageKind {
    Info,
    Error,
    Success,
}

enum Action {
    Letter(char),
    Backspace,
    Enter,
    Quit,
    Other, // any other key
    Redraw,
}

/// Screen positions for one frame, computed from the terminal size
struct Layout {
    cols: u16,
    title_y: u16,
    grid_y: u16,
    gap: u16, // rows between grid/keyboard lines: 2 when there's room, else 1
    keyboard_y: u16,
    message_y: u16,
    footer_y: u16,
}

impl Layout {
    fn new(cols: u16, rows: u16, max_guesses: usize, legend: bool) -> Option<Layout> {
        if cols < MIN_TERMINAL_WIDTH as u16 {
            return None;
        }
        let height = |gap: u16| {
            let grid = (max_guesses as u16 - 1) * gap + 1;
            let keyboard = 2 * gap + 1;
            // title, blank, grid, blank, keyboard, blank, message, footer, [legend]
            2 + grid + 1 + keyboard + 1 + 2 + legend as u16
        };
        let gap = [2, 1].into_iter().find(|&g| height(g) <= rows)?;
        let title_y = (rows - height(gap)) / 2;
        let grid_y = title_y + 2;
        let keyboard_y = grid_y + (max_guesses as u16 - 1) * gap + 2;
        let message_y = keyboard_y + 2 * gap + 2;
        Some(Layout {
            cols,
            title_y,
            grid_y,
            gap,
            keyboard_y,
            message_y,
            footer_y: message_y + 1,
        })
    }

    fn min_size(max_guesses: usize, legend: bool) -> (u16, u16) {
        let rows = 2 + max_guesses as u16 + 1 + 3 + 1 + 2 + legend as u16;
        (MIN_TERMINAL_WIDTH as u16, rows)
    }

    fn centered_x(&self, width: usize) -> u16 {
        (self.cols.saturating_sub(width as u16)) / 2
    }
}

/// Restores the terminal to its normal state
fn restore_terminal() {
    let _ = execute!(
        io::stdout(),
        ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    );
    let _ = terminal::disable_raw_mode();
}

pub struct Ui {
    out: Stdout,
    color: bool,
}

impl Ui {
    /// Switches the terminal to full-screen raw mode. The terminal is restored
    /// when the Ui is dropped, and also if the program panics.
    pub fn new(color: bool) -> io::Result<Ui> {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_terminal();
            default_hook(info);
        }));

        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(Ui { out, color })
    }

    fn read_action(&self) -> io::Result<Action> {
        loop {
            let key = match event::read()? {
                Event::Key(key) => key,
                Event::Resize(_, _) => return Ok(Action::Redraw),
                _ => continue,
            };
            // Windows reports key releases too; only act on presses
            if key.kind == KeyEventKind::Release {
                continue;
            }
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            return Ok(match key.code {
                KeyCode::Char('c') | KeyCode::Char('d') if ctrl => Action::Quit,
                KeyCode::Esc => Action::Quit,
                KeyCode::Enter => Action::Enter,
                KeyCode::Backspace | KeyCode::Delete => Action::Backspace,
                KeyCode::Char(_) if ctrl => Action::Other,
                KeyCode::Char(c) if c.is_ascii_alphabetic() => {
                    Action::Letter(c.to_ascii_lowercase())
                }
                KeyCode::Char(_) => Action::Other,
                _ => continue,
            });
        }
    }

    fn print_at(&mut self, x: u16, y: u16, text: &str, cols: u16) -> io::Result<()> {
        let room = cols.saturating_sub(x) as usize;
        let text: String = text.chars().take(room).collect();
        queue!(self.out, cursor::MoveTo(x, y), Print(text))
    }

    fn draw_tile(&mut self, x: u16, y: u16, ch: char, style: TileStyle) -> io::Result<()> {
        if !self.color {
            // Without colors, brackets carry the meaning (see legend)
            let text = match style {
                TileStyle::Blank => " _ ".to_string(),
                TileStyle::Typed | TileStyle::Key => format!(" {} ", ch.to_ascii_uppercase()),
                TileStyle::Scored(KeyBoardHelperState::GREEN) => {
                    format!("[{}]", ch.to_ascii_uppercase())
                }
                TileStyle::Scored(KeyBoardHelperState::AMBER) => {
                    format!("({})", ch.to_ascii_uppercase())
                }
                TileStyle::Scored(_) => format!(" {} ", ch.to_ascii_lowercase()),
            };
            return queue!(self.out, cursor::MoveTo(x, y), Print(text));
        }

        let (fg, bg) = match style {
            TileStyle::Blank | TileStyle::Key => (Color::Black, Color::Grey),
            TileStyle::Typed => (Color::White, Color::DarkBlue),
            TileStyle::Scored(KeyBoardHelperState::GREEN) => (Color::Black, Color::DarkGreen),
            TileStyle::Scored(KeyBoardHelperState::AMBER) => (Color::Black, Color::DarkYellow),
            TileStyle::Scored(KeyBoardHelperState::GRAY) => (Color::White, Color::DarkGrey),
            TileStyle::Scored(KeyBoardHelperState::NONE) => (Color::Black, Color::Grey),
        };
        queue!(
            self.out,
            cursor::MoveTo(x, y),
            SetForegroundColor(fg),
            SetBackgroundColor(bg),
            Print(format!(" {} ", ch.to_ascii_uppercase())),
            ResetColor
        )
    }

    fn draw_title(&mut self, layout: &Layout) -> io::Result<()> {
        use KeyBoardHelperState::*;
        let pattern = [GREEN, NONE, AMBER, AMBER, GREEN, NONE, GREEN];
        let width = TITLE.len() * CHAR_SPACING as usize - 1;
        if width > layout.cols as usize {
            let x = layout.centered_x(TITLE.len());
            return self.print_at(x, layout.title_y, TITLE, layout.cols);
        }
        let x = layout.centered_x(width);
        for (i, ch) in TITLE.chars().enumerate() {
            let style = if self.color {
                TileStyle::Scored(pattern[i % pattern.len()])
            } else {
                TileStyle::Key
            };
            self.draw_tile(
                x + i as u16 * CHAR_SPACING as u16,
                layout.title_y,
                ch,
                style,
            )?;
        }
        Ok(())
    }

    fn draw_grid(&mut self, layout: &Layout, game: &GameData, input: &str) -> io::Result<()> {
        let width = WORD_LENGTH * CHAR_SPACING as usize - 1;
        let x = layout.centered_x(width);
        let mut guesses = game.guesses();
        for row in 0..game.max_guesses() {
            let y = layout.grid_y + row as u16 * layout.gap;
            let tiles: Vec<(char, TileStyle)> = if let Some((word, scores)) = guesses.next() {
                word.chars()
                    .zip(scores.iter())
                    .map(|(c, &s)| (c, TileStyle::Scored(s)))
                    .collect()
            } else if row == game.guess_count() && game.status() == GameStatus::PLAYING {
                let mut typed: Vec<(char, TileStyle)> =
                    input.chars().map(|c| (c, TileStyle::Typed)).collect();
                typed.resize(WORD_LENGTH, (' ', TileStyle::Blank));
                typed
            } else {
                vec![(' ', TileStyle::Blank); WORD_LENGTH]
            };
            for (i, (ch, style)) in tiles.into_iter().enumerate() {
                self.draw_tile(x + i as u16 * CHAR_SPACING as u16, y, ch, style)?;
            }
        }
        Ok(())
    }

    fn draw_keyboard(&mut self, layout: &Layout, game: &GameData) -> io::Result<()> {
        let helper = game.keyboard_helper();
        for (r, keys) in KEYBOARD_ROWS.iter().enumerate() {
            let y = layout.keyboard_y + r as u16 * layout.gap;
            let x = layout.centered_x(keys.len() * CHAR_SPACING as usize - 1);
            for (i, ch) in keys.chars().enumerate() {
                let style = match helper[ch as usize - 'a' as usize] {
                    KeyBoardHelperState::NONE => TileStyle::Key,
                    state => TileStyle::Scored(state),
                };
                self.draw_tile(x + i as u16 * CHAR_SPACING as u16, y, ch, style)?;
            }
        }
        Ok(())
    }

    fn draw_message(
        &mut self,
        layout: &Layout,
        message: &Option<(String, MessageKind)>,
    ) -> io::Result<()> {
        let Some((text, kind)) = message else {
            return Ok(());
        };
        let text = format!(" {} ", text);
        let x = layout.centered_x(text.len());
        if self.color {
            let (fg, bg) = match kind {
                MessageKind::Info => (Color::Reset, Color::Reset),
                MessageKind::Error => (Color::White, Color::DarkRed),
                MessageKind::Success => (Color::Black, Color::DarkGreen),
            };
            queue!(self.out, SetForegroundColor(fg), SetBackgroundColor(bg))?;
        }
        self.print_at(x, layout.message_y, &text, layout.cols)?;
        queue!(self.out, ResetColor)
    }

    fn draw_too_small(&mut self, max_guesses: usize) -> io::Result<()> {
        let (cols, rows) = terminal::size()?;
        let (min_cols, min_rows) = Layout::min_size(max_guesses, !self.color);
        let text = format!(
            "Terminal too small: {}x{} (need {}x{}). Resize, or press Esc to quit.",
            cols, rows, min_cols, min_rows
        );
        let text: String = text.chars().take(cols as usize).collect();
        queue!(self.out, cursor::MoveTo(0, 0), Print(text))
    }

    fn draw_game(
        &mut self,
        game: &GameData,
        input: &str,
        message: &Option<(String, MessageKind)>,
        footer: &str,
    ) -> io::Result<()> {
        queue!(self.out, ResetColor, terminal::Clear(ClearType::All))?;
        let (cols, rows) = terminal::size()?;
        match Layout::new(cols, rows, game.max_guesses(), !self.color) {
            Some(layout) => {
                self.draw_title(&layout)?;
                self.draw_grid(&layout, game, input)?;
                self.draw_keyboard(&layout, game)?;
                self.draw_message(&layout, message)?;
                let x = layout.centered_x(footer.len());
                self.print_at(x, layout.footer_y, footer, layout.cols)?;
                if !self.color {
                    let legend = "[X] right spot   (X) wrong spot   x not in word";
                    let x = layout.centered_x(legend.len());
                    self.print_at(x, layout.footer_y + 1, legend, layout.cols)?;
                }
            }
            None => self.draw_too_small(game.max_guesses())?,
        }
        self.out.flush()
    }

    fn show_stats(&mut self, stats: &GameStats, max_guesses: usize) -> io::Result<()> {
        loop {
            queue!(self.out, ResetColor, terminal::Clear(ClearType::All))?;
            let (cols, rows) = terminal::size()?;
            let mut lines = format_stats(stats, max_guesses);
            lines.push(String::new());
            lines.push("Press any key to go back".to_string());
            if lines.len() >= rows as usize {
                lines.retain(|l| !l.is_empty());
            }
            let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
            let x = cols.saturating_sub(width as u16) / 2;
            for (i, line) in lines.iter().enumerate() {
                self.print_at(x, 1 + i as u16, line, cols)?;
            }
            self.out.flush()?;
            if !matches!(self.read_action()?, Action::Redraw) {
                return Ok(());
            }
        }
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Plain-text statistics, shared by the stats screen and `--stats`
pub fn format_stats(stats: &GameStats, max_guesses: usize) -> Vec<String> {
    let mut lines = vec![
        "STATISTICS".to_string(),
        String::new(),
        format!("Games played:    {}", stats.games_played),
        format!("Win rate:        {:.0}%", stats.win_percentage()),
        format!("Current streak:  {}", stats.current_streak),
        format!("Max streak:      {}", stats.max_streak),
        format!("Average guesses: {:.1}", stats.average_guesses()),
        String::new(),
        "Guess distribution:".to_string(),
    ];
    let most_guesses = stats.guess_distribution.keys().copied().max().unwrap_or(0) as usize;
    let most_wins = stats
        .guess_distribution
        .values()
        .copied()
        .max()
        .unwrap_or(0);
    for guess in 1..=max_guesses.max(most_guesses) {
        let count = stats
            .guess_distribution
            .get(&(guess as u32))
            .copied()
            .unwrap_or(0);
        let bar_len = if most_wins > 0 {
            (count * 30).div_ceil(most_wins) as usize
        } else {
            0
        };
        lines.push(format!("{:>3} | {} {}", guess, "#".repeat(bar_len), count));
    }
    lines
}

fn playing_footer(game: &GameData) -> String {
    let mut footer = format!("Guess {}/{}", game.guess_count() + 1, game.max_guesses());
    if game.is_hard_mode() {
        footer.push_str(" | HARD MODE");
        if !game.revealed_letters().is_empty() {
            let letters: String = game
                .revealed_letters()
                .iter()
                .map(|c| c.to_ascii_uppercase())
                .collect();
            footer.push_str(&format!(" | Must use: {}", letters));
        }
    }
    footer.push_str(" | Esc: quit");
    footer
}

/// Runs games until the player quits
pub fn run(config: &Config, stats_manager: &mut StatsManager) -> io::Result<()> {
    let mut ui = Ui::new(config.use_color())?;

    loop {
        let mut game = GameData::new(config);
        let mut input = String::new();
        let mut message = None;

        // Play one game
        while game.status() == GameStatus::PLAYING {
            ui.draw_game(&game, &input, &message, &playing_footer(&game))?;
            let action = ui.read_action()?;
            if !matches!(action, Action::Redraw) {
                message = None;
            }
            match action {
                Action::Quit => return Ok(()),
                Action::Letter(c) if input.len() < WORD_LENGTH => input.push(c),
                Action::Backspace => {
                    input.pop();
                }
                Action::Enter if input.len() < WORD_LENGTH => {
                    message = Some(("Not enough letters".to_string(), MessageKind::Error));
                }
                Action::Enter if !words::word_isvalid(&input) => {
                    message = Some(("Not in word list".to_string(), MessageKind::Error));
                }
                Action::Enter => match game.new_guess(&input) {
                    Ok(_) => input.clear(),
                    Err(e) => message = Some((e, MessageKind::Error)),
                },
                _ => {}
            }
        }

        // Game over: record and show result
        let won = game.status() == GameStatus::WON;
        let result = if won {
            let text = match game.guess_count() {
                1 => "Incredible! You got it in 1 guess!".to_string(),
                n => format!("You got it in {} guesses!", n),
            };
            (text, MessageKind::Success)
        } else {
            let word = game.winning_word().to_ascii_uppercase();
            (
                format!("The word was {}. Better luck next time!", word),
                MessageKind::Info,
            )
        };
        let saved = stats_manager.record_game(won, won.then(|| game.guess_count()));
        let message = match saved {
            Ok(()) => Some(result),
            Err(e) => Some((format!("{} (stats not saved: {})", result.0, e), result.1)),
        };

        loop {
            ui.draw_game(&game, "", &message, "R: play again | S: stats | Q: quit")?;
            match ui.read_action()? {
                Action::Letter('r') | Action::Enter => break,
                Action::Letter('s') => {
                    ui.show_stats(stats_manager.get_stats(), config.max_guesses)?
                }
                Action::Letter('q') | Action::Quit => return Ok(()),
                _ => {}
            }
        }
    }
}
