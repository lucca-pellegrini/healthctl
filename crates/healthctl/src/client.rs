use anyhow::{Context, Result};
use healthctl_lib::ipc::{self, Request, Response, ResponseData};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use crate::daemon_ctl;

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

/// Print a response to stdout in a human-readable way.
pub fn print_response(response: Response) {
    match response {
        Response::Ok(data) => match data {
            ResponseData::Ack => println!("ok"),
            ResponseData::Pong => println!("pong"),
            ResponseData::Event(event) => {
                println!("{}", serde_json::to_string_pretty(&event).unwrap());
            }
            ResponseData::Events(events) => {
                if events.is_empty() {
                    println!("no events found");
                } else {
                    for event in &events {
                        print_event_summary(event);
                    }
                    println!("\n{} event(s) total", events.len());
                }
            }
            ResponseData::Summary(summary) => {
                println!("=== Today ===");
                println!("  Events:         {}", summary.today_events);
                println!("  Calories:       {:.0} kcal", summary.today_calories);
                println!("  Active time:    {:.0} min", summary.today_active_minutes);
                println!("  Streak:         {} days", summary.streak_days);
            }
            ResponseData::Report(report) => {
                println!("=== {:?} Report ===", report.period);
                println!("  Total events:       {}", report.total_events);
                println!("  Total calories:     {:.0} kcal", report.total_calories);
                println!(
                    "  Total active time:  {:.0} min",
                    report.total_active_minutes
                );
                println!(
                    "  Avg daily calories: {:.0} kcal",
                    report.avg_daily_calories
                );
                println!(
                    "  Avg daily active:   {:.0} min",
                    report.avg_daily_active_minutes
                );
            }
        },
        Response::Error { message } => {
            eprintln!("error: {message}");
            std::process::exit(1);
        }
    }
}

fn print_event_summary(event: &healthctl_lib::event::Event) {
    let time_str = event
        .start_time
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .or_else(|| {
            event
                .end_time
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        })
        .unwrap_or_else(|| "no time".into());

    let duration_str = event
        .duration_secs
        .map(|d| format_duration(d))
        .unwrap_or_default();

    let type_str = format!("{:?}", event.event_type);
    let tags_str = if event.tags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", event.tags.join(", "))
    };

    println!(
        "  {} | {} | {} | {}{}",
        &event.id.to_string()[..8],
        time_str,
        type_str,
        duration_str,
        tags_str,
    );
}

fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    if h > 0 {
        format!("{h}h{m:02}m")
    } else {
        format!("{m}m")
    }
}
