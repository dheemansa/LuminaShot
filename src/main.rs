use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use chrono::Local;

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
#[command(author, version, about = "A reactive screenshot tool for Hyprland written in Rust")]
struct Cli {
    #[arg(short, long, value_enum, default_value_t = Mode::Monitor)]
    mode: Mode,
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
    let cli = Cli::parse();

    let geometry = match cli.mode {
        Mode::Region => region_mode().await?,
        Mode::Window => window_mode().await?,
        Mode::Monitor => monitor_mode().await?,
    };

    if let Some(geom) = geometry {
        println!("Capturing geometry: {}", geom);
        let file_path = run_grim(&geom).await?;
        send_notification(&file_path, &cli.mode).await?;
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
        // --- 1. Get the current state ---
        let initial_workspace_id = get_active_workspace_id().await?;
        let windows = get_windows_on_workspace(initial_workspace_id).await?;
        
        if windows.is_empty() {
            println!("No windows on active workspace. Waiting for a window or workspace change...");
            // If there are no windows, just wait for a workspace change to restart.
            monitor_workspace_changes_by_polling(initial_workspace_id).await?;
            continue; // Restart the loop on the new workspace.
        }

        let slurp_input = windows
            .iter()
            .map(|w| format!("{},{} {}x{} {}", w.at.0, w.at.1, w.size.0, w.size.1, w.address))
            .collect::<Vec<_>>()
            .join("\n");

        // --- 2. Spawn slurp as a child process ---
        let mut slurp_process = Command::new("slurp")
            .args(["-r", "-b", "#FFFFFF44", "-f", "%l"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to spawn slurp")?;
        
        if let Some(mut stdin) = slurp_process.stdin.take() {
            tokio::io::AsyncWriteExt::write_all(&mut stdin, slurp_input.as_bytes()).await?;
        }

        let slurp_pid = slurp_process.id().context("Failed to get slurp PID")?;

        // --- 3. Spawn the workspace monitor using polling ---
        let mut monitor_handle = tokio::spawn(async move {
            monitor_workspace_changes_by_polling(initial_workspace_id).await
        });

        // --- 4. The Race! ---
        tokio::select! {
            // Case A: The user selected a window or cancelled slurp
            slurp_result = slurp_process.wait_with_output() => {
                monitor_handle.abort(); // Stop the monitor
                let output = slurp_result?;
                if output.status.success() {
                    let selected_address = String::from_utf8(output.stdout)?.trim().to_string();
                    let final_geom = get_geometry_for_address(&selected_address).await?;
                    return Ok(Some(final_geom)); // Success! Exit the loop.
                } else {
                    return Ok(None); // User cancelled with Esc.
                }
            },
            // Case B: The polling monitor detected a workspace change
            monitor_result = &mut monitor_handle => {
                let _ = Command::new("kill").arg(slurp_pid.to_string()).status().await;
                if monitor_result.is_ok() {
                    println!("Workspace changed, restarting selection...");
                }
                // The loop continues...
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
                return Ok(()); // Workspace has changed
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

/// Executes the final grim command to take the screenshot and returns the file path.
async fn run_grim(geometry: &str) -> Result<String> {
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let pictures_dir = std::env::var("XDG_PICTURES_DIR").unwrap_or_else(|_| format!("{}/Pictures", std::env::var("HOME").unwrap()));
    
    let save_dir = format!("{}/Screenshots", pictures_dir);
    std::fs::create_dir_all(&save_dir)?;

    let file_path = format!("{}/{}-luminashot.png", save_dir, timestamp);

    let status = Command::new("grim")
        .arg("-g")
        .arg(geometry)
        .arg(&file_path)
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("grim command failed!");
    }
    
    Ok(file_path)
}

/// Sends a desktop notification with the screenshot preview.
async fn send_notification(file_path: &str, mode: &Mode) -> Result<()> {
    let mode_str = format!("{:?}", mode);
    let summary = format!("LuminaShot - {} Mode", mode_str);
    let body = format!("Screenshot saved to {}", file_path);

    let status = Command::new("notify-send")
        .arg(&summary)
        .arg(&body)
        .arg("-i")
        .arg(file_path)
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("notify-send command failed");
    }

    Ok(())
}
