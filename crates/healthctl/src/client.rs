use anyhow::{Context, Result};
use healthctl_lib::ipc::{self, Request, Response, ResponseData};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use crate::daemon_ctl;
use crate::format;

/// Send a request to the daemon and return the response.
/// If the daemon isn't running, attempt to spawn it first.
pub fn send_request(request: Request) -> Result<Response> {
    let socket_path = ipc::socket_path();

    let stream = match UnixStream::connect(&socket_path) {
        Ok(s) => s,
        Err(_) => {
            tracing::info!("daemon not running, spawning...");
            daemon_ctl::spawn_daemon()?;
            // Wait briefly for the daemon to start listening.
            std::thread::sleep(Duration::from_millis(500));
            UnixStream::connect(&socket_path).context("failed to connect to daemon after spawn")?
        }
    };

    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut stream = stream;
    let payload = serde_json::to_string(&request)?;
    writeln!(stream, "{payload}")?;
    stream.flush()?;

    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    let response: Response =
        serde_json::from_str(&response_line).context("failed to parse daemon response")?;

    Ok(response)
}

/// Send a request without auto-spawning the daemon, with a short timeout.
///
/// Used by shell completion: if the daemon isn't already running we return an
/// error rather than paying the cost of spawning it (and blocking the user's
/// <TAB>). Callers should degrade gracefully (emit no candidates) on `Err`.
pub fn try_send_request(request: Request) -> Result<Response> {
    let socket_path = ipc::socket_path();
    let stream = UnixStream::connect(&socket_path).context("daemon not running")?;

    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    let mut stream = stream;
    let payload = serde_json::to_string(&request)?;
    writeln!(stream, "{payload}")?;
    stream.flush()?;

    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    let response: Response =
        serde_json::from_str(&response_line).context("failed to parse daemon response")?;
    Ok(response)
}

/// Print a response to stdout in a human-readable way.
pub fn print_response(response: Response) {
    match response {
        Response::Ok(data) => match data {
            ResponseData::Ack(_) => println!("ok"),
            ResponseData::Pong(_) => println!("pong"),
            ResponseData::Event(event) => {
                format::print_event_detail(&event);
            }
            ResponseData::Events(events) => {
                format::print_events_table(&events);
            }
            ResponseData::Summary(summary) => {
                format::print_status(&summary);
            }
            ResponseData::Report(report) => {
                format::print_report(&report, &[]);
            }
            // Completion payloads are consumed by the `__complete` path, not
            // printed for humans; emit one item per line as a sane fallback.
            ResponseData::Completions(candidates) => {
                for c in candidates {
                    println!("{}\t{}", c.short_id, c.description);
                }
            }
            ResponseData::Tags(tags) => {
                for tag in tags {
                    println!("{tag}");
                }
            }
        },
        Response::Error { message } => {
            eprintln!("{}: {message}", owo_colors::OwoColorize::red(&"error"));
            std::process::exit(1);
        }
    }
}

/// Print a brief event summary for deletion confirmation.
pub fn print_event_summary_for_delete(event: &healthctl_lib::event::Event) {
    format::print_event_delete_confirm(event);
}

/// Print a single event in full detail.
pub fn print_event_detail(response: Response) -> Result<()> {
    match response {
        Response::Ok(ResponseData::Event(event)) => {
            format::print_event_detail(&event);
            Ok(())
        }
        Response::Error { message } => {
            anyhow::bail!("{message}");
        }
        _ => anyhow::bail!("unexpected response from daemon"),
    }
}
