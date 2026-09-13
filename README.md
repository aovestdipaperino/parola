# Parola

A falling-word typing game for the terminal, rendered with [cfonts](https://crates.io/crates/cfonts) block characters.

Words fall from the top of the screen. Type the letters of the lowest word to clear it before it reaches the bottom. Words change color from cold blue to hot red as they approach the bottom, and correctly typed letters turn green.

## Features

- Words rendered as semi-graphic block characters using the cfonts **tiny** font.
- Words fall periodically; typing always targets the **lowest** word.
- Words interpolate from **blue (cold)** to **red (hot)** as they approach the bottom.
- Correct letters turn **green**; the next letter of the target word is highlighted in **yellow**.
- Score: each correct letter adds `word_length × 1`, plus a `word_length × 10` completion bonus.
- Difficulty ramps up: words start at length 3, the average length grows over time, and fall speed increases once the average reaches 8.
- Game over when any word reaches the bottom; the game-over screen shows for 2 seconds, then offers restart.
- HUD showing score, average length, and fall speed.

## Requirements

- Rust (edition 2024) and Cargo.

## Run

```sh
cargo run
```

## Controls

| Key | Action |
|-----|--------|
| `a`–`z` | Type the next letter of the lowest word |
| `Esc` | Quit |
| `R` (after game over) | Restart |

## Project layout

- `src/main.rs` — terminal setup and the game loop.
- `src/game.rs` — game state, update, input, scoring, and difficulty.
- `src/render.rs` — cfonts block composition, color interpolation, and drawing.
- `src/words.rs` — embedded word list and length selection.

## Tests

```sh
cargo test
```

## License

MIT — see [LICENSE](LICENSE).
