use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use crate::LMetrics;

fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut full = Vec::with_capacity(100);
    loop {
        std::thread::sleep(Duration::from_millis(10));
        let mut buffer = [0u8; 100];
        let len = stream.read(&mut buffer)?;
        if len == 0 {
            break;
        }
        let buffer = &buffer[..len];
        full.extend_from_slice(buffer);
        if &buffer[len - 4..] == b"\r\n\r\n" {
            break;
        }
    }
    Ok(String::from_utf8(full).unwrap())
}

fn respond_404() -> &'static str {
    "HTTP/1.1 404 Not found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\nnot found"
}

fn respond_200(data: String) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        data.as_bytes().len(),
        data
    )
}

fn respond_500() -> &'static str {
    "HTTP/1.1 500 Internal server error\r\nContent-Type: text/plain\r\nContent-Length: 21\r\n\r\ninternal server error"
}

pub trait HttpServer {
    type Error;
    fn process_http_request(&self, mut stream: TcpStream) -> std::result::Result<(), Self::Error>;
    fn accept(&self, listener: &mut std::net::TcpListener) -> std::result::Result<(), Self::Error>;
}

impl HttpServer for LMetrics {
    type Error = std::io::Error;

    fn process_http_request(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let request = read_request(&mut stream)?;
        if request.starts_with("GET /metrics") {
            let data = match self.encode_metrics() {
                Ok(data) => respond_200(data),
                Err(e) => {
                    log::error!("Error in encoding of metrics: {}", e);
                    respond_500().to_string()
                }
            };
            stream.write(data.as_bytes())?;
        } else {
            stream.write(respond_404().as_bytes())?;
        }
        Ok(())
    }

    fn accept(&self, listener: &mut std::net::TcpListener) -> std::io::Result<()> {
        match listener.accept() {
            Err(err) => match err.kind() {
                std::io::ErrorKind::WouldBlock => return Ok(()),
                _ => return Err(err),
            },
            Ok((stream, _)) => self.process_http_request(stream)?,
        }

        Ok(())
    }
}
