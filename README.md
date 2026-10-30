# letterlock

_Formerly `cwordle`._

A free five-letter word-guessing game for the terminal. Guess the hidden word in
six tries; after each guess the tiles show which letters are in the right spot,
which are in the word but in the wrong spot, and which aren't in the word at all.

Runs natively on Linux, macOS and Windows (Windows Terminal, PowerShell, cmd) with
no system libraries required.

## Install

From source with Rust (any platform):

```sh
cargo install letterlock
```

Prebuilt binaries for Linux, macOS and Windows (x86_64 and ARM64) are attached to each
[GitHub release](https://github.com/pravin/cwordle/releases).

## Play

```sh
letterlock              # new game
letterlock --hard-mode  # revealed hints must be used in later guesses
letterlock --stats      # print your statistics
letterlock --help       # all options
```

| Key | Action |
| --- | --- |
| `a`–`z` | Type a letter |
| `Backspace` | Delete a letter |
| `Enter` | Submit guess |
| `Esc` / `Ctrl+C` | Quit |

After a game: `R` to play again, `S` for statistics, `Q` to quit.

**Hard mode:** letters in the right spot must stay there, and letters found in the
wrong spot must be used in every later guess.

**No colour:** run with `--no-color` or set `NO_COLOR=1`. Tiles are then marked
`[X]` (right spot), `(X)` (wrong spot) and lowercase `x` (not in the word).

Statistics are saved to:

- Linux: `~/.local/share/letterlock/stats.json` (respects `$XDG_DATA_HOME`)
- macOS: `~/Library/Application Support/letterlock/stats.json`
- Windows: `%APPDATA%\letterlock\stats.json`

## Screenshots
![Starting screen: letterlock](img/1.png)
Starting screen

![Image](img/2.png)
Partway into a game

![Image](img/3.png)
Winning the game

![Image](img/4.png)
Entering a word that isn't in the word list

![Image](img/5.png)
Losing the game

## Development

```sh
cargo test
cargo run -- --word crane   # play with a fixed answer
```

Releases are built by `.github/workflows/release.yml` when a `v*` tag is pushed.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
