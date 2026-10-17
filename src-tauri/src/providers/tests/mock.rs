//! A tiny scripted HTTP server for tests (std only): each connection gets the next reply.
//! Streams are written chunk by chunk; a "hold" keeps the connection open with keep-alive
//! comments until the client hangs up, which is recorded (for the cancel test).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    chunks: Vec<Vec<u8>>,
    gap: Duration,
    /// Keep the stream open this long after the chunks (unless the client leaves).
    hold: Duration,
    /// Send the headers, then nothing at all for this long.
    stall: Duration,
    sse: bool,
}

impl Reply {
    /// An SSE stream, sent one event per chunk.
    pub fn sse(stream: &str) -> Self {
        let chunks = stream
            .split_inclusive("\n\n")
            .map(|part| part.as_bytes().to_vec())
            .collect();
        Self {
            status: 200,
            headers: vec![("content-type".into(), "text/event-stream".into())],
            chunks,
            gap: Duration::from_millis(2),
            hold: Duration::ZERO,
            stall: Duration::ZERO,
            sse: true,
        }
    }

    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: vec![("content-type".into(), "application/json".into())],
            chunks: vec![body.as_bytes().to_vec()],
            gap: Duration::ZERO,
            hold: Duration::ZERO,
            stall: Duration::ZERO,
            sse: false,
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn hold(mut self, hold: Duration) -> Self {
        self.hold = hold;
        self
    }

    pub fn stall(mut self, stall: Duration) -> Self {
        self.stall = stall;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Recorded {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).expect("request body is JSON")
    }
}

pub struct MockServer {
    pub base: String,
    requests: Arc<Mutex<Vec<Recorded>>>,
    client_left: Arc<AtomicBool>,
}

impl MockServer {
    pub fn start(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let client_left = Arc::new(AtomicBool::new(false));
        let (log, left) = (requests.clone(), client_left.clone());
        std::thread::spawn(move || {
            for reply in replies {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                if let Some(request) = read_request(&stream) {
                    log.lock().expect("log").push(request);
                }
                if write_reply(stream, &reply).is_err() {
                    left.store(true, Ordering::SeqCst);
                }
            }
        });
        Self {
            base,
            requests,
            client_left,
        }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().expect("log").clone()
    }

    /// The client closed the connection while the server was still streaming.
    pub fn client_left(&self) -> bool {
        self.client_left.load(Ordering::SeqCst)
    }
}

/// A local port that refuses connections for as long as the socket lives: bound but never
/// listening. Binding a listener and dropping it would free the port, and a mock server of
/// a parallel test could be given it (then it would answer this test's request instead).
pub fn refusing_port() -> (socket2::Socket, u16) {
    use socket2::{Domain, Socket, Type};
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).expect("socket");
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().expect("addr");
    socket.bind(&addr.into()).expect("bind");
    let port = socket
        .local_addr()
        .ok()
        .and_then(|a| a.as_socket())
        .expect("bound address")
        .port();
    (socket, port)
}

fn read_request(stream: &TcpStream) -> Option<Recorded> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_owned();
    let path = parts.next()?.to_owned();
    let mut headers = HashMap::new();
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    let length = headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some(Recorded {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn write_reply(mut stream: TcpStream, reply: &Reply) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {} Mock\r\n", reply.status);
    for (name, value) in &reply.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if !reply.sse {
        let length: usize = reply.chunks.iter().map(Vec::len).sum();
        head.push_str(&format!("content-length: {length}\r\n"));
    }
    head.push_str("connection: close\r\n\r\n");
    stream.write_all(head.as_bytes())?;
    stream.flush()?;
    std::thread::sleep(reply.stall);
    for chunk in &reply.chunks {
        stream.write_all(chunk)?;
        stream.flush()?;
        std::thread::sleep(reply.gap);
    }
    let step = Duration::from_millis(20);
    let mut held = Duration::ZERO;
    while held < reply.hold {
        std::thread::sleep(step);
        held += step;
        stream.write_all(b": still here\n\n")?;
        stream.flush()?;
    }
    Ok(())
}
