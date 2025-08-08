# `dotatui`: A Nimble Git Tui in Rust

<p align="center">
<img src="https://raw.githubusercontent.com/rust-lang/rust-artwork/master/logo/rust-logo-512x512.png" width="150" alt="Rust Logo">
</p>
<p align="center">
A terminal-based Git client written in Rust, inspired by `lazygit` and the UI philosophy of `lazyvim`.
</p>
<p align="center">
<img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT">
<img src="https://img.shields.io/badge/rust-1.75%2B-orange.svg" alt="Rust Version">
<img src="https://img.shields.io/badge/build-passing-brightgreen" alt="Build Status">
</p>

---

<!-- Replace this block with an actual project GIF-->

```
+------------------------------------------------------------------+
|                                                                  |
|         [A screenshot or GIF of dotatui in action]               |
|         (Showing panel navigation, staging, and committing)      |
|                                                                  |
+------------------------------------------------------------------+
```

## Table of Contents

- [About the Project](#about-the-project)
- [Key Features](#key-features)
- [Installtion](#installation)
- [Usage & Keybindings](#usage--keybindings)
- [Technical Deep Dive](#technical-deep-dive)
  - [Core Technologies](#core-technologies)
  - [Architectural Overview](#architectural-overview)
- [Development](#development)
- [Roadmap](#roadmap)
- [License](#license)
- [Acknowledgments](#acknowledgments)

## About The Project

Dotatui is a terminal user interface (TUI) for Git, built from the ground up in idiomatic Rust. It was created to provide a fast, keyboard-centric, and highly responsive alternative to traditional command-line Git or heavier GUI clients.

The primary design goal is to streamline the management of dotfiles and system configurations, providing an intuitive way to review, stage, and commit changes directly from the terminal without context switching.

## Key Features

- **Comprehensive Status View:** See staged and unstaged changes in a clear, dual-panel layout.
- **Seamless Staging:** Stage and unstage entire files with a singel keypress.
- **Interactive Hunk Mode:** Enter a hunk selection mode to prepare for line-by-line staging(V2 feature in progress)
- **In-App Committing:** A popup interface allows you to write and submit commit messages without leaving the application.
- **Commit History:** Browse the commit log in a clean, tabular format.
- **Asynchronous Remotes:** Push changes to your remote repository without freezing the UI.
- **Modern TUI Experience:**
  - **Full Mouse Support:** Click to select files and change panel focus, scroll to navigate lists.
  - **Vim-Style Navigation:** Use `h`/`l` to switch between the Files and Diff panels, and `j`/`k` for list navigation.
  - **Visual Feedback**: The active panel is clearly highlighted.

## Installation

### Prerequisites

- Rust and Cargo (latest stable version recommended)
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- A C compiler (like `gcc` or `clang`) for building `libgit2` dependencies
- `pkg-config`, `libssl-dev` (or `openssl-devel`) may be required on some systems.

### Build From Source

1. **Clone the repository:**
   ```sh
   git clone https://github.com/sarang-kernel/dotatui.git
   cd dotatui
   ```
2. **Build the optimized release binary:**
   ```sh
   cargo build --release
   ```
3. **Run the application:**
   The binary will be located at `target/release/dotatui`. You can run it from withing any Git repository on your system.

   ```sh
   # Example:
   /path/to/dotatui/target/release/dotatuij

   # For convenience, copy it to a location in your PATH:
   sudo cp target/release/dotatui /usr/local/bin/
   ```

## Usage & Keybindings

Launch `dotatui` from within any directory that is part of a Git repository

| Key(s)               | Action                               | Context             |
| -------------------- | ------------------------------------ | ------------------- |
| `q`                  | Quit application or exit hunk-mode   | Global              |
| `?`                  | Show Help popup                      | Global              |
| `s`                  | Switch to Status view                | Global              |
| `l`                  | Switch to Log view                   | Global              |
| `esc`                | Close any active popup               | Popups              |
| `j` / `↓` / `Scroll` | Navigate down in the active list     | Lists               |
| `k` / `↑` / `Scroll` | Navigate up in the active list       | Lists               |
| `h`                  | Set focus to the left (Files) panel  | Status View         |
| `l`                  | Set focus to the right (Diff) panel  | Status View         |
| `space`              | Stage the selected file or hunk      | Status View (Files) |
| `u`                  | Unstage the selected file            | Status View (Files) |
| `enter`              | Enter Hunk Selection mode for a file | Status View (Files) |
| `c`                  | Open Commit message popup            | Status View         |
| `Shift + P`          | Push changes to remote (`origin`)    | Status View         |
| `Click`              | Select item / Change panel focus     | Status View         |

## Technical Deep Dive

### Core Technologies

- **[Rust](https://www.rust-lang.org/)**: Chosen for its performance, memory safety, and powerful type system, which are ideal for building robust, concurrent applications.
- **[Ratatui](https://ratatui.rs/)**: A modern, community-maintained TUI framework for Rust. It provides a rich set of widgets and a flexible layout engine.
- **[Crossterm](https://github.com/crossterm-rs/crossterm)**: The backend for Ratatui, enabling cross-platform terminal manipulation, raw mode, and event handling (keyboard/mouse).
- **[git2-rs](https://github.com/rust-lang/git2-rs)**: Safe Rust bindings for `libgit2`. This was chosen over shelling out to the `git` command to ensure performance, type safety, and avoid brittle command parsing.
- **[Tokio](https://tokio.rs/)**: The de-facto asynchronous runtime for Rust. It is used here to handle slow network operations (like `git push`) in the background without blocking the main UI thread.
- **[thiserror](https://github.com/dtolnay/thiserror)**: Provides a derive macro for creating idiomatic, boilerplate-free custom error types.
- **[simplelog](https://github.com/drakulix/simplelog.rs)**: A straightforward logging facade for debugging. Logs are written to a file to avoid corrupting the TUI display.

### Architechtural Overview

Dotatui is built on several key architectural principles to ensure robustness and maintainability.

### 1. State as the Single Source of Truth

The application follows a pattern similar to the Elm Architechture (Model-View-Update).

- **Model:** The `App` struct in `app.rs` holds the entire state of the application.
- **View:** The `ui.rs` module contains pure functions that render the UI based _only_ on the current state passed from the `App` struct.
- **Update:** The main loop in `main.rs` processes events and calls methods on the `App` struct to udpate its state.

A critical design decision was to create a `status_display_list` within the `App` state. Early prototypes suffered from the bugs where the UI's list (containing headers) would desynchronize from the raw data list. By making the `App` state responsible for building the exact list to be dislayed, we created a single source of truth, eliminating this entire class of bugs.

### 2. Non-Blocking Asynchronous Operations

To keep the UI responsive, long-running I/O operations like `git push` are handled asynchronously.

- When the push is initiated, a `tokio::spawn` task is created.
- Crucially, `git2::Repository` is not thread-safe(`!Send`/`!Sync`). The solution is to pass the repository's `PathBuf` (which is thread-safe) to the new task which then opens it's own `Repository` instance.
- Communications back to the main UI thread is managed within a `tokio::sync::mpsc` channel, sending an `AppEvent` on completion(success or failure).

### 3. Robustness and Portability

- **CWD Handling:** The application correctly identifies the Git repository root on startup and immediately sets it as the process's Current Working Directory. This prevents a common and subtle class of path resolution errors, ensuring that `dotatui` behaves predictably no matter where it is launched from.
- **Centralized Error Handling:** A custom `AppError` enum defined in `error.rs` with`thiserror` provides a unified error type for the entire application, making function signatures clean and error propogation clear.

## Development

Instructions for developers and contributors.

- **Build for Debugging:**
  ```sh
  cargo build
  ```
- **Run Tests:**
  ```sh
  cargo test
  ```
- **Live Debug Logging:**
  While the application is running, you can monitor its internal state and events in a seperate terminal:
  ```sh
  tail -f dotatui.log
  ```

## Roadmap

Dotatui is under active development. Future plans include:

- [] **Full Hunk Staging:** Implement the UI and backend for staging/unstaging individual hunks in the diff view.
- [] **Branch Management:** Add a popup and backend functions to view, switch, create and delete branches.
- [] **Fetch & Pull:** Complete the remote workflow with fetch and pull operations.
- [] **Interative Log:** Allow checking out commits and viewing commit diffs directly from the log view.
- [] **Configuration File:** Allow users to customize keybindings and colors via a config file (e.g., `config.toml`).

## License

Distributed under the MIT License. See `License.txt` for more information.

## Acknowledgments

- Jesse Duffield for creating [lazygit](https://github.com.jesseduffield/lazygit), the primary inspiration for this project.

- The teams behind [Ratatui](https://ratatui.rs/) and the broader Rust TUI ecosystem.
