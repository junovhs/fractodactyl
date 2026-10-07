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

#[test]
fn serves_page_and_renders() {
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let child = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(["explore", "--port", &port.to_string(), "--threads", "2"])
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
}
