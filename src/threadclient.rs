use memmap2::Mmap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::Shutdown::Both;
use std::net::TcpStream;
use thiserror::Error;
use tracing::{Level, info, span};

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Request parse error")]
    BadRequest,
}

pub fn handle_connection(stream: TcpStream) -> Result<(), ServerError> {
    let per_addr = stream.peer_addr()?;
    let client_ip = per_addr.ip();

    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);

    let mut request_line = String::new();

    reader.read_line(&mut request_line)?;

    let (method, path) = parse_request(&request_line)?;

    let span = span! {
        Level::INFO,
        "http",
        method=%method.trim(),
        path=%path.trim(),
        client_ip=%client_ip
    };
    let _enter = span.enter();

    info!("Request");

    if method == "GET" {
        let (status_line, filepath) = if path == "/" {
            ("HTTP/1.1 200 OK\r\n", "index.html")
        } else {
            ("HTTP/1.1 404 NOT FOUND\r\n", "404.html")
        };

        let file = File::open(filepath)?;
        let nmap = unsafe { Mmap::map(&file)? };

        let header = format!(
            "{}Content-Length: {}\r\nContent-Type: text/html\r\n\r\n",
            status_line,
            nmap.len(),
        );

        writer.write_all(header.as_bytes())?;
        writer.write_all(&nmap)?;
        writer.flush()?;

        info!(status_line=%status_line.trim(),"Reponse");

        stream.shutdown(Both)?;
    }

    Ok(())
}

fn parse_request(request: &str) -> Result<(&str, &str), ServerError> {
    let mut line = request.lines();
    let request_line = line.next().ok_or(ServerError::BadRequest);

    let mut part = request_line?.split_whitespace();

    let method = part.next().ok_or(ServerError::BadRequest);
    let path = part.next().ok_or(ServerError::BadRequest);

    Ok((method?, path?))
}
