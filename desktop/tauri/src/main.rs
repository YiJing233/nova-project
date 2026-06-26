#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use std::time::Duration;

use regex::Regex;
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
        if p.exists() && (p.is_dir() || arg.ends_with(".toml") || arg.ends_with(".nova")) {
            return Some(arg);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn fix_sidecar_permissions(path: &std::path::Path) {
    use std::process::Command;
    let path_str = path.to_string_lossy().to_string();
    let _ = Command::new("chmod").args(["+x", &path_str]).status();
    let _ = Command::new("xattr").args(["-dr", "com.apple.quarantine", &path_str]).status();
}

#[cfg(target_os = "linux")]
fn fix_sidecar_permissions(path: &std::path::Path) {
    use std::process::Command;
    let _ = Command::new("chmod").args(["+x", &path.to_string_lossy()]).status();
}

#[cfg(target_os = "windows")]
fn fix_sidecar_permissions(_path: &std::path::Path) {}

fn loading_page_html() -> String {
    r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Nova</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{width:100vw;height:100vh;display:flex;align-items:center;justify-content:center;background:linear-gradient(135deg,#0f0c29 0%,#302b63 50%,#24243e 100%);font-family:-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC','Microsoft YaHei',sans-serif;overflow:hidden;-webkit-user-select:none;user-select:none}
.container{text-align:center;animation:fadeIn .6s ease-out}
@keyframes fadeIn{from{opacity:0;transform:translateY(10px)}to{opacity:1;transform:translateY(0)}}
.logo{width:80px;height:80px;margin:0 auto 24px;background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);border-radius:20px;display:flex;align-items:center;justify-content:center;font-size:36px;font-weight:700;color:#fff;box-shadow:0 8px 32px rgba(102,126,234,.3);animation:pulse 2s ease-in-out infinite}
@keyframes pulse{0%,100%{transform:scale(1);box-shadow:0 8px 32px rgba(102,126,234,.3)}50%{transform:scale(1.05);box-shadow:0 12px 40px rgba(102,126,234,.5)}}
.app-name{font-size:28px;font-weight:600;color:#fff;margin-bottom:8px;letter-spacing:-.5px}
.tagline{font-size:14px;color:rgba(255,255,255,.5);margin-bottom:40px}
.loading-bar{width:200px;height:3px;background:rgba(255,255,255,.1);border-radius:2px;margin:0 auto;overflow:hidden;position:relative}
.loading-bar::after{content:'';position:absolute;top:0;left:0;height:100%;width:40%;background:linear-gradient(90deg,transparent,#667eea,#764ba2,transparent);border-radius:2px;animation:loading 1.5s ease-in-out infinite}
@keyframes loading{0%{left:-40%}100%{left:100%}}
.status{margin-top:20px;font-size:13px;color:rgba(255,255,255,.4);min-height:18px}
</style>
</head>
<body>
<div class="container">
  <div class="logo">N</div>
  <div class="app-name">Nova</div>
  <div class="tagline">AI-native 创作工作台</div>
  <div class="loading-bar"></div>
  <div class="status" id="status">正在启动...</div>
</div>
</body>
</html>"#.to_string()
}

fn navigate_to_loading(window: &tauri::WebviewWindow) {
    use base64::Engine;
    let html = loading_page_html();
    let b64 = base64::engine::general_purpose::STANDARD.encode(html.as_bytes());
    let url = format!("data:text/html;base64,{}", b64);
    let _ = window.eval(&format!("window.location.replace('{}');", url.replace('\'', "%27")));
}

fn build_menu(app: &tauri::App) -> Result<tauri::Menu, tauri::Error> {
    let pkg_info = app.package_info();
    let app_name = &pkg_info.name;

    let mut menu = MenuBuilder::new(app);

    let app_submenu = SubmenuBuilder::new(app, app_name)
        .item(&PredefinedMenuItem::about(app, Some("关于 Nova"), Default::default())?)
        .separator()
        .item(&PredefinedMenuItem::services(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, Some("隐藏"))?)
        .item(&PredefinedMenuItem::hide_others(app, Some("隐藏其他"))?)
        .item(&PredefinedMenuItem::show_all(app, Some("显示全部"))?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some("退出 Nova"))?)
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

    let view_submenu = SubmenuBuilder::new(app, "视图")
        .item(&MenuItemBuilder::with_id("reload", "重新加载").accelerator("CmdOrCtrl+R").build(app)?)
        .separator()
        .item(&PredefinedMenuItem::fullscreen(app, Some("进入全屏"))?)
        .separator()
        .item(&MenuItemBuilder::with_id("devtools", "切换开发者工具").accelerator("CmdOrCtrl+Alt+I").build(app)?)
        .build()?;

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
}

fn show_user_error(window: &tauri::WebviewWindow, title: &str, user_message: &str, technical_logs: Option<&str>) {
    let logs_section = if let Some(logs) = technical_logs {
        format!(r#"
<div style="margin-top:24px">
  <details style="background:#0f0f1a;border-radius:8px;overflow:hidden">
    <summary style="padding:12px 16px;cursor:pointer;color:#888;font-size:13px;user-select:none;list-style:none;display:flex;justify-content:space-between;align-items:center">
      <span>技术详情（用于向开发者反馈）</span>
      <span style="transition:transform .2s">▶</span>
    </summary>
    <pre style="margin:0;padding:16px;border-top:1px solid #222;overflow-x:auto;color:#aaa;font-family:'SF Mono',Consolas,monospace;font-size:12px;white-space:pre-wrap;max-height:300px;overflow-y:auto">{logs}</pre>
  </details>
</div>
"#, logs = escape_js_string(logs))
    } else {
        String::new()
    };

    let html = format!(r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Nova 启动遇到问题</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{margin:0;padding:48px 24px;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC','Microsoft YaHei',sans-serif;background:linear-gradient(135deg,#1a1a2e 0%,#16213e 100%);color:#eee;min-height:100vh;display:flex;justify-content:center}}
.container{{max-width:520px;width:100%}}
.icon{{width:56px;height:56px;margin-bottom:24px;background:linear-gradient(135deg,#ff6b6b 0%,#ee5a5a 100%);border-radius:16px;display:flex;align-items:center;justify-content:center;font-size:28px}}
h1{{color:#fff;font-size:22px;font-weight:600;margin-bottom:12px}}
.message{{color:#aaa;font-size:15px;line-height:1.7;margin-bottom:24px}}
.tips{{background:rgba(255,255,255,.05);border-radius:12px;padding:20px}}
.tips h3{{color:#4ecdc4;font-size:13px;font-weight:600;text-transform:uppercase;letter-spacing:.5px;margin-bottom:12px}}
.tips ul{{list-style:none;padding:0;margin:0}}
.tips li{{color:#ccc;font-size:14px;line-height:1.8;padding-left:20px;position:relative}}
.tips li::before{{content:"•";color:#4ecdc4;position:absolute;left:0}}
details summary::-webkit-details-marker{{display:none}}
details[open] summary span:last-child{{transform:rotate(90deg)}}
</style>
</head>
<body>
<div class="container">
  <div class="icon">!</div>
  <h1>{title}</h1>
  <div class="message">{user_message}</div>
  <div class="tips">
    <h3>您可以尝试</h3>
    <ul>
      <li>重启应用程序</li>
      <li>重启电脑后再次尝试打开</li>
      <li>确认应用程序已正确安装，未被移动或删除</li>
      <li>macOS 用户：如提示"无法验证开发者"，请右键点击应用图标，按住 Option 键选择"打开"</li>
      <li>Windows 用户：如 SmartScreen 提示，请点击"更多信息"然后选择"仍要运行"</li>
    </ul>
  </div>
  {logs_section}
</div>
</body>
</html>"#,
        title = escape_js_string(title),
        user_message = escape_js_string(user_message),
        logs_section = logs_section
    );

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(html.as_bytes());
    let url = format!("data:text/html;base64,{}", b64);
    let _ = window.eval(&format!("window.location.replace('{}');", url.replace('\'', "%27")));
    let _ = window.show();
}

fn set_window_status(window: &tauri::WebviewWindow, text: &str) {
    let text = escape_js_string(text);
    let js = format!(r#"
        (function() {{
            var el = document.getElementById('status');
            if (el) el.textContent = '{}';
        }})();
    "#, text);
    let _ = window.eval(&js);
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window");

            navigate_to_loading(&window);
            let _ = window.show();

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

            set_window_status(&window, "正在启动...");

            let mut args = vec!["--desktop".to_string()];
            if let Some(ws) = get_workspace_from_args(app) {
                args.push("--workspace".to_string());
                args.push(ws);
            }

            let sidecar_result = app.shell().sidecar("nova");
            let sidecar_command = match sidecar_result {
                Ok(cmd) => cmd,
                Err(e) => {
                    eprintln!("[tauri] Failed to locate sidecar: {}", e);
                    show_user_error(
                        &window,
                        "应用文件不完整",
                        "Nova 的核心组件未找到，可能是安装包损坏或安装不完整。请重新下载安装。",
                        Some(&format!("Sidecar 定位失败: {}", e))
                    );
                    return Ok(());
                }
            };

            if let Ok(path_ref) = sidecar_command.program() {
                fix_sidecar_permissions(path_ref.as_ref());
            }

            set_window_status(&window, "正在初始化服务...");

            let (mut rx, child) = match sidecar_command
                .args(args)
                .spawn()
            {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("[tauri] Failed to spawn sidecar: {}", e);
                    let err_str = e.to_string();
                    let (title, user_msg) = if err_str.contains("permission denied") || err_str.contains("Permission denied") {
                        ("无法启动应用", "没有权限启动核心服务。请尝试重新安装应用，或检查安全软件设置。")
                    } else if err_str.contains("No such file") || err_str.contains("not found") || err_str.contains("NotFound") {
                        ("应用文件不完整", "核心组件缺失，请重新下载并安装应用。")
                    } else {
                        ("无法启动应用", "启动核心服务时遇到问题，请尝试重启应用或重新安装。如问题持续存在，请联系开发者。")
                    };
                    show_user_error(
                        &window,
                        title,
                        user_msg,
                        Some(&format!("启动进程失败: {}", e))
                    );
                    return Ok(());
                }
            };

            app.manage(SidecarProcess(Mutex::new(Some(child))));

            let window_clone = window.clone();
            tauri::async_runtime::spawn(async move {
                let port_re = Regex::new(r"\[nova-desktop-ready\] port=(\d+)").unwrap();
                let mut found_port: Option<u16> = None;
                let mut startup_log = String::new();

                for attempt in 0..180 {
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
                                CommandEvent::Error(err) => {
                                    eprintln!("[nova] Error: {:?}", err);
                                    startup_log.push_str(&format!("[error] {:?}\n", err));
                                }
                                CommandEvent::Terminated(status) => {
                                    eprintln!("[nova] Process terminated: {:?}", status);
                                    show_user_error(
                                        &window_clone,
                                        "服务启动失败",
                                        "应用核心进程意外退出。这可能是由于安全软件阻止或系统环境问题。请尝试重启电脑后重试，如问题持续存在请重新安装。",
                                        Some(&format!("{}\n退出状态: {:?}", startup_log, status))
                                    );
                                    return;
                                }
                                _ => {}
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_millis(500)) => {
                            if found_port.is_some() {
                                break;
                            }
                            let elapsed = (attempt + 1) / 2;
                            if elapsed == 2 {
                                set_window_status(&window_clone, "正在加载工作区...");
                            } else if elapsed == 5 {
                                set_window_status(&window_clone, "正在初始化服务...");
                            } else if elapsed == 10 {
                                set_window_status(&window_clone, "即将就绪...");
                            }
                            if elapsed % 15 == 0 && elapsed > 0 {
                                println!("[tauri] Still waiting for backend... ({}s)", elapsed);
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
                    println!("[tauri] No port in stdout, scanning...");
                    set_window_status(&window_clone, "正在连接...");

                    let client = reqwest::blocking::Client::builder()
                        .timeout(Duration::from_millis(500))
                        .build()
                        .unwrap();

                    let mut scanned = None;
                    for p in 8080..8400 {
                        if client.get(format!("http://127.0.0.1:{}", p)).send().is_ok() {
                            scanned = Some(p);
                            break;
                        }
                    }
                    match scanned {
                        Some(p) => {
                            println!("[tauri] Found backend on port {}", p);
                            p
                        }
                        None => {
                            show_user_error(
                                &window_clone,
                                "启动超时",
                                "应用启动时间过长。这可能是由于系统资源不足、安全软件阻止或网络问题。请尝试关闭其他应用后重试。",
                                Some(&startup_log)
                            );
                            return;
                        }
                    }
                };

                let url = format!("http://127.0.0.1:{}", port);
                println!("[tauri] Backend ready, navigating to {}", url);
                set_window_status(&window_clone, "正在加载界面...");

                tokio::time::sleep(Duration::from_millis(200)).await;

                window_clone.eval(&format!("window.location.replace('{}');", url)).ok();
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let state = window.state::<SidecarProcess>();
                let mut guard = state.0.lock().unwrap();
                if let Some(child) = guard.take() {
                    println!("[tauri] Window closing, killing nova process");
                    let _ = child.kill();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
