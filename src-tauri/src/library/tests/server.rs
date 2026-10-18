//! A tiny routing HTTP server for the library tests (std only): replies by request path
//! (query included), each route answering its replies in turn and repeating the last one.
//! Requests are recorded; `If-None-Match` matching a route's ETag answers 304. Dropping
//! the server stops it, so later requests fail like being offline.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    pub fn status(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_ascii_lowercase(), value.into()));
        self
    }

    /// GitHub's rate limit headers.
    pub fn rate(self, remaining: u64, reset_secs: u64) -> Self {
        self.header("x-ratelimit-limit", "60")
            .header("x-ratelimit-remaining", &remaining.to_string())
            .header("x-ratelimit-reset", &reset_secs.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub path: String,
    pub headers: HashMap<String, String>,
}

type Routes = Arc<Mutex<HashMap<String, VecDeque<Reply>>>>;

pub struct Server {
    pub base: String,
    routes: Routes,
    log: Arc<Mutex<Vec<Request>>>,
    stop: Arc<AtomicBool>,
}

impl Server {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.set_nonblocking(true).expect("nonblocking");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        let routes: Routes = Arc::default();
        let log: Arc<Mutex<Vec<Request>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let (r, l, s) = (routes.clone(), log.clone(), stop.clone());
        std::thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let (r, l) = (r.clone(), l.clone());
                        std::thread::spawn(move || serve(stream, &r, &l));
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(2)),
                }
            }
        });
        Self {
            base,
            routes,
            log,
            stop,
        }
    }

    pub fn route(&self, path: &str, replies: Vec<Reply>) {
        self.routes
            .lock()
            .expect("routes")
            .insert(path.to_owned(), replies.into());
    }

    pub fn requests(&self) -> Vec<Request> {
        self.log.lock().expect("log").clone()
    }

    pub fn count(&self, prefix: &str) -> usize {
        self.requests()
            .iter()
            .filter(|r| r.path.starts_with(prefix))
            .count()
    }

    pub fn clear_log(&self) {
        self.log.lock().expect("log").clear();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn serve(stream: TcpStream, routes: &Routes, log: &Arc<Mutex<Vec<Request>>>) {
    let _ = stream.set_nonblocking(false);
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let path = line.split_whitespace().nth(1).unwrap_or("/").to_owned();
        let mut headers = HashMap::new();
        let mut length = 0usize;
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) == 0 || header.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                let name = name.trim().to_ascii_lowercase();
                if name == "content-length" {
                    length = value.trim().parse().unwrap_or(0);
                }
                headers.insert(name, value.trim().to_owned());
            }
        }
        let mut body = vec![0; length];
        let _ = reader.read_exact(&mut body);
        log.lock().expect("log").push(Request {
            path: path.clone(),
            headers: headers.clone(),
        });
        let reply = {
            let mut routes = routes.lock().expect("routes");
            match routes.get_mut(&path) {
                Some(queue) if queue.len() > 1 => queue.pop_front(),
                Some(queue) => queue.front().cloned(),
                None => None,
            }
        }
        .unwrap_or_else(|| Reply::status(404, "{\"message\":\"Not Found\"}"));
        let etag = reply
            .headers
            .iter()
            .find(|(n, _)| n == "etag")
            .map(|(_, v)| v.clone());
        let not_modified = etag.is_some() && headers.get("if-none-match") == etag.as_ref();
        let (status, body) = if not_modified {
            (304, Vec::new())
        } else {
            (reply.status, reply.body.clone())
        };
        let mut head = format!(
            "HTTP/1.1 {status} X\r\ncontent-length: {}\r\nconnection: keep-alive\r\n",
            body.len()
        );
        for (name, value) in &reply.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        let mut stream = reader.get_ref();
        if stream.write_all(head.as_bytes()).is_err() || stream.write_all(&body).is_err() {
            return;
        }
    }
}
