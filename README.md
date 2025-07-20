# LuminaShot 📸

A fast, reactive screenshot tool for the **Hyprland** Wayland compositor, written in Rust.

> LuminaShot is designed to be a simple yet powerful utility for capturing your screen. Its standout feature is a fully reactive window selection mode that intelligently restarts when you switch workspaces, ensuring you always capture the window you intend to.

---

## ✨ Features

* **Multiple Capture Modes**:
    * `Monitor`: Automatically captures the monitor your cursor is currently on. *(Default)*
    * `Window`: Interactively select any window on your current workspace.
    * `Region`: Click and drag to capture any portion of your screen.
* **Reactive Window Selection**: The window selection mode is built to be robust. If you switch workspaces while selecting, the process seamlessly restarts on the new workspace. ~~No more incorrect captures!~~
* **Desktop Notifications**: Get an instant notification with a preview of your screenshot as soon as it's saved.
* **Fast and Efficient**: Built in Rust with performance in mind. It's lightweight and has minimal overhead.

---

## 🛠️ Dependencies

To run LuminaShot, you need the following programs installed on your system:

-   `hyprland` (Provides the `hyprctl` command)
-   `grim` (The backend that captures the screen pixels)
-   `slurp` (The tool for interactive selection)
-   `libnotify` (Provides the `notify-send` command for desktop notifications)

---

## 🚀 Installation

These instructions are for Arch Linux, but can be adapted for other distributions.

1.  **Install Dependencies:**
    Open a terminal and install the required packages from the official repositories.
    ```bash
    sudo pacman -Syu hyprland grim slurp libnotify
    ```

2.  **Install the Rust Toolchain:**
    We recommend using `rustup` to install and manage your Rust installation.[^1]
    ```bash
    sudo pacman -S rustup
    rustup default stable
    ```

3.  **Build from Source:**
    Clone this repository and build the project in release mode for the best performance.
    ```bash
    # Replace with the actual repository URL
    git clone [https://github.com/your-username/luminashot.git](https://github.com/your-username/luminashot.git)
    cd luminashot
    cargo build --release
    ```

4.  **Install the Binary:**
    Copy the compiled binary to a directory in your system's `PATH`.
    ```bash
    sudo cp target/release/luminashot /usr/local/bin/luminashot
    ```

---

## ⌨️ Usage & Configuration

Once installed, you can run LuminaShot from your terminal or, more conveniently, bind it to a key in your `hyprland.conf`.

### Terminal Usage

| Mode | Command | Description |
| :--- | :--- | :--- |
| Monitor | `luminashot -m monitor` | Capture the monitor under the cursor. |
| Window | `luminashot -m window` | Interactively select a window. |
| Region | `luminashot -m region` | Interactively select a region. |

### Example Keybinds (`hyprland.conf`)

Here is an example of how you can set up keybinds for LuminaShot:
'''
#Screenshot Keybinds for LuminaShot
bind = $mainMod, P, exec, luminashot -m window      # Select a window
bind = $mainMod SHIFT, P, exec, luminashot -m region    # Select a region
bind = $mainMod CTRL, P, exec, luminashot -m monitor   # Capture current monitor
'''

---

## 🗺️ Roadmap

-   [x] Implement reactive window selection
-   [x] Add desktop notifications
-   [ ] Add a `--clipboard-only` flag
-   [ ] Allow custom filenames via command-line argument
-   [ ] Add support for `LUMINA_SAVE_DIR` environment variable

That's the plan! :joy:

[^1]: Using `rustup` is the official and recommended way to install Rust. It allows you to easily manage multiple toolchains and keep your compiler up to date.
