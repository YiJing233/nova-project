#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use std::time::Duration;

use regex::Regex;
use tauri::Menu;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
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

fn build_menu(app: &tauri::App) -> Result<Menu(), tauri::Error> {
    let pkg_info = app.package_info();
    let app_name = &pkg_info.name;

    let mut menu = MenuBuilder::new(app);

    let app_submenu = SubmenuBuilder::new(app, app_name)
        .item(&PredefinedMenuItem::about(app, Some("关于 Nova"), Default::default())?)
        .separator()
        .item(&PredefinedMenuItem::services(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, None)?)
        .item(&PredefinedMenuItem::hide_others(app, None)?)
        .item(&PredefinedMenuItem::show_all(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some("退出"))?)
        .build()?;

    let edit_submenu = SubmenuBuilder::new(app, "编辑")
        .item(&PredefinedMenuItem::undo(app, Some("撤销"))?)
        .item(&PredefinedMenuItem::redo(app, Some("重做"))?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, Some("剪切"))?)
        .item(&PredefinedMenuItem::copy(app, Some("复制"))?)
        .item(&PredefinedMenuItem::paste(app, Some("粘贴"))?)
        .item(&PredefinedMenuItem::select_all(app, Some("全选"))?)
        .build()?;

    let view_submenu = {
        let mut builder = SubmenuBuilder::new(app, "视图");
        builder = builder
            .item(&MenuItemBuilder::with_id("reload", "重新加载").accelerator("CmdOrCtrl+R").build(app)?)
            .item(&MenuItemBuilder::with_id("go-back", "后退").accelerator("CmdOrCtrl+[").build(app)?)
            .item(&MenuItemBuilder::with_id("go-forward", "前进").accelerator("CmdOrCtrl+]").build(app)?)
            .separator()
            .item(&PredefinedMenuItem::fullscreen(app, Some("进入全屏"))?);

        if cfg!(debug_assertions) {
            builder = builder.separator()
                .item(&MenuItemBuilder::with_id("devtools", "切换开发者工具").accelerator("CmdOrCtrl+Shift+I").build(app)?);
        }
        builder.build()?
    };

    let window_submenu = SubmenuBuilder::new(app, "窗口")
        .item(&PredefinedMenuItem::minimize(app, Some("最小化"))?)
        .item(&PredefinedMenuItem::close_window(app, Some("关闭窗口"))?)
        .build()?;

    #[cfg(target_os = "macos")]
    {
        menu = menu.item(&app_submenu);
    }
    menu = menu.item(&edit_submenu).item(&view_submenu).item(&window_submenu);

    #[cfg(not(target_os = "macos"))]
    {
        menu = menu.item(&app_submenu);
    }

    menu.build()
}

fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
}

fn show_error_page(window: &tauri::WebviewWindow, title: &str, message: &str) {
    let js = format!(r#"
        document.body.style.cssText = 'margin:0;padding:40px;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;background:#1a1a2e;color:#eee;display:flex;justify-content:center;';
        document.body.innerHTML = '<div style="max-width:640px">' +
            '<h1 style="color:#ff6b6b;font-size:24px;margin-bottom:16px">{title}</h1>' +
            '<pre style="background:#0f0f1a;padding:16px;border-radius:8px;overflow-x:auto;color:#e0e0e0;font-family:SF Mono,Consolas,monospace;font-size:13px;white-space:pre-wrap">{message}</pre>' +
            '<div style="margin-top:24px;padding:16px;background:#16213e;border-radius:8px;border-left:4px solid #4ecdc4">' +
            '<h3 style="margin-top:0;color:#4ecdc4;font-size:14px">调试提示</h3>' +
            '<p style="line-height:1.6;color:#aaa;margin:8px 0">• 查看终端/控制台输出获取详细日志</p>' +
            '<p style="line-height:1.6;color:#aaa;margin:8px 0">• 按 <code style="background:#0f0f1a;padding:2px 6px;border-radius:3px;font-size:12px">F12</code> 或 <code style="background:#0f0f1a;padding:2px 6px;border-radius:3px;font-size:12px">Cmd+Opt+I</code> 打开开发者工具</p>' +
            '<p style="line-height:1.6;color:#aaa;margin:8px 0">• 确认 <code style="background:#0f0f1a;padding:2px 6px;border-radius:3px;font-size:12px">nova</code> 可执行文件存在且有执行权限</p>' +
            '<p style="line-height:1.6;color:#aaa;margin:8px 0">• 后端日志位于程序目录下的 <code style="background:#0f0f1a;padding:2px 6px;border-radius:3px;font-size:12px">log/</code> 文件夹</p>' +
            '</div></div>';
    "#,
        title = escape_js_string(title),
        message = escape_js_string(message)
    );
    let _ = window.eval(&js);
    let _ = window.show();
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window");

            if cfg!(debug_assertions) {
                window.open_devtools();
            }

            let menu = build_menu(app)?;
            app.set_menu(menu)?;

            app.on_menu_event(move |app, event| {
                let win = app.get_webview_window("main");
                if win.is_none() { return; }
                let win = win.unwrap();
                match event.id().0.as_str() {
                    "reload" => {
                        let _ = win.eval("location.reload()");
                    }
                    "go-back" => {
                        let _ = win.eval("history.back()");
                    }
                    "go-forward" => {
                        let _ = win.eval("history.forward()");
                    }
                    "devtools" => {
                        if win.is_devtools_open() {
                            win.close_devtools();
                        } else {
                            win.open_devtools();
                        }
                    }
                    _ => {}
                }
            });

            let mut args = vec!["--desktop".to_string()];
            if let Some(ws) = get_workspace_from_args(app) {
                args.push("--workspace".to_string());
                args.push(ws);
            }

            let sidecar_command = match app.shell().sidecar("nova") {
                Ok(cmd) => cmd,
                Err(e) => {
                    let msg = format!("无法定位 sidecar 程序 nova\n错误信息: {}", e);
                    eprintln!("[tauri] {}", msg);
                    show_error_page(&window, "Sidecar 程序未找到", &msg);
                    return Ok(());
                }
            };

            let (mut rx, child) = match sidecar_command
                .args(args)
                .spawn()
            {
                Ok(pair) => pair,
                Err(e) => {
                    let msg = format!("无法启动 nova 进程\n错误信息: {}\n\n请确认 nova 二进制文件存在且有执行权限。", e);
                    eprintln!("[tauri] {}", msg);
                    show_error_page(&window, "后端进程启动失败", &msg);
                    return Ok(());
                }
            };

            app.manage(SidecarProcess(Mutex::new(Some(child))));

            let window_clone = window.clone();
            tauri::async_runtime::spawn(async move {
                let port_re = Regex::new(r"\[nova-desktop-ready\] port=(\d+)").unwrap();
                let mut found_port: Option<u16> = None;
                let mut startup_log = String::new();

                for attempt in 0..120 {
                    tokio::select! {
                        Some(event) = rx.recv() => {
                            match event {
                                CommandEvent::Stdout(line) => {
                                    let line = String::from_utf8_lossy(&line);
                                    let trimmed = line.trim();
                                    println!("[nova:stdout] {}", trimmed);
                                    startup_log.push_str(&format!("[stdout] {}\n", trimmed));
                                    if let Some(caps) = port_re.captures(&line) {
                                        if let Some(m) = caps.get(1) {
                                            found_port = m.as_str().parse::<u16>().ok();
                                        }
                                    }
                                }
                                CommandEvent::Stderr(line) => {
                                    let line = String::from_utf8_lossy(&line);
                                    let trimmed = line.trim();
                                    eprintln!("[nova:stderr] {}", trimmed);
                                    startup_log.push_str(&format!("[stderr] {}\n", trimmed));
                                }
                                CommandEvent::Terminated(status) => {
                                    eprintln!("[nova] 进程已终止 exit_status={:?}", status);
                                    let msg = format!("后端进程在启动过程中退出。\n\n启动日志：\n{}\n\n退出状态: {:?}", startup_log, status);
                                    show_error_page(&window_clone, "后端进程异常退出", &msg);
                                    return;
                                }
                                _ => {}
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_millis(500)) => {
                            if found_port.is_some() {
                                break;
                            }
                            if attempt % 10 == 0 {
                                println!("[tauri] 等待后端就绪... ({}s)", attempt / 2);
                            }
                        }
                    }
                    if found_port.is_some() {
                        break;
                    }
                }

                let port = if let Some(p) = found_port {
                    p
                } else {
                    println!("[tauri] 未从 stdout 获取到端口，尝试扫描...");
                    let client = reqwest::blocking::Client::builder()
                        .timeout(Duration::from_millis(300))
                        .build()
                        .unwrap();

                    let mut scanned = None;
                    for p in 8080..8300 {
                        if client.get(format!("http://127.0.0.1:{}", p)).send().is_ok() {
                            scanned = Some(p);
                            break;
                        }
                    }
                    match scanned {
                        Some(p) => {
                            println!("[tauri] 扫描到服务在端口 {}", p);
                            p
                        }
                        None => {
                            let msg = format!("等待后端启动超时（60秒）。\n\n启动日志：\n{}", startup_log);
                            show_error_page(&window_clone, "后端启动超时", &msg);
                            return;
                        }
                    }
                };

                let url = format!("http://127.0.0.1:{}", port);
                println!("[tauri] 后端就绪，导航到 {}", url);
                window_clone.eval(&format!("window.location.replace('{}');", url)).ok();
                window_clone.show().ok();
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let state = window.state::<SidecarProcess>();
                let mut guard = state.0.lock().unwrap();
                if let Some(child) = guard.take() {
                    println!("[tauri] 窗口关闭，终止 nova 进程");
                    let _ = child.kill();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
