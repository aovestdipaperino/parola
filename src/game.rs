//! Game state, update loop, input handling, scoring, and difficulty.

use rand::RngExt;

use crate::render::{WordBlock, render_word_block};
use crate::words;

/// Rows reserved for the HUD at the top of the screen.
pub const HUD_ROWS: f64 = 2.0;
/// Minimum word length.
pub const MIN_LEN: usize = 3;
/// Maximum word length.
pub const MAX_LEN: usize = 14;
/// Completed words needed to raise the average length by one.
pub const WORDS_PER_LEVEL: usize = 5;
/// Base fall speed in rows per second.
pub const BASE_SPEED: f64 = 1.0;
/// Extra speed per average-length step above the minimum.
pub const SPEED_PER_LEN: f64 = 0.25;
/// Speed multiplier applied once the average length reaches 8.
pub const SPEED_BOOST_AT_8: f64 = 1.5;
/// Base spawn interval in seconds.
pub const BASE_SPAWN: f64 = 2.0;
/// Minimum spawn interval in seconds.
pub const MIN_SPAWN: f64 = 0.6;

/// A falling word.
pub struct Word {
    pub text: String,
    pub x: u16,
    /// Top row of the block (fractional for smooth movement).
    pub y: f64,
    /// Number of letters typed correctly so far.
    pub progress: usize,
    pub block: WordBlock,
}

/// The game state.
pub struct Game {
    pub words: Vec<Word>,
    pub score: u64,
    /// Target average word length; grows as words are completed.
    pub target_len: usize,
    pub completed: usize,
    pub spawn_timer: f64,
    pub game_over: bool,
    pub width: u16,
    pub height: u16,
}

impl Game {
    pub fn new(width: u16, height: u16) -> Self {
        Game {
            words: Vec::new(),
            score: 0,
            target_len: MIN_LEN,
            completed: 0,
            spawn_timer: 0.0,
            game_over: false,
            width,
            height,
        }
    }

    /// Current fall speed in rows per second.
    pub fn current_speed(&self) -> f64 {
        let base = BASE_SPEED + (self.target_len as f64 - MIN_LEN as f64) * SPEED_PER_LEN;
        if self.target_len >= 8 {
            base * SPEED_BOOST_AT_8
        } else {
            base
        }
    }

    /// Current spawn interval in seconds.
    pub fn spawn_interval(&self) -> f64 {
        (BASE_SPAWN - (self.target_len as f64 - MIN_LEN as f64) * 0.1).max(MIN_SPAWN)
    }

    /// Pick a word length around the current target.
    fn pick_len(&self) -> usize {
        let mut rng = rand::rng();
        let delta: i32 = rng.random_range(-1..=1);
        (self.target_len as i32 + delta).clamp(MIN_LEN as i32, MAX_LEN as i32) as usize
    }

    /// Spawn a new word at the top if there is room.
    fn spawn_word(&mut self) {
        let len = self.pick_len();
        let Some(word) = words::random_word(len, len) else {
            return;
        };
        let block = render_word_block(word);
        let word_width = block.width;
        if word_width + 2 > self.width as usize {
            return;
        }
        let max_x = self.width as usize - word_width;
        let mut rng = rand::rng();
        for _ in 0..20 {
            let x = rng.random_range(0..=max_x);
            let x_start = x;
            let x_end = x + word_width;
            let ok = self.words.iter().all(|w| {
                let wx = w.x as usize;
                let ww = w.block.width;
                let overlap = x_start < wx + ww && wx < x_end;
                let near_top = w.y < (HUD_ROWS + w.block.lines.len() as f64 + 2.0);
                !(overlap && near_top)
            });
            if ok {
                self.words.push(Word {
                    text: word.to_string(),
                    x: x as u16,
                    y: HUD_ROWS,
                    progress: 0,
                    block,
                });
                return;
            }
        }
    }

    /// Advance the game by `dt` seconds.
    pub fn update(&mut self, dt: f64) {
        if self.game_over {
            return;
        }
        let speed = self.current_speed();
        for w in &mut self.words {
            w.y += speed * dt;
        }

        // Game over when any word's bottom reaches the last row.
        for w in &self.words {
            let bottom = w.y + w.block.lines.len() as f64;
            if bottom >= self.height.saturating_sub(1) as f64 {
                self.game_over = true;
                return;
            }
        }

        self.spawn_timer += dt;
        if self.spawn_timer >= self.spawn_interval() {
            self.spawn_timer = 0.0;
            self.spawn_word();
        }
    }

    /// Handle a typed character. Typing applies to the lowest word.
    pub fn type_char(&mut self, c: char) {
        if self.game_over {
            return;
        }
        let Some(idx) = self
            .words
            .iter()
            .enumerate()
            .max_by(|a, b| {
                a.1.y
                    .partial_cmp(&b.1.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
        else {
            return;
        };

        let len = self.words[idx].text.chars().count();
        let next = self.words[idx].text.chars().nth(self.words[idx].progress);
        if next == Some(c) {
            // Per correct letter: word_length * 1.
            self.score += len as u64;
            self.words[idx].progress += 1;
            if self.words[idx].progress == len {
                // Completion bonus: word_length * 10.
                self.score += len as u64 * 10;
                self.words.remove(idx);
                self.completed += 1;
                if self.completed.is_multiple_of(WORDS_PER_LEVEL) {
                    self.target_len = (self.target_len + 1).min(MAX_LEN);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        Game::new(120, 40)
    }

    #[test]
    fn speed_starts_low_and_boosts_at_8() {
        let g = game();
        assert_eq!(g.target_len, MIN_LEN);
        let early = g.current_speed();
        assert!((early - BASE_SPEED).abs() < 1e-9);

        let mut g8 = game();
        g8.target_len = 8;
        let at8 = g8.current_speed();
        let expected = (BASE_SPEED + (8.0 - MIN_LEN as f64) * SPEED_PER_LEN) * SPEED_BOOST_AT_8;
        assert!((at8 - expected).abs() < 1e-9);
    }

    #[test]
    fn spawn_interval_shrinks_with_difficulty() {
        let g = game();
        let early = g.spawn_interval();
        let mut g8 = game();
        g8.target_len = 8;
        let late = g8.spawn_interval();
        assert!(late < early);
        assert!(late >= MIN_SPAWN);
    }

    #[test]
    fn typing_correct_letter_scores_and_progresses() {
        let mut g = game();
        g.words.push(Word {
            text: "cat".to_string(),
            x: 0,
            y: 10.0,
            progress: 0,
            block: render_word_block("cat"),
        });
        g.type_char('c');
        assert_eq!(g.score, 3);
        assert_eq!(g.words[0].progress, 1);
    }

    #[test]
    fn wrong_letter_is_ignored() {
        let mut g = game();
        g.words.push(Word {
            text: "cat".to_string(),
            x: 0,
            y: 10.0,
            progress: 0,
            block: render_word_block("cat"),
        });
        g.type_char('x');
        assert_eq!(g.score, 0);
        assert_eq!(g.words[0].progress, 0);
    }

    #[test]
    fn completing_word_adds_bonus_and_removes() {
        let mut g = game();
        g.words.push(Word {
            text: "cat".to_string(),
            x: 0,
            y: 10.0,
            progress: 0,
            block: render_word_block("cat"),
        });
        g.type_char('c');
        g.type_char('a');
        g.type_char('t');
        assert_eq!(g.score, 3 + 3 + 3 + 30);
        assert!(g.words.is_empty());
        assert_eq!(g.completed, 1);
    }

    #[test]
    fn typing_targets_lowest_word() {
        let mut g = game();
        g.words.push(Word {
            text: "cat".to_string(),
            x: 0,
            y: 5.0,
            progress: 0,
            block: render_word_block("cat"),
        });
        g.words.push(Word {
            text: "dog".to_string(),
            x: 0,
            y: 20.0,
            progress: 0,
            block: render_word_block("dog"),
        });
        g.type_char('d');
        assert_eq!(g.words[1].progress, 1);
        assert_eq!(g.words[0].progress, 0);
    }

    #[test]
    fn word_reaching_bottom_ends_game() {
        let mut g = game();
        let block = render_word_block("cat");
        g.words.push(Word {
            text: "cat".to_string(),
            x: 0,
            y: g.height as f64 - block.lines.len() as f64 - 1.0,
            progress: 0,
            block,
        });
        g.update(0.1);
        assert!(g.game_over);
    }

    #[test]
    fn target_len_grows_after_level() {
        let mut g = game();
        for _ in 0..WORDS_PER_LEVEL {
            g.completed += 1;
            if g.completed.is_multiple_of(WORDS_PER_LEVEL) {
                g.target_len = (g.target_len + 1).min(MAX_LEN);
            }
        }
        assert_eq!(g.target_len, MIN_LEN + 1);
    }
}
