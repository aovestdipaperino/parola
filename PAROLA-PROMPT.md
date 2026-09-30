# PAROLA-PROMPT.md

Build a terminal typing game called **Parola** in Rust, exactly as specified below. Follow every detail; do not add features, change constants, or deviate from the described behavior.

## Overview

Words fall from the top of the terminal screen. The player types the letters of the **lowest** word (the one closest to the bottom) to clear it before it reaches the bottom. Words are rendered as semi-graphic block characters using the `cfonts` crate's **tiny** font. Words change color from cold blue to hot red as they approach the bottom; correctly typed letters turn green.

## Tech stack

- Rust (edition 2024), Cargo.
- `cfonts = "1.3"` — renders text as block-character fonts.
- `crossterm = "0.29"` — raw terminal input, terminal size, colored output, alternate screen.
- `rand = "0.10"` — random word selection and placement.

## Rendering

- Use `cfonts::render` with `Fonts::FontTiny`, `background: BgColors::Transparent`, `env: Env::Cli`, and default options otherwise.
- Render **each letter of a word separately** with cfonts, then compose the letter blocks side by side with a single space between letters. This allows per-letter coloring.
- To extract a letter's plain-text lines from the cfonts output: join `output.vec` with `\n`, split on `\n`, and drop empty strings. The tiny font produces **2 lines** per letter.
- For each letter, compute its width as the maximum character count across its lines, pad every line to that width, and record the letter's column range. The range for a non-final letter includes the trailing separator space.
- Compose the word block into `Vec<String>` lines (one per row) plus a list of per-letter `(start, end)` character-index ranges and the total block width.
- **Important:** the tiny font uses multi-byte Unicode block characters (`█`, `▀`, `▄` are 3 bytes each). When slicing a line for a letter, convert the character-index range to byte indices via `line.char_indices()` before slicing; never slice by character indices directly (this panics).

## Colors

- Letters already typed correctly: **green**.
- The next letter to type, but **only on the target (lowest) word**: **bold yellow**.
- All other letters: interpolate from **blue (cold)** to **red (hot)** based on the word's vertical position. Cold = RGB(0, 100, 255), hot = RGB(255, 40, 40). Compute `t = clamp((word_y + block_height) / terminal_height, 0, 1)` and lerp each channel.

## Game mechanics

- Words spawn at the top (row 2, below the HUD) at intervals and fall at a speed in rows per second.
- Typing always applies to the **lowest** word (the one with the greatest y position).
- A correct letter advances progress by 1 and adds `word_length × 1` to the score.
- Wrong letters are **ignored** (no penalty, no reset).
- When a word is fully typed, it is removed and the score gains a completion bonus of `word_length × 10`.
- When any word's bottom edge reaches the last row (`word_y + block_height >= terminal_height - 1`), the game ends immediately.

## Difficulty

- Words start at length **3**; the average length target grows by 1 every **5** completed words, capped at **14**.
- Word length is chosen as `target_len ± 1`, clamped to `[3, 14]`.
- Fall speed: `base = 1.0 + (target_len - 3) × 0.25` rows/sec. When `target_len >= 8`, multiply by **1.5**.
- Spawn interval: `max(0.6, 2.0 - (target_len - 3) × 0.1)` seconds.

## Placement

- Words fall in fixed horizontal lanes. When spawning, pick a random x position within `[0, terminal_width - word_width]`; reject positions that horizontally overlap an existing word that is still near the top (to avoid vertical overlap). Try up to 20 random positions; if none fit, skip spawning this tick.

## HUD

- Top row shows: `Score: {score}   Avg length: {target_len}   Speed: {speed:.1} rows/s   (Esc to quit)` in white.

## Game over flow

- When the game ends, clear the screen and show **"GAMEOVER"** rendered in the tiny font, centered horizontally and vertically, in **white**, with `Final score: {score}` centered below it.
- The game-over screen stays for **2 seconds**.
- Then show `Press R to restart, Esc to quit` centered below the score.
- Pressing **R** restarts a fresh game (score, words, and difficulty reset). Pressing **Esc** quits.

## Controls

- `a`–`z`: type the next letter of the lowest word.
- `Esc`: quit.
- `R` (after game over): restart.

## Word list

- Embed a curated list of common lowercase English words covering lengths **3 through 14**. Provide a function that picks a random word of a given length (or within a length range). Ensure every length 3–14 has at least one word.

## File structure

- `src/main.rs` — terminal setup (raw mode, alternate screen, hide cursor, restore on drop) and the game loop.
- `src/game.rs` — game state, update, input, scoring, difficulty, spawning.
- `src/render.rs` — cfonts letter rendering, word-block composition, color interpolation, drawing (words, HUD, game over, restart prompt).
- `src/words.rs` — embedded word list and length selection.
- `README.md` — project description, features, run instructions, controls, layout, tests, license.
- `LICENSE` — MIT License.

## Tests

Include unit tests covering at least: color interpolation (blue→red, clamping), word-block composition (one range per letter, consistent line width), scoring (correct letter, wrong letter ignored, completion bonus), typing targets the lowest word, game over on bottom hit, difficulty progression (speed boost at 8, spawn interval shrink, target length growth), and word-length bounds. All tests must pass.

## License

MIT License, copyright `aovestdipaperino`.
