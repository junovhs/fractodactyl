//! `fd-viewer` (VIEW-06): the explorer in its own desktop window. It starts the real
//! renderer, `fd explore`, on a free loopback port as a child process and shows its V2
//! page (the approved stone viewer, viewer/mockup) in a Dioxus 0.7 desktop window, set
//! up as ../ishoo's dashboard window is. The child holds this process's stdin pipe
//! (`--exit-with-stdin 1`), so it stops whenever the viewer does, however it exits.
//!
//! `--check` starts the renderer, fetches the page and one render, prints one line and
//! exits without opening a window. `--fd PATH` (or `FD_BIN`) picks the `fd` binary,
//! by default the one next to this executable.
// A desktop app: no console window on Windows release builds (--check prints nothing there).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use dioxus::desktop::{use_wry_event_handler, Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const USAGE: &str =
    "usage: fd-viewer [--check] [--fd PATH]\n\nManual: https://fractodactyl.dev/docs/";

/// The V2 page the window shows.
static URL: OnceLock<String> = OnceLock::new();
/// The renderer's end of the lifeline: held until this process exits.
static LIFELINE: OnceLock<std::sync::Mutex<(Child, ChildStdin)>> = OnceLock::new();

fn main() {
    if let Err(e) = run() {
        eprintln!("fd-viewer: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut check = false;
    let mut fd = std::env::var_os("FD_BIN").map(PathBuf::from);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            // SITE-11 (DEC-31): help points at the site's manual.
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            "--check" => check = true,
            "--fd" => fd = Some(args.next().ok_or("--fd needs a path")?.into()),
            _ => return Err(USAGE.into()),
        }
    }
    let fd = match fd {
        Some(f) => f,
        None => sibling_fd()?,
    };
    let port = free_port()?;
    let mut cmd = Command::new(&fd);
    cmd.args([
        "explore",
        "--port",
        &port.to_string(),
        "--exit-with-stdin",
        "1",
    ])
    .current_dir(home(&fd))
    .stdin(Stdio::piped())
    .stdout(Stdio::null());
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: the renderer gets no console window of its own.
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|e| format!("{}: {e}", fd.display()))?;
    let stdin = child.stdin.take().expect("piped stdin");
    wait_up(port, &mut child)?;
    let url = format!("http://127.0.0.1:{port}/v2/");
    if check {
        let page = get(port, "/v2/")?;
        if !page.contains("live-view") {
            return Err("/v2/ is not the viewer page".into());
        }
        let render = get_bytes(
            port,
            "/render?re=-0.75&im=0&width=3&w=64&h=36&ss=1&iter=200&gen=1",
        )?;
        println!(
            "fd-viewer: renderer {} on {url}: page {} bytes, 64x36 render {} bytes",
            fd.display(),
            page.len(),
            render.len()
        );
        let _ = child.kill();
        return if render.len() >= 64 * 36 * 3 {
            Ok(())
        } else {
            Err("the render came back short".into())
        };
    }
    let _ = URL.set(url);
    let _ = LIFELINE.set(std::sync::Mutex::new((child, stdin)));
    launch_window();
    Ok(())
}

/// The window, as ../ishoo's launch_dashboard sets its own up.
fn launch_window() {
    let icon = dioxus::desktop::tao::window::Icon::from_rgba(
        include_bytes!("../assets/icon-64.rgba").to_vec(),
        64,
        64,
    )
    .ok();
    let window = WindowBuilder::new()
        .with_title("Fractodactyl")
        .with_window_icon(icon)
        // The page's banner is the title bar (live.js wires it to `chrome`).
        .with_decorations(false)
        .with_resizable(true)
        .with_inner_size(LogicalSize::new(1280.0, 820.0))
        .with_min_inner_size(LogicalSize::new(800.0, 560.0));
    let cfg = Config::new()
        .with_window(window)
        .with_menu(None)
        .with_background_color((0x1b, 0x1e, 0x24, 0xff));
    dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

/// The window's content: a child webview on the renderer's page, filling the window
/// and following its size. Dioxus's own webview stays underneath, behind it.
#[component]
fn App() -> Element {
    let ctx = dioxus::desktop::window();
    let page = use_hook(|| {
        let url = URL.get().cloned().unwrap_or_default();
        let win = ctx.window.clone();
        let builder = wry::WebViewBuilder::new()
            .with_url(&url)
            .with_ipc_handler(move |req| chrome(&win, req.body()))
            .with_bounds(fill(ctx.window.inner_size()));
        // On Linux the page is a GTK widget in the window's own box, in place of Dioxus's
        // webview (hidden). As an X11 child window (build_as_child) it stopped receiving
        // the mouse wheel after the first click, so wheel zoom died after a pan (VIEW-09).
        #[cfg(target_os = "linux")]
        let built = {
            use dioxus::desktop::tao::platform::unix::WindowExtUnix;
            use wry::WebViewBuilderExtUnix;
            let _ = ctx.webview.set_visible(false);
            match ctx.window.default_vbox() {
                Some(vbox) => builder.build_gtk(vbox),
                None => builder.build_as_child(&*ctx.window),
            }
        };
        #[cfg(not(target_os = "linux"))]
        let built = builder.build_as_child(&*ctx.window);
        built.map(std::rc::Rc::new).map_err(|e| e.to_string())
    });
    let view = page.clone();
    use_wry_event_handler(move |event, _| {
        use dioxus::desktop::tao::event::{Event, WindowEvent};
        if let (
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            },
            Ok(v),
        ) = (event, &view)
        {
            let _ = v.set_bounds(fill(*size));
        }
    });
    match page {
        Ok(_) => rsx! {},
        Err(e) => rsx! {
            div { style: "font: 14px sans-serif; color: #ccc; padding: 2em;", "fd-viewer: {e}" }
        },
    }
}

/// The page's stone chrome acting on the frameless window (viewer/mockup/live.js).
fn chrome(win: &dioxus::desktop::tao::window::Window, message: &str) {
    use dioxus::desktop::tao::window::ResizeDirection as R;
    if std::env::var_os("FD_VIEWER_DEBUG").is_some() {
        eprintln!("fd-viewer: chrome {message}");
    }
    match message {
        "minimize" => win.set_minimized(true),
        "maximize" => win.set_maximized(!win.is_maximized()),
        // The renderer follows: it exits when its stdin pipe closes.
        "close" => std::process::exit(0),
        "drag" => drop(win.drag_window()),
        m => {
            let dir = match m.strip_prefix("resize:") {
                Some("n") => R::North,
                Some("s") => R::South,
                Some("e") => R::East,
                Some("w") => R::West,
                Some("ne") => R::NorthEast,
                Some("nw") => R::NorthWest,
                Some("se") => R::SouthEast,
                Some("sw") => R::SouthWest,
                _ => return,
            };
            let _ = win.drag_resize_window(dir);
        }
    }
}

/// The whole window, in physical pixels.
fn fill(size: dioxus::desktop::tao::dpi::PhysicalSize<u32>) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::LogicalPosition::new(0.0, 0.0).into(),
        size: wry::dpi::PhysicalSize::new(size.width, size.height).into(),
    }
}

/// `fd` next to this executable (cargo puts both in the same target directory).
fn sibling_fd() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let name = if cfg!(windows) { "fd.exe" } else { "fd" };
    let fd = exe.with_file_name(name);
    if fd.is_file() {
        Ok(fd)
    } else {
        Err(format!(
            "no {name} next to {}: build it (cargo build --release -p fd-cli) or pass --fd PATH",
            exe.display()
        ))
    }
}

/// The directory `fd explore` resolves looks/, places/ and data/zones/ in: the first
/// ancestor of the `fd` binary or the working directory that has a looks/ folder (the
/// repository), else the release package `fd` sits in (REL-01: its looks/ is beside it),
/// else the working directory.
fn home(fd: &Path) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let from_fd = fd.canonicalize().ok();
    let package = from_fd
        .as_deref()
        .and_then(Path::parent)
        .filter(|d| d.join("looks").is_dir())
        .map(Path::to_path_buf);
    [Some(cwd.clone()), from_fd]
        .into_iter()
        .flatten()
        .flat_map(|p| p.ancestors().map(Path::to_path_buf).collect::<Vec<_>>())
        .find(|d| d.join("looks").is_dir() && d.join("viewer").is_dir())
        .or(package)
        .unwrap_or(cwd)
}

fn free_port() -> Result<u16, String> {
    let l = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    l.local_addr().map(|a| a.port()).map_err(|e| e.to_string())
}

/// Wait until the renderer accepts connections (it may first load a zone).
fn wait_up(port: u16, child: &mut Child) -> Result<(), String> {
    let start = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("fd explore exited ({status}) before listening"));
        }
        if start.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            return Err(format!(
                "fd explore did not listen on port {port} within 60 s"
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

/// One HTTP/1.0 GET: the body if the status is 200.
fn get_bytes(port: u16, path: &str) -> Result<Vec<u8>, String> {
    let mut s = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    write!(s, "GET {path} HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let at = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("no HTTP header end")?;
    let head = String::from_utf8_lossy(&buf[..at]);
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        return Err(format!("GET {path}: {}", head.lines().next().unwrap_or("")));
    }
    Ok(buf[at + 4..].to_vec())
}

fn get(port: u16, path: &str) -> Result<String, String> {
    get_bytes(port, path).map(|b| String::from_utf8_lossy(&b).into_owned())
}
