use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
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
    pub fn created(body: impl Into<String>) -> Self {
        Self {
            status: 201,
            reason: "Created",
            body: body.into(),
        }
    }
    pub fn no_content() -> Self {
        Self {
            status: 204,
            reason: "No Content",
            body: String::new(),
        }
    }
    pub fn bad_request(b: impl Into<String>) -> Self {
        Self {
            status: 400,
            reason: "Bad Request",
            body: b.into(),
        }
    }
    pub fn unauthorized(b: impl Into<String>) -> Self {
        Self {
            status: 401,
            reason: "Unauthorized",
            body: b.into(),
        }
    }
    pub fn not_found(b: impl Into<String>) -> Self {
        Self {
            status: 404,
            reason: "Not Found",
            body: b.into(),
        }
    }
    pub fn conflict(b: impl Into<String>) -> Self {
        Self {
            status: 409,
            reason: "Conflict",
            body: b.into(),
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

    let mut headers = HashMap::new();
    loop {
        let mut buf = String::new();
        let n = reader.read_line(&mut buf)?;
        if n == 0 || buf == "\r\n" || buf == "\n" {
            break;
        }
        if let Some((k, v)) = buf.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let len: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body_buf = vec![0u8; len];
    if len > 0 {
        reader.read_exact(&mut body_buf)?;
    }
    let body = String::from_utf8(body_buf).unwrap_or_default();
    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

pub fn write_response(stream: &mut TcpStream, resp: &HttpResponse) -> std::io::Result<()> {
    let body = resp.body.as_bytes();
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        resp.status, resp.reason, body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
