//! Parola: a falling-word typing game rendered with cfonts block characters.

mod game;
mod render;
mod words;

use std::cmp::Ordering;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode, size,
    },
};

use game::Game;
use render::{draw_game_over, draw_hud, draw_restart_prompt, draw_word};

/// Restores the terminal to normal mode when dropped.
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    let _guard = TerminalGuard::new()?;
    let mut stdout = io::stdout();

    let (width, height) = size()?;
    let mut quit = false;

    'outer: while !quit {
        let mut game = Game::new(width, height);
        let mut last = Instant::now();

        while !game.game_over {
            let now = Instant::now();
            let dt = now.duration_since(last).as_secs_f64();
            last = now;

            // Read pending input.
            while event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()?
                    && key.kind == KeyEventKind::Press
                {
                    match key.code {
                        KeyCode::Char(c) => game.type_char(c),
                        KeyCode::Esc => break 'outer,
                        _ => {}
                    }
                }
            }

            game.update(dt);

            // Draw.
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
            draw_hud(
                &mut stdout,
                game.score,
                game.target_len,
                game.current_speed(),
                width,
            )?;
            let target_idx = game
                .words
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.y.partial_cmp(&b.1.y).unwrap_or(Ordering::Equal))
                .map(|(i, _)| i);
            for (i, w) in game.words.iter().enumerate() {
                draw_word(
                    &mut stdout,
                    w.progress,
                    &w.block,
                    w.x,
                    w.y as u16,
                    height,
                    Some(i) == target_idx,
                )?;
            }
            stdout.flush()?;

            // Small sleep to avoid busy-looping.
            std::thread::sleep(Duration::from_millis(16));
        }

        // Game over screen: show the score for 2 seconds, then offer restart.
        execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
        draw_game_over(&mut stdout, game.score, width, height)?;
        stdout.flush()?;
        std::thread::sleep(Duration::from_secs(2));
        draw_restart_prompt(&mut stdout, width, height)?;
        stdout.flush()?;
        loop {
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Char('r') | KeyCode::Char('R') => continue 'outer,
                    KeyCode::Esc => quit = true,
                    _ => {}
                }
                break;
            }
        }
    }

    Ok(())
}
