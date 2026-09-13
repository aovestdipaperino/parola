//! Rendering: build cfonts letter blocks, compose words, and draw them with color.

use std::io::{self, Write};

use cfonts::{BgColors, Env, Fonts, Options, render};
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};

/// Number of lines in the FontTiny font.
pub const LETTER_LINES: usize = 2;

/// A pre-rendered word block: the composed lines plus per-letter column ranges.
pub struct WordBlock {
    /// One string per row; each row is the full word block.
    pub lines: Vec<String>,
    /// Per-letter `(start, end)` byte ranges into each line. The range for a
    /// non-final letter includes the trailing separator space.
    pub ranges: Vec<(usize, usize)>,
    /// Total width of the block in columns.
    pub width: usize,
}

/// Render a single letter with cfonts (FontTiny, transparent background) and
/// return its plain-text lines. Buffer lines are stripped.
fn letter_lines(c: char) -> Vec<String> {
    let out = render(Options {
        text: c.to_string(),
        font: Fonts::FontTiny,
        background: BgColors::Transparent,
        env: Env::Cli,
        ..Options::default()
    });
    out.vec
        .join("\n")
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Build a word block by rendering each letter separately and composing them
/// side by side with a single space between letters.
pub fn render_word_block(word: &str) -> WordBlock {
    let mut letters: Vec<Vec<String>> = Vec::new();
    let mut widths: Vec<usize> = Vec::new();

    for c in word.chars() {
        let lines = letter_lines(c);
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let padded: Vec<String> = lines
            .iter()
            .map(|l| {
                let mut s = l.to_string();
                while s.chars().count() < width {
                    s.push(' ');
                }
                s
            })
            .collect();
        letters.push(padded);
        widths.push(width);
    }

    let max_rows = letters
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(LETTER_LINES);
    let mut lines: Vec<String> = vec![String::new(); max_rows];
    let mut ranges = Vec::new();
    let mut col = 0usize;

    for (i, letter) in letters.iter().enumerate() {
        let start = col;
        for (row, line_out) in lines.iter_mut().enumerate() {
            if let Some(line) = letter.get(row) {
                line_out.push_str(line);
            } else {
                for _ in 0..widths[i] {
                    line_out.push(' ');
                }
            }
        }
        col += widths[i];
        let end = col;
        if i + 1 < letters.len() {
            for line_out in lines.iter_mut() {
                line_out.push(' ');
            }
            col += 1;
            ranges.push((start, end + 1));
        } else {
            ranges.push((start, end));
        }
    }

    WordBlock {
        lines,
        ranges,
        width: col,
    }
}

/// Interpolate the "heat" color: cold blue at the top, hot red near the bottom.
/// `t` is clamped to `[0, 1]`.
pub fn heat_color(t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let r = (0.0 + t * 255.0).round() as u8;
    let g = (100.0 + t * (40.0 - 100.0)).round() as u8;
    let b = (255.0 + t * (40.0 - 255.0)).round() as u8;
    Color::Rgb { r, g, b }
}

/// Draw a word block at `(x, y)`.
///
/// - Letters already typed correctly are green.
/// - Only the target (lowest) word highlights its next letter in bold yellow.
/// - Remaining letters use the heat color based on the word's vertical position.
pub fn draw_word<W: Write>(
    out: &mut W,
    progress: usize,
    block: &WordBlock,
    x: u16,
    y: u16,
    height: u16,
    is_target: bool,
) -> io::Result<()> {
    let t = ((y as f64 + block.lines.len() as f64) / height.max(1) as f64).clamp(0.0, 1.0);
    let heat = heat_color(t);

    for (row, line) in block.lines.iter().enumerate() {
        let row_y = y + row as u16;
        execute!(out, MoveTo(x, row_y))?;
        for (i, (start, end)) in block.ranges.iter().enumerate() {
            let color = if i < progress {
                Color::Green
            } else if is_target && i == progress {
                Color::Yellow
            } else {
                heat
            };
            // Ranges are character indices; the tiny font uses multi-byte
            // Unicode block characters, so convert to byte indices before
            // slicing the string.
            let byte_start = line
                .char_indices()
                .nth(*start)
                .map(|(b, _)| b)
                .unwrap_or(line.len());
            let byte_end = line
                .char_indices()
                .nth(*end)
                .map(|(b, _)| b)
                .unwrap_or(line.len());
            let seg = &line[byte_start..byte_end];
            if is_target && i == progress {
                execute!(
                    out,
                    SetAttribute(Attribute::Bold),
                    SetForegroundColor(color),
                    Print(seg)
                )?;
            } else {
                execute!(out, SetForegroundColor(color), Print(seg))?;
            }
        }
    }
    execute!(
        out,
        SetAttribute(Attribute::Reset),
        SetForegroundColor(Color::Reset)
    )?;
    Ok(())
}

/// Draw the HUD line at the top of the screen.
pub fn draw_hud<W: Write>(
    out: &mut W,
    score: u64,
    target_len: usize,
    speed: f64,
    width: u16,
) -> io::Result<()> {
    let text = format!(
        "Score: {score}   Avg length: {target_len}   Speed: {speed:.1} rows/s   (Esc to quit)"
    );
    let len = text.chars().count() as u16;
    execute!(
        out,
        MoveTo(0, 0),
        SetForegroundColor(Color::White),
        Print(text)
    )?;
    // Clear the rest of the HUD row so stale text does not linger.
    if len < width {
        execute!(
            out,
            MoveTo(len, 0),
            Print(" ".repeat((width - len) as usize))
        )?;
    }
    execute!(out, SetForegroundColor(Color::Reset))?;
    Ok(())
}

/// Draw the game-over screen: "GAMEOVER" in the tiny font, centered, white,
/// with the final score below it.
pub fn draw_game_over<W: Write>(
    out: &mut W,
    score: u64,
    width: u16,
    height: u16,
) -> io::Result<()> {
    let block = render_word_block("GAMEOVER");
    let x = width.saturating_sub(block.width as u16) / 2;
    let y = height.saturating_sub(block.lines.len() as u16) / 2;
    for (row, line) in block.lines.iter().enumerate() {
        let row_y = y + row as u16;
        execute!(
            out,
            MoveTo(x, row_y),
            SetForegroundColor(Color::White),
            Print(line)
        )?;
    }
    let score_msg = format!("Final score: {score}");
    let sx = width.saturating_sub(score_msg.chars().count() as u16) / 2;
    let sy = y + block.lines.len() as u16 + 1;
    execute!(
        out,
        MoveTo(sx, sy),
        SetForegroundColor(Color::White),
        Print(score_msg)
    )?;
    execute!(out, SetForegroundColor(Color::Reset))?;
    Ok(())
}

/// Draw the restart prompt below the game-over message.
pub fn draw_restart_prompt<W: Write>(out: &mut W, width: u16, height: u16) -> io::Result<()> {
    let prompt = "Press R to restart, Esc to quit";
    let px = width.saturating_sub(prompt.chars().count() as u16) / 2;
    let y = height / 2 + 1;
    execute!(
        out,
        MoveTo(px, y),
        SetForegroundColor(Color::White),
        Print(prompt)
    )?;
    execute!(out, SetForegroundColor(Color::Reset))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_color_interpolates_blue_to_red() {
        let cold = heat_color(0.0);
        let hot = heat_color(1.0);
        let mid = heat_color(0.5);
        match cold {
            Color::Rgb { r, g, b } => {
                assert_eq!(r, 0);
                assert_eq!(g, 100);
                assert_eq!(b, 255);
            }
            _ => panic!("expected rgb"),
        }
        match hot {
            Color::Rgb { r, g, b } => {
                assert_eq!(r, 255);
                assert_eq!(g, 40);
                assert_eq!(b, 40);
            }
            _ => panic!("expected rgb"),
        }
        match mid {
            Color::Rgb { r, g, b } => {
                assert_eq!(r, 128);
                assert_eq!(g, 70);
                assert_eq!(b, 148);
            }
            _ => panic!("expected rgb"),
        }
    }

    #[test]
    fn heat_color_clamps_out_of_range() {
        assert_eq!(heat_color(-1.0), heat_color(0.0));
        assert_eq!(heat_color(2.0), heat_color(1.0));
    }

    #[test]
    fn word_block_has_one_range_per_letter() {
        let block = render_word_block("cat");
        assert_eq!(block.ranges.len(), 3);
        assert_eq!(block.lines.len(), LETTER_LINES);
        assert!(block.width > 0);
        // Ranges are ordered and non-overlapping.
        for pair in block.ranges.windows(2) {
            assert!(pair[0].1 <= pair[1].0);
        }
    }

    #[test]
    fn word_block_lines_are_consistent_width() {
        let block = render_word_block("hello");
        for line in &block.lines {
            assert_eq!(line.chars().count(), block.width);
        }
    }

    #[test]
    fn draw_word_handles_multibyte_block_chars() {
        // The tiny font uses multi-byte Unicode block characters; drawing must
        // not panic when slicing per-letter ranges.
        let block = render_word_block("cat");
        let mut buf = Vec::new();
        draw_word(&mut buf, 0, &block, 0, 0, 40, true).unwrap();
        assert!(!buf.is_empty());
    }
}
