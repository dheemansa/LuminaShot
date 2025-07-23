# LuminaShot 📸

A fast, reactive screenshot tool for the **Hyprland** Wayland compositor, written in Rust.

> LuminaShot is designed to be a simple yet powerful utility for capturing your screen. Its standout feature is a fully reactive window selection mode that intelligently restarts when you switch workspaces, ensuring you always capture the window you intend to.

> [!WARNING]
> **LuminaShot** is designed *exclusively* for the **Hyprland** Wayland compositor.
>
> It depends on `hyprctl` and Hyprland-specific behavior — it **will not** work with other compositors like Sway, Weston, or GNOME.

## ✨ Features

* **Multiple Capture Modes**:

  * `Monitor`: Automatically captures the monitor your cursor is currently on. *(Default)*

  * `Window`: Interactively select any window on your current workspace.

  * `Region`: Click and drag to capture any portion of your screen.

* **Flexible Output**:

  * Save screenshots to a file. *(Default)*

  * Copy screenshots directly to the clipboard.

  * Do both at the same time!

* **Reactive Window Selection**: The window selection mode is built to be robust. If you switch workspaces while selecting, the process seamlessly restarts on the new workspace.

* **Desktop Notifications**: Get an instant notification with a preview of your screenshot as soon as it's captured.

* **Fast and Efficient**: Built in Rust with performance in mind. It's lightweight and has minimal overhead.

## ⚙️ Dependencies

To run LuminaShot, you need the following programs installed on your system:

* `hyprland` (Provides the `hyprctl` command)

* `grim` (The backend that captures the screen pixels)

* `slurp` (The tool for interactive selection)

* `wl-clipboard` (Provides `wl-copy` for clipboard support)

* `libnotify` (Provides `notify-send` for desktop notifications)

## 📥 Installation

These instructions are for Arch Linux, but can be adapted for other distributions.

1. **Install Dependencies:**
   Open a terminal and install the required packages.

   ```bash
   sudo pacman -S grim slurp wl-clipboard libnotify --needed
   ```

2. **Install the Rust Toolchain:**
   We recommend using `rustup` to install and manage your Rust installation.[^1]

   ```bash
   sudo pacman -S rustup --needed
   rustup default stable
   ```

3. **Build from Source:**
   Clone this repository and build the project in release mode for the best performance.

   ```bash
   git clone [https://github.com/dheemansa/LuminaShot.git](https://github.com/dheemansa/LuminaShot.git)
   cd LuminaShot
   cargo build --release
   ```

4. **Install the Binary:**
   Copy the compiled binary to a directory in your system's `PATH`.

   ```bash
   sudo cp target/release/luminashot /usr/local/bin/luminashot
   ```

## ⌨️ Usage & Configuration

Once installed, you can run LuminaShot from your terminal or, more conveniently, bind it to a key in your `hyprland.conf`.

### Command-Line Flags

| Flag | Long Flag | Description |
| :--- | :--- | :--- |
| `-s` | `--save` | Save the screenshot to a file. **This is the default action if no flags are provided.** |
| `-c` | `--copy` | Copy the screenshot to the clipboard. |
| `-cs`| `--copy --save` | Perform both actions: copy to clipboard and save to a file. |
| `-m` | `--mode` | Set the capture mode (`monitor`, `window`, or `region`). Defaults to `monitor`. |
| `-h` | `--help` | Show the help message with all options and examples. |

### Example Keybinds (`hyprland.conf`)

Here is an example of how you can set up keybinds for LuminaShot to handle different actions:

```
# Screenshot Keybinds for LuminaShot

# Save to file (default action)
bind = $mainMod, P, exec, luminashot -m window

# Copy to clipboard ONLY
bind = $mainMod SHIFT, P, exec, luminashot -m region -c

# Save AND Copy
bind = $mainMod CTRL, P, exec, luminashot -m monitor -cs
```

## 🗺️ Roadmap

* [x] Implement reactive window selection

* [x] Add desktop notifications

* [x] Add support for clipboard (`-c` flag)

* [x] Add flags for save (`-s`) and help (`-h`)

* [ ] Add flags for custom file name (`-f`) and custom save location

* [ ] Add support for `LUMINA_SAVE_DIR` environment variable

* [ ] Make my own version of slurp for better user interaction

## 💖 Credits

This project was inspired by [Hyprshot](https://github.com/Gustash/Hyprshot) by [@Gustash](https://github.com/Gustash).

[^1]:
Using `rustup` is the official and recommended way to install Rust. It allows you to easily manage multiple toolchains and keep your compiler up to date.
