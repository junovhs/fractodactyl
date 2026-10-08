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
    let at = all.windows(4).position(|w| w == b"\r\n\r\n").expect("no header end") + 4;
    (String::from_utf8_lossy(&all[..at]).into_owned(), all[at..].to_vec())
}

fn post(port: u16, path: &str, body: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut all = String::new();
    s.read_to_string(&mut all).unwrap();
    all
}

#[test]
fn serves_page_and_renders() {
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
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
        assert!(start.elapsed() < Duration::from_secs(10), "fd explore did not start");
        std::thread::sleep(Duration::from_millis(50));
    }

    let (head, body) = get(port, "/");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(String::from_utf8_lossy(&body).contains("<title>fd explore</title>"));

    let (head, body) = get(port, "/render?re=-0.65&im=0&width=4.2&w=64&h=36&ss=2&iter=500&gen=5");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(head.contains("X-Width: 64") && head.contains("X-Height: 36") && head.contains("kernel=pert-f64/1"), "{head}");
    assert_eq!(body.len(), 64 * 36 * 3);
    assert!(body.contains(&0) && body.iter().any(|&b| b > 128), "expected both set and lit background");

    // EXPL-03: raw shading inputs, four finite f32 per pixel, with set and background.
    let (head, body) = get(port, "/render?re=-0.65&im=0&width=4.2&w=64&h=36&ss=2&iter=500&gen=5&raw=1");
    assert!(head.starts_with("HTTP/1.1 200") && head.contains("X-Width: 64"), "{head}");
    assert_eq!(body.len(), 64 * 36 * 16);
    let f: Vec<f32> = body.chunks(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    assert!(f.iter().all(|v| v.is_finite()));
    let cover: Vec<f32> = f.chunks(4).map(|p| p[3]).collect();
    assert!(cover.contains(&0.0) && cover.contains(&1.0), "expected interior and escaped pixels");
    assert!(f.chunks(4).filter(|p| p[3] > 0.0).all(|p| p[0] > 0.0 && (0.0..=1.0).contains(&p[1])));

    // EXPL-07: studio inputs for every sample (2x2 per pixel) from the navigation kernel.
    let (head, body) = get(port, "/render?re=-0.65&im=0&width=4.2&w=64&h=36&ss=2&iter=500&gen=5&raw=3");
    assert!(head.starts_with("HTTP/1.1 200") && head.contains("kernel=pert-f64/1") && head.contains("nubase="), "{head}");
    assert_eq!(body.len(), 128 * 72 * 16);
    let f: Vec<f32> = body.chunks(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    assert!(f.iter().all(|v| v.is_finite()));
    let class: Vec<f32> = f.chunks(4).map(|p| p[3]).collect();
    assert!(class.contains(&0.0) && class.contains(&1.0) && !class.contains(&2.0), "escaped and interior, unresolved shown as interior");
    assert!(f.chunks(4).filter(|p| p[3] == 0.0).all(|p| p[0] >= 0.0 && p[1] >= 0.0), "nu re-based to the smallest escaped");

    // Deep: the scaled tier at 1e-300.
    let (head, body) = get(port, "/render?re=0&im=1&width=1e-300&w=32&h=18&ss=1&iter=2000&gen=6");
    assert!(head.starts_with("HTTP/1.1 200") && head.contains("pert-fx-scaled/1"), "{head}");
    assert_eq!(body.len(), 32 * 18 * 3);

    // Older than generation 6: skipped, not rendered.
    let (head, body) = get(port, "/render?re=0&im=0&width=1&w=8&h=8&ss=1&iter=100&gen=4");
    assert!(head.starts_with("HTTP/1.1 204"), "{head}");
    assert!(body.is_empty());

    let (head, _) = get(port, "/render?re=x&im=0&width=1&w=8&h=8&ss=1&iter=100&gen=7");
    assert!(head.starts_with("HTTP/1.1 400"), "{head}");

    // EXPL-08: save a place (into $FD_PLACES), list it, refuse a bad one.
    let place = "re -0.75\nim 0.1\nwidth 1.5e-40\niter 30000\n";
    let head = post(port, "/place?name=spot-1", place);
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    let (head, body) = get(port, "/places");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert_eq!(String::from_utf8_lossy(&body), "spot-1\t-0.75\t0.1\t1.5e-40\t30000");
    assert!(post(port, "/place?name=spot-2", "re 1x\nim 0\nwidth 1").starts_with("HTTP/1.1 400"));
    assert!(post(port, "/place?name=../evil", place).starts_with("HTTP/1.1 400"));
}
