//! The timeout must cover the response body, not just the headers.
//!
//! A mock transport cannot test this: it answers instantly, so it would pass
//! whether the timeout wrapped the whole exchange or only the connect. This
//! needs a real socket that answers promptly and then stops talking — which is
//! exactly how a loaded server fails, and the case a client is most likely to
//! get wrong by hanging forever.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};

use typesafe_sdk_error::Error;
use typesafe_sdk_headers::Headers;
use typesafe_sdk_http::{Method, Request, Reqwest, Transport as _};

/// Promises this many bytes and sends fewer, so the body never completes.
const PROMISED: usize = 4096;

/// Serves one request, then behaves as `finish` dictates.
fn serve_once(finish: impl FnOnce(&mut TcpStream) + Send + 'static) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut scratch = [0_u8; 1024];
        let _ = stream.read(&mut scratch);
        finish(&mut stream);
    });
    format!("http://{addr}/v1/models")
}

fn request(url: String, timeout_ms: u64) -> Request {
    Request {
        method: Method::Get,
        url,
        headers: Headers::new(),
        body: None,
        timeout_ms,
    }
}

/// Headers arrive at once; the body then stops partway and never finishes.
#[tokio::test]
async fn a_body_that_stalls_after_the_headers_times_out() {
    let url = serve_once(|stream| {
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {PROMISED}\r\n\r\n");
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(b"{\"models\":[");
        let _ = stream.flush();
        // Hold the connection open without completing the body.
        std::thread::sleep(std::time::Duration::from_secs(30));
    });

    let started = std::time::Instant::now();
    let result = Reqwest::new().unwrap().send(request(url, 400)).await;

    match result {
        Err(Error::Timeout { timeout_ms }) => assert_eq!(timeout_ms, 400),
        other => panic!("expected a timeout, got {other:?}"),
    }
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "it waited far longer than the timeout: {:?}",
        started.elapsed()
    );
}

/// The control: the same server, answering completely, must not time out.
#[tokio::test]
async fn a_complete_response_within_the_timeout_succeeds() {
    let url = serve_once(|stream| {
        let body = "{\"models\":[]}";
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(body.as_bytes());
        let _ = stream.flush();
    });

    let response = Reqwest::new()
        .unwrap()
        .send(request(url, 5_000))
        .await
        .expect("a complete response must not time out");
    assert_eq!(response.status, 200);
    assert_eq!(response.body, "{\"models\":[]}");
}

/// A connection closed before any response is a connection error, not a
/// timeout: the client should report what happened, not wait for the clock.
#[tokio::test]
async fn a_connection_closed_early_is_not_reported_as_a_timeout() {
    let url = serve_once(|stream| {
        let _ = stream.shutdown(std::net::Shutdown::Both);
    });

    match Reqwest::new().unwrap().send(request(url, 5_000)).await {
        Err(error) => assert!(
            error.is_connection() && !matches!(error, Error::Timeout { .. }),
            "expected a connection error, got {error:?}"
        ),
        Ok(response) => panic!("expected a failure, got {}", response.status),
    }
}
