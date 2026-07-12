# pik

![demo](assets/demo.gif)

Minimal interactive line picker for the command line.

[![CI](https://img.shields.io/github/actions/workflow/status/programmersd21/pik/ci.yml?branch=main&style=for-the-badge&logo=github&logoColor=white&label=CI&labelColor=000000&color=2EA043)](https://github.com/programmersd21/pik/actions)
[![Stars](https://img.shields.io/github/stars/programmersd21/pik?style=for-the-badge&logo=github&logoColor=white&label=Stars&labelColor=000000&color=F5A623)](https://github.com/programmersd21/pik/stargazers)
[![License: MIT](https://img.shields.io/badge/License-MIT-585DEC?style=for-the-badge&logo=opensourceinitiative&logoColor=white&labelColor=000000)](LICENSE)
[![Made with Rust](https://img.shields.io/badge/Made%20with-Rust-F74C00?style=for-the-badge&logo=rust&logoColor=white&labelColor=000000)](https://www.rust-lang.org/)
[![AUR](https://img.shields.io/badge/AUR-pik--bin-1793D1?style=for-the-badge&logo=archlinux&logoColor=white&labelColor=000000)](https://aur.archlinux.org/packages/pik-bin)

Read newline-separated choices from stdin or a file and select one interactively.

## Install

```bash
cargo install --path .
```

## Usage

```bash
pik --file list.txt --prompt "Select:"
cat list.txt | pik
git branch | pik | xargs git checkout
```

## Options

| Flag | Description |
|------|-------------|
| `-p, --prompt <text>` | Header text |
| `-f, --file <path>` | Read from file |
| `-V, --version` | Show version |
| `-h, --help` | Show help |

## Keybindings

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Home` / `g` | Jump to first |
| `End` / `G` | Jump to last |
| `PageUp` / `Ctrl+u` | Page up |
| `PageDown` / `Ctrl+d` | Page down |
| `Enter` | Confirm |
| `Esc` / `q` / `Ctrl+c` | Cancel |
| `Click` | Move cursor |
| `Double-click` | Confirm |

## Mouse

Mouse capture is enabled automatically.

- Click a row to move the cursor to it, press `Enter` to confirm, same as keyboard navigation.
- Scroll wheel moves the cursor up or down one row at a time.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Selection made |
| 1 | Error |
| 130 | Cancelled |

## License

MIT
