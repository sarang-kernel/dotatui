# `dotatui` - A Dotfile-Focused Git TUI

**dotatui** is a fast, terminal-based UI for managing git repositories, written in Rust. Inspired by the convenience of `lazygit` and the aesthetics of `lazyvim`, its primary focus is to provide a streamlined workflow for managing system dotfiles and configurations.

[![Project Status: Active](https://www.repostatus.org/badges/latest/active.svg)](https://www.repostatus.org/#active)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

![dotatui screenshot placeholder](https://user-images.githubusercontent.com/563232/232049953-2940742a-2895-4a6f-8700-1c05f778396c.png)
_(A real screenshot of the application would go here)_

## ✨ Features

- **Fast and Responsive:** Built in Rust for native performance.
- **Two-Panel Status View:** Clear separation of staged and unstaged changes.
- **Diff Viewer:** Instantly view diffs for any selected file.
- **Intuitive Staging:** Stage/unstage entire files, including deletions, with a single key press.
- **Commit Workflow:** Simple popup for writing and submitting commit messages.
- **Commit Log:** Browse your repository's commit history.
- **Asynchronous Git Operations:** Push to your remote without blocking the UI.
- **Vim-Style Navigation:**
  - Use `j`/`k` to navigate lists.
  - Use `h`/`l` to switch between panels.
- **Mouse Support:** Click to select files and change panel focus, and use the scroll wheel to navigate lists.

## 🚀 Installation

### Prerequisites

You need the Rust toolchain and the `libgit2` development library installed on your system.

**1. Install Rust:**
If you don't have Rust, install it via `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**2. Install `libgit2`:**

- **Debian / Ubuntu:**
  ```bash
  sudo apt-get install -y pkg-config libssl-dev libgit2-dev
  ```
- **Arch Linux:**
  ```bash
  sudo pacman -S pkg-config openssl libgit2
  ```
- **macOS (Homebrew):**
  ```bash
  brew install libgit2
  ```
- **Windows:** Installation is more complex. Please follow the instructions on the [`git2-rs` repository](https://github.com/rust-lang/git2-rs#windows).

### From Source (Current Method)

1.  Clone the repository:

    ```bash
    git clone https://github.com/your-username/dotatui.git
    cd dotatui
    ```

2.  Build the release binary:

    ```bash
    cargo build --release
    ```

3.  The executable will be located at `./target/release/dotatui`. For easy access, move it to a directory in your system's `PATH`:
    ```bash
    sudo mv ./target/release/dotatui /usr/local/bin/
    ```

### From Crates.io (Coming Soon)

Once published, installation will be as simple as:

```bash
cargo install dotatui
```

## ⌨️ Usage & Keybindings

Launch the application from within any git repository:

```bash
dotatui
```

### Global

| Key | Action                |
| --- | --------------------- |
| `q` | Quit the application  |
| `s` | Switch to Status View |
| `l` | Switch to Log View    |
| `?` | Show Help Popup       |

### Status View

| Key                   | Action                                            |
| --------------------- | ------------------------------------------------- |
| `h` / `l`             | Switch focus between Files and Diff panels        |
| `j` / `k` / `↓` / `↑` | Navigate the list in the active panel             |
| `space`               | Stage the selected unstaged item                  |
| `u`                   | Unstage the selected staged item                  |
| `enter`               | Enter "Hunk Selection" mode for the selected file |
| `c`                   | Open commit message popup                         |
| `Shift`+`P`           | Push to `origin`                                  |

### Hunk Selection Mode (In Progress)

| Key       | Action                        |
| --------- | ----------------------------- |
| `j` / `k` | Navigate hunks                |
| `q`       | Return to File Selection mode |

### Popups

| Key     | Action                               |
| ------- | ------------------------------------ |
| `esc`   | Close any popup (Help, Commit, etc.) |
| `enter` | Confirm action (e.g., submit commit) |

### Mouse

| Action           | Effect                                   |
| ---------------- | ---------------------------------------- |
| **Click**        | Select a file or change the active panel |
| **Scroll Wheel** | Navigate up/down in the Files list       |

## 🗺️ Roadmap (V2 and Beyond)

`dotatui` is actively being developed. Here are the next major features planned:

- [ ] **Full Interactive Staging:**
  - [x] Parse diffs into hunks.
  - [ ] Stage/unstage individual hunks.
  - [ ] Stage/unstage individual lines.
- [ ] **Branch Management:**
  - [ ] List, create, and switch branches from a popup.
  - [ ] Delete local branches.
- [ ] **Remote Operations:**
  - [ ] Fetch from remotes.
  - [ ] Pull (fetch + merge).
- [ ] **Interactive Log:**
  - [ ] View the full diff for any commit in the log.
  - [ ] Checkout a specific commit.
- [ ] **Configuration File:**
  - [ ] Allow user-defined keybindings and colors via a config file.

## 🤝 Contributing

Contributions are welcome! Feel free to open an issue to report a bug or suggest a feature, or open a pull request to contribute code.

### Development

- To run in debug mode with logging: `cargo run`
- To check the logs: `tail -f dotatui.log`
- To run tests: `cargo test`

## ⚖️ License

This project is licensed under the **MIT License**. See the `LICENSE` file for details.
