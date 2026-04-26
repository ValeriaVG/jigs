use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

pub struct HttpRequest {
    pub method: String,
    pub path: String,
}

pub struct HttpResponse {
    pub status: u16,
    pub reason: &'static str,
    pub body: String,
}

impl HttpResponse {
    pub fn ok(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            reason: "OK",
            body: body.into(),
        }
    }
    pub fn bad_request(body: impl Into<String>) -> Self {
        Self {
            status: 400,
            reason: "Bad Request",
            body: body.into(),
        }
    }
    pub fn not_found(body: impl Into<String>) -> Self {
        Self {
            status: 404,
            reason: "Not Found",
            body: body.into(),
        }
    }
    pub fn method_not_allowed() -> Self {
        Self {
            status: 405,
            reason: "Method Not Allowed",
            body: "method not allowed".into(),
        }
    }
}

pub fn read_request(stream: &mut TcpStream) -> std::io::Result<HttpRequest> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();

    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf)?;
        if n == 0 || buf == "\r\n" || buf == "\n" {
            break;
        }
    }
    Ok(HttpRequest { method, path })
}

pub fn write_response(stream: &mut TcpStream, resp: &HttpResponse) -> std::io::Result<()> {
    let body = resp.body.as_bytes();
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        resp.status,
        resp.reason,
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

#[allow(dead_code)]
pub fn drain<R: Read>(mut r: R) {
    let mut sink = Vec::new();
    let _ = r.read_to_end(&mut sink);
}
