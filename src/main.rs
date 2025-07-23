use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use chrono::Local;
use tokio::io::AsyncWriteExt;

// --- Data Structures for Hyprland's JSON Output ---

#[derive(Deserialize, Debug)]
struct HyprlandClient {
    address: String,
    at: (i32, i32),
    size: (i32, i32),
    workspace: HyprlandWorkspace,
    hidden: bool,
}

#[derive(Deserialize, Debug, Clone)]
struct HyprlandWorkspace {
    id: i32,
    name: String,
}

#[derive(Deserialize, Debug)]
struct HyprlandMonitor {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Deserialize, Debug)]
struct HyprlandCursorPos {
    x: i32,
    y: i32,
}

// --- Command-Line Argument Parsing ---

#[derive(Parser, Debug)]
#[command(
author,
version,
about, // clap will use the crate's description from Cargo.toml
after_help = r#"EXAMPLES:
# Capture the current monitor and save it (default action)
luminashot -m monitor

# Select a region and copy it to the clipboard
luminashot -m region -c

# Select a window, save it to a file, AND copy it to the clipboard
luminashot -m window --copy --save

# A shorter way to do the same as above
luminashot -m window -cs"#
)]
struct Cli {
    #[arg(short, long, value_enum, default_value_t = Mode::Monitor, help = "Set the capture mode")]
    mode: Mode,

    #[arg(short, long, help = "Copy the screenshot to the clipboard")]
    copy: bool,

    #[arg(short, long, help = "Save the screenshot to a file (default if no output flag is specified)")]
    save: bool,
}

#[derive(ValueEnum, Clone, Debug)]
enum Mode {
    Region,
    Window,
    Monitor,
}

// --- Main Application Logic ---

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    // Default action is to save if no output flag is specified.
    if !cli.copy && !cli.save {
        cli.save = true;
    }

    let geometry = match cli.mode {
        Mode::Region => region_mode().await?,
        Mode::Window => window_mode().await?,
        Mode::Monitor => monitor_mode().await?,
    };

    if let Some(geom) = geometry {
        println!("Capturing geometry: {}", geom);

        // Capture the image data into a buffer in memory first.
        let image_buffer = capture_geometry_to_buffer(&geom).await?;
        let mut file_path: Option<String> = None;

        if cli.save {
            let path = save_buffer_to_file(&image_buffer).await?;
            file_path = Some(path);
        }

        if cli.copy {
            copy_buffer_to_clipboard(&image_buffer).await?;
        }

        // Send a notification based on the actions performed.
        send_notification(cli.copy, file_path.as_deref(), &cli.mode).await?;

    } else {
        println!("Action cancelled.");
    }

    Ok(())
}

// --- Screenshot Mode Implementations ---

/// Simple region selection mode.
async fn region_mode() -> Result<Option<String>> {
    let slurp_output = Command::new("slurp")
    .arg("-b")
    .arg("#FFFFFF44")
    .output()
    .await?;

    if slurp_output.status.success() {
        Ok(Some(String::from_utf8(slurp_output.stdout)?.trim().to_string()))
    } else {
        Ok(None)
    }
}

/// Auto-detects the monitor under the cursor.
async fn monitor_mode() -> Result<Option<String>> {
    let cursor_pos_output = Command::new("hyprctl")
    .arg("cursorpos")
    .arg("-j")
    .output()
    .await?;

    let cursor_pos: HyprlandCursorPos = serde_json::from_slice(&cursor_pos_output.stdout)?;

    let monitors_output = Command::new("hyprctl")
    .arg("monitors")
    .arg("-j")
    .output()
    .await?;

    let monitors: Vec<HyprlandMonitor> = serde_json::from_slice(&monitors_output.stdout)?;

    for monitor in monitors {
        if cursor_pos.x >= monitor.x && cursor_pos.x < monitor.x + monitor.width &&
            cursor_pos.y >= monitor.y && cursor_pos.y < monitor.y + monitor.height {
                return Ok(Some(format!("{},{} {}x{}", monitor.x, monitor.y, monitor.width, monitor.height)));
            }
    }

    anyhow::bail!("Could not find a monitor under the cursor.");
}

/// Implements the full reactive "monitor and restart" window selection using polling.
async fn window_mode() -> Result<Option<String>> {
    loop {
        let initial_workspace_id = get_active_workspace_id().await?;
        let windows = get_windows_on_workspace(initial_workspace_id).await?;

        if windows.is_empty() {
            println!("No windows on active workspace. Waiting for a window or workspace change...");
            monitor_workspace_changes_by_polling(initial_workspace_id).await?;
            continue;
        }

        let slurp_input = windows
        .iter()
        .map(|w| format!("{},{} {}x{} {}", w.at.0, w.at.1, w.size.0, w.size.1, w.address))
        .collect::<Vec<_>>()
        .join("\n");

        let mut slurp_process = Command::new("slurp")
        .args(["-r", "-b", "#FFFFFF44", "-f", "%l"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn slurp")?;

        if let Some(mut stdin) = slurp_process.stdin.take() {
            stdin.write_all(slurp_input.as_bytes()).await?;
        }

        let slurp_pid = slurp_process.id().context("Failed to get slurp PID")?;

        let mut monitor_handle = tokio::spawn(async move {
            monitor_workspace_changes_by_polling(initial_workspace_id).await
        });

        tokio::select! {
            slurp_result = slurp_process.wait_with_output() => {
                monitor_handle.abort();
                let output = slurp_result?;
                if output.status.success() {
                    let selected_address = String::from_utf8(output.stdout)?.trim().to_string();
                    let final_geom = get_geometry_for_address(&selected_address).await?;
                    return Ok(Some(final_geom));
                } else {
                    return Ok(None);
                }
            },
            monitor_result = &mut monitor_handle => {
                let _ = Command::new("kill").arg(slurp_pid.to_string()).status().await;
                if monitor_result.is_ok() {
                    println!("Workspace changed, restarting selection...");
                }
            }
        }
    }
}

// --- Helper Functions ---

/// Gets the ID of the currently active workspace.
async fn get_active_workspace_id() -> Result<i32> {
    let output = Command::new("hyprctl")
    .arg("activeworkspace")
    .arg("-j")
    .output()
    .await?;
    let workspace: HyprlandWorkspace = serde_json::from_slice(&output.stdout)?;
    Ok(workspace.id)
}

/// Gets the list of all visible windows on a specific workspace ID.
async fn get_windows_on_workspace(workspace_id: i32) -> Result<Vec<HyprlandClient>> {
    let clients_output = Command::new("hyprctl")
    .arg("clients")
    .arg("-j")
    .output()
    .await?;
    let all_clients: Vec<HyprlandClient> = serde_json::from_slice(&clients_output.stdout)?;

    let visible_clients = all_clients
    .into_iter()
    .filter(|c| !c.hidden && c.workspace.id == workspace_id)
    .collect();

    Ok(visible_clients)
}

/// Monitors for workspace changes by polling `hyprctl`.
async fn monitor_workspace_changes_by_polling(initial_id: i32) -> Result<()> {
    loop {
        sleep(Duration::from_millis(200)).await;
        if let Ok(current_id) = get_active_workspace_id().await {
            if current_id != initial_id {
                return Ok(());
            }
        }
    }
}

/// After a window is selected, this gets its final, most up-to-date geometry.
async fn get_geometry_for_address(address: &str) -> Result<String> {
    let clients_output = Command::new("hyprctl")
    .arg("clients")
    .arg("-j")
    .output()
    .await?;
    let all_clients: Vec<HyprlandClient> = serde_json::from_slice(&clients_output.stdout)?;

    for client in all_clients {
        if client.address == address {
            return Ok(format!("{},{} {}x{}", client.at.0, client.at.1, client.size.0, client.size.1));
        }
    }

    anyhow::bail!("Could not find window with address {} after selection", address);
}

/// Runs grim and captures the output to a byte buffer in memory.
async fn capture_geometry_to_buffer(geometry: &str) -> Result<Vec<u8>> {
    let output = Command::new("grim")
    .arg("-g")
    .arg(geometry)
    .arg("-") // Output to stdout
    .output()
    .await?;

    if !output.status.success() {
        anyhow::bail!("grim command failed!");
    }

    Ok(output.stdout)
}

/// Takes an image buffer and saves it to a file.
async fn save_buffer_to_file(buffer: &[u8]) -> Result<String> {
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let pictures_dir = std::env::var("XDG_PICTURES_DIR").unwrap_or_else(|_| format!("{}/Pictures", std::env::var("HOME").unwrap()));

    let save_dir = format!("{}/Screenshots", pictures_dir);
    tokio::fs::create_dir_all(&save_dir).await?;

    let file_path = format!("{}/{}-luminashot.png", save_dir, timestamp);
    tokio::fs::write(&file_path, buffer).await?;

    Ok(file_path)
}

/// Takes an image buffer and pipes it to wl-copy.
async fn copy_buffer_to_clipboard(buffer: &[u8]) -> Result<()> {
    let mut wl_copy_cmd = Command::new("wl-copy")
    .stdin(Stdio::piped())
    .spawn()
    .context("Failed to spawn wl-copy")?;

    let mut wl_copy_stdin = wl_copy_cmd.stdin.take().context("Failed to get wl-copy stdin")?;

    // Write the buffer to wl-copy's stdin
    wl_copy_stdin.write_all(buffer).await?;
    drop(wl_copy_stdin); // Close stdin to signal end of data

    let wl_copy_status = wl_copy_cmd.wait().await?;
    if !wl_copy_status.success() {
        anyhow::bail!("wl-copy command failed!");
    }

    Ok(())
}

/// Sends a desktop notification summarizing the actions taken.
async fn send_notification(copied: bool, file_path: Option<&str>, mode: &Mode) -> Result<()> {
    let mode_str = format!("{:?}", mode);
    let summary = format!("LuminaShot - {} Mode", mode_str);

    let body = match (copied, file_path) {
        (true, Some(path)) => format!("Copied and saved to {}", path),
        (true, None) => "Copied to clipboard.".to_string(),
        (false, Some(path)) => format!("Saved to {}", path),
        (false, None) => return Ok(()), // Should not happen with current logic
    };

    let mut notify_cmd = Command::new("notify-send");
    notify_cmd.arg(&summary).arg(&body);

    // Use a file path for the icon if available, otherwise use a generic icon for copy.
    if let Some(path) = file_path {
        notify_cmd.arg("-i").arg(path);
    } else {
        notify_cmd.arg("-i").arg("edit-copy");
    }

    let status = notify_cmd.status().await?;

    if !status.success() {
        anyhow::bail!("notify-send command failed");
    }

    Ok(())
}
