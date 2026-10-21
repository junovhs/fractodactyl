//! EXPL-01: `fd explore` serves its page and renders relief images over HTTP, skips a
//! request older than the newest generation, and refuses a bad view.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

fn get(port: u16, path: &str) -> (String, Vec<u8>) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut all = Vec::new();
    s.read_to_end(&mut all).unwrap();
    let at = all
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no header end")
        + 4;
    (
        String::from_utf8_lossy(&all[..at]).into_owned(),
        all[at..].to_vec(),
    )
}

fn post(port: u16, path: &str, body: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut all = String::new();
    s.read_to_string(&mut all).unwrap();
    all
}

#[test]
fn serves_page_and_renders() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let places = std::env::temp_dir().join(format!("fd-explore-places-{port}"));
    let _ = std::fs::remove_dir_all(&places);
    let child = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(["explore", "--port", &port.to_string(), "--threads", "2"])
        .env("FD_PLACES", &places)
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let _server = Server(child);
    let start = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "fd explore did not start"
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    let (head, body) = get(port, "/");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(String::from_utf8_lossy(&body).contains("<title>fd explore</title>"));

    let (head, body) = get(
        port,
        "/render?re=-0.65&im=0&width=4.2&w=64&h=36&ss=2&iter=500&gen=5",
    );
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(
        head.contains("X-Width: 64")
            && head.contains("X-Height: 36")
            && head.contains("kernel=pert-f64/1"),
        "{head}"
    );
    assert_eq!(body.len(), 64 * 36 * 3);
    assert!(
        body.contains(&0) && body.iter().any(|&b| b > 128),
        "expected both set and lit background"
    );

    // EXPL-03: raw shading inputs, four finite f32 per pixel, with set and background.
    let (head, body) = get(
        port,
        "/render?re=-0.65&im=0&width=4.2&w=64&h=36&ss=2&iter=500&gen=5&raw=1",
    );
    assert!(
        head.starts_with("HTTP/1.1 200") && head.contains("X-Width: 64"),
        "{head}"
    );
    assert_eq!(body.len(), 64 * 36 * 16);
    let f: Vec<f32> = body
        .chunks(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    assert!(f.iter().all(|v| v.is_finite()));
    let cover: Vec<f32> = f.chunks(4).map(|p| p[3]).collect();
    assert!(
        cover.contains(&0.0) && cover.contains(&1.0),
        "expected interior and escaped pixels"
    );
    assert!(f
        .chunks(4)
        .filter(|p| p[3] > 0.0)
        .all(|p| p[0] > 0.0 && (0.0..=1.0).contains(&p[1])));

    // EXPL-07: studio inputs for every sample (2x2 per pixel) from the navigation kernel.
    let (head, body) = get(
        port,
        "/render?re=-0.65&im=0&width=4.2&w=64&h=36&ss=2&iter=500&gen=5&raw=3",
    );
    assert!(
        head.starts_with("HTTP/1.1 200")
            && head.contains("kernel=pert-f64/1")
            && head.contains("nubase="),
        "{head}"
    );
    assert_eq!(body.len(), 128 * 72 * 16);
    let f: Vec<f32> = body
        .chunks(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    assert!(f.iter().all(|v| v.is_finite()));
    let class: Vec<f32> = f.chunks(4).map(|p| p[3]).collect();
    assert!(
        class.contains(&0.0) && class.contains(&1.0) && !class.contains(&2.0),
        "escaped and interior, unresolved shown as interior"
    );
    assert!(
        f.chunks(4)
            .filter(|p| p[3] == 0.0)
            .all(|p| p[0] >= 0.0 && p[1] >= 0.0),
        "nu re-based to the smallest escaped"
    );

    // Deep: the scaled tier at 1e-300.
    let (head, body) = get(
        port,
        "/render?re=0&im=1&width=1e-300&w=32&h=18&ss=1&iter=2000&gen=6",
    );
    assert!(
        head.starts_with("HTTP/1.1 200") && head.contains("pert-fx-scaled/1"),
        "{head}"
    );
    assert_eq!(body.len(), 32 * 18 * 3);

    // Older than generation 6: skipped, not rendered.
    let (head, body) = get(
        port,
        "/render?re=0&im=0&width=1&w=8&h=8&ss=1&iter=100&gen=4",
    );
    assert!(head.starts_with("HTTP/1.1 204"), "{head}");
    assert!(body.is_empty());

    let (head, _) = get(
        port,
        "/render?re=x&im=0&width=1&w=8&h=8&ss=1&iter=100&gen=7",
    );
    assert!(head.starts_with("HTTP/1.1 400"), "{head}");

    // EXPL-08: save a place (into $FD_PLACES), list it, refuse a bad one.
    let place = "re -0.75\nim 0.1\nwidth 1.5e-40\niter 30000\n";
    let head = post(port, "/place?name=spot-1", place);
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    let (head, body) = get(port, "/places");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert_eq!(
        String::from_utf8_lossy(&body),
        "spot-1\t-0.75\t0.1\t1.5e-40\t30000"
    );
    assert!(post(port, "/place?name=spot-2", "re 1x\nim 0\nwidth 1").starts_with("HTTP/1.1 400"));
    assert!(post(port, "/place?name=../evil", place).starts_with("HTTP/1.1 400"));
}

/// FIX-67: the viewer's real backend and film share runtime place storage.
#[test]
fn place_from_release_layout() {
    use std::path::PathBuf;
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let root = Temp(std::env::temp_dir().join(format!("fd-place-release-{}", std::process::id())));
    let package = root.0.join("package");
    let elsewhere = root.0.join("unrelated");
    std::fs::create_dir_all(package.join("places")).unwrap();
    std::fs::create_dir_all(&elsewhere).unwrap();
    let shipped = "re -0.5\nim 0\nwidth 2\niter 100\n";
    std::fs::write(package.join("places/shipped.place"), shipped).unwrap();
    let exe = package.join(format!("fd{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(env!("CARGO_BIN_EXE_fd"), &exe).unwrap();
    for mode in ["data", "home", "override"] {
        let data = root.0.join(mode).join("data");
        let home = root.0.join(mode).join("home");
        let override_dir = root.0.join(mode).join("override");
        let expected = match mode {
            "override" => override_dir.clone(),
            "home" if !cfg!(windows) => home.join(".local/share/fractodactyl/places"),
            _ => data.join("fractodactyl/places"),
        };
        let command = || {
            let mut c = Command::new(&exe);
            c.current_dir(&elsewhere)
                .env_remove("FD_PLACES")
                .env("HOME", &home)
                .env("APPDATA", &data)
                .env_remove("XDG_DATA_HOME");
            if mode != "home" {
                c.env("XDG_DATA_HOME", &data);
            }
            if mode == "override" {
                c.env("FD_PLACES", &override_dir);
            }
            c
        };
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let child = command()
            .args(["explore", "--port", &port.to_string(), "--threads", "1"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let mut server = Server(child);
        let start = Instant::now();
        while TcpStream::connect(("127.0.0.1", port)).is_err() {
            assert!(start.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(30));
        }
        // Check before save: the red version must never write into its build checkout.
        let (head, body) = get(port, "/places");
        assert!(head.starts_with("HTTP/1.1 200"));
        assert!(
            String::from_utf8_lossy(&body).contains("shipped\t-0.5\t0\t2\t100"),
            "release package place missing: {}",
            String::from_utf8_lossy(&body)
        );
        assert!(
            !expected.exists(),
            "listing must not create personal storage"
        );
        let saved = "re -0.75\nim 0.1\nwidth 1\niter 100\n";
        let response = post(port, "/place?name=shipped", saved);
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert_eq!(
            std::fs::read_to_string(package.join("places/shipped.place")).unwrap(),
            shipped
        );
        let text = std::fs::read_to_string(expected.join("shipped.place")).unwrap();
        assert!(text.contains(saved), "{text}");
        let (_, body) = get(port, "/places");
        assert_eq!(
            String::from_utf8_lossy(&body),
            "shipped\t-0.75\t0.1\t1\t100"
        );
        let movie = root.0.join(format!("{mode}.mp4"));
        let film = command()
            .args([
                "film",
                "--place",
                "shipped",
                "--from",
                "4",
                "--seconds",
                "1",
                "--fps",
                "2",
                "--size",
                "16x16",
                "--threads",
                "1",
                "--ss",
                "1",
                "--keyframes",
                "off",
                "--look",
                "umber",
                "--x264",
                "ultrafast",
                "--frames",
                "0",
                "--mp4",
            ])
            .arg(&movie)
            .output()
            .unwrap();
        assert!(
            film.status.success(),
            "{}",
            String::from_utf8_lossy(&film.stderr)
        );
        assert!(std::fs::metadata(&movie).unwrap().len() > 0);
        assert!(String::from_utf8_lossy(&film.stdout).contains("\"schema\":\"fd-film/1\""));
        eprintln!(
            "{mode}: saved/listed/read {}, film --place shipped --frames 0 exit {}",
            expected.display(),
            film.status
        );
        server.0.kill().unwrap();
        server.0.wait().unwrap();
    }
}
