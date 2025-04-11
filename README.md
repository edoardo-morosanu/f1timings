# F1Timings-RS

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**❗ Development Status: Paused Indefinitely ❗**

> **This Rust version of F1Timings is currently not under active development.** Development efforts have shifted to the **Python** implementation, leveraging Python's ecosystem for planned features like real-time telemetry processing.
>
> **For the actively developed version, please see the Python implementation:** > **➡️ [f1timings-py](https://github.com/edoardo-morosanu/f1timings-py) ⬅️**

---

This repository contains the original **Rust** implementation using the Axum framework, intended to track F1 lap times, manage driver data, and serve basic admin/display frontends. While functional for its initial scope (API for lap times, track info, data export), new features like real-time UDP telemetry processing and automatic display updates are planned **exclusively for the Python version**.

## Features

- **Web API:**
  - Manage drivers and their fastest lap times (Add, View, Delete).
  - Set and retrieve the current track name.
  - In-memory data storage.
- **Data Export:** Export current lap time standings to CSV and JSON formats.
- **Static File Serving:** Serves static HTML/JS frontends for administration and display (`/admin`, `/display`).

_(For planned features like real-time UDP telemetry and WebSocket support for automatic display updates, please refer to the [f1timings-py](https://github.com/edoardo-morosanu/f1timings-py) repository.)_

## Prerequisites

- **Rust:** Latest stable version recommended. Install via [rustup](https://rustup.rs/).
- **Cargo:** Rust's package manager and build tool (comes with rustup).

## Installation & Setup

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/edoardo-morosanu/f1timings-rs.git
    cd f1timings-rs
    ```

2.  **Build the project:**

    ```bash
    # For development
    cargo build

    # For production
    cargo build --release
    ```

## Running the Application

- **Using Cargo:**

  ```bash
  # Development
  cargo run

  # Production
  cargo run --release
  ```

By default, the server listens on `0.0.0.0:8080`.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
