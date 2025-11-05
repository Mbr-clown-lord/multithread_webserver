use mt_webserver::threadclient;
use mt_webserver::threadpool;
use std::net::TcpListener;
use thiserror::Error;
use tracing::{error, info};
use tracing_appender::rolling;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Request parse error")]
    BadRequest,
}

fn main() {
    let file_appender = rolling::daily("logs", "server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    if let Err(e) = run_server() {
        let err = format!("Run Server error {}", e);
        error!(err);
    }
}

fn run_server() -> Result<(), ServerError> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr)?;

    println!("Server running...");
    info!("Server running...");

    let n_worker = 4;

    let pool = threadpool::ThreadPool::new(n_worker);

    for stream in listener.incoming() {
        let stream = stream?;

        pool.execute(move || {
            if let Err(e) = threadclient::handle_connection(stream) {
                let err = format!("Connection error {}", e);
                error!(err);
            }
        })
    }

    Ok(())
}
