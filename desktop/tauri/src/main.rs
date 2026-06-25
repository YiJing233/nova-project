#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader};
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use regex::Regex;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

struct SidecarProcess(Mutex<Option<CommandChild>>);

fn get_workspace_from_args(app: &tauri::App) -> Option<String> {
    let args: Vec<String> = {
        #[cfg(debug_assertions)]
        {
            std::env::args().skip(1).collect()
        }
        #[cfg(not(debug_assertions))]
        {
            app.env().args_os.iter()
                .skip(1)
                .map(|s| s.to_string_lossy().to_string())
                .collect()
        }
    };

    for arg in args {
        let p = std::path::Path::new(&arg);
        if p.exists() && (p.is_dir() || arg.ends_with(".toml")) {
            return Some(arg);
        }
    }
    None
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window");

            let mut args = vec!["--desktop".to_string()];
            if let Some(ws) = get_workspace_from_args(app) {
                args.push("--workspace".to_string());
                args.push(ws);
            }

            let sidecar_command = app.shell().sidecar("nova")
                .map_err(|e| format!("Failed to create sidecar: {}", e))?;

            let (mut rx, child) = sidecar_command
                .args(args)
                .spawn()
                .map_err(|e| format!("Failed to spawn nova: {}", e))?;

            app.manage(SidecarProcess(Mutex::new(Some(child))));

            let window_clone = window.clone();
            tauri::async_runtime::spawn(async move {
                let port_re = Regex::new(r"\[nova-desktop-ready\] port=(\d+)").unwrap();
                let mut found_port: Option<u16> = None;

                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(line) => {
                            let line = String::from_utf8_lossy(&line);
                            println!("[nova] {}", line.trim());
                            if let Some(caps) = port_re.captures(&line) {
                                if let Some(m) = caps.get(1) {
                                    found_port = m.as_str().parse::<u16>().ok();
                                }
                            }
                        }
                        CommandEvent::Stderr(line) => {
                            let line = String::from_utf8_lossy(&line);
                            eprintln!("[nova] {}", line.trim());
                        }
                        CommandEvent::Terminated(status) => {
                            eprintln!("[nova] process terminated: {:?}", status);
                            break;
                        }
                        _ => {}
                    }
                }

                let port = if let Some(p) = found_port {
                    p
                } else {
                    let client = reqwest::blocking::Client::builder()
                        .timeout(Duration::from_millis(300))
                        .build()
                        .unwrap();

                    let mut scanned = 8080;
                    for p in 8080..8200 {
                        if client.get(format!("http://127.0.0.1:{}", p)).send().is_ok() {
                            scanned = p;
                            break;
                        }
                    }
                    scanned
                };

                let url = format!("http://127.0.0.1:{}", port);
                window_clone.eval(&format!("window.location.replace('{}');", url)).ok();
                window_clone.show().ok();
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let state = window.state::<SidecarProcess>();
                if let Ok(mut guard) = state.0.lock() {
                    if let Some(ref mut child) = *guard {
                        let _ = child.kill();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
