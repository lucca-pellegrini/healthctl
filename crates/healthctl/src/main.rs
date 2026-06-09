mod cli;
mod client;
mod complete;
mod daemon_ctl;
mod dashboard_ctl;
mod format;

use anyhow::Result;
use healthctl_lib::ipc::{Request, Response, ResponseData};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = cli::parse();
    match args.command {
        cli::Command::Add(add_cmd) => {
            let event = cli::build_event(add_cmd)?;
            let response = client::send_request(Request::Add(event))?;
            client::print_response(response);
        }
        cli::Command::Show { event_id } => {
            let request = resolve_get_request(&event_id);
            let response = client::send_request(request)?;
            client::print_event_detail(response)?;
        }
        cli::Command::Clone(clone_cmd) => {
            let request = Request::Clone {
                source_id: clone_cmd.event_id,
                overrides: clone_cmd.to_overrides()?,
            };
            let response = client::send_request(request)?;
            client::print_response(response);
        }
        cli::Command::List(list_cmd) => {
            let filter = list_cmd.to_filter()?;
            let response = client::send_request(Request::List(filter))?;
            client::print_response(response);
        }
        cli::Command::Edit { event_id } => {
            let request = resolve_get_request(&event_id);
            let response = client::send_request(request)?;
            match response {
                Response::Ok(ResponseData::Event(event)) => {
                    let updated = cli::edit_event(event)?;
                    let response = client::send_request(Request::Update(updated))?;
                    client::print_response(response);
                }
                Response::Error { message } => {
                    anyhow::bail!("{message}");
                }
                _ => anyhow::bail!("unexpected response from daemon"),
            }
        }
        cli::Command::Remove { event_id, yes } => {
            // First fetch the event to show what will be deleted
            let get_request = resolve_get_request(&event_id);
            let get_response = client::send_request(get_request)?;
            match get_response {
                Response::Ok(ResponseData::Event(event)) => {
                    // Show what will be deleted
                    client::print_event_summary_for_delete(&event);

                    // Confirm unless -y flag was passed
                    if !yes {
                        print!("Delete this event? [y/N] ");
                        std::io::Write::flush(&mut std::io::stdout())?;
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        if !input.trim().eq_ignore_ascii_case("y") {
                            println!("Cancelled.");
                            return Ok(());
                        }
                    }

                    // Perform the deletion
                    let delete_request = resolve_delete_request(&event_id);
                    let response = client::send_request(delete_request)?;
                    match response {
                        Response::Ok(ResponseData::Ack) => {
                            println!("Deleted event {}", &event.id.to_string()[..8]);
                        }
                        Response::Error { message } => {
                            anyhow::bail!("{message}");
                        }
                        _ => anyhow::bail!("unexpected response from daemon"),
                    }
                }
                Response::Error { message } => {
                    anyhow::bail!("{message}");
                }
                _ => anyhow::bail!("unexpected response from daemon"),
            }
        }
        cli::Command::Status => {
            let response = client::send_request(Request::Status)?;
            client::print_response(response);
        }
        cli::Command::Report(report_cmd) => {
            let period = cli::parse_report_period(&report_cmd.period)?;
            let cards = report_cmd.selected_cards();
            let response = client::send_request(Request::Report { period })?;
            match response {
                Response::Ok(ResponseData::Report(report)) => {
                    format::print_report(&report, &cards);
                }
                Response::Error { message } => anyhow::bail!("{message}"),
                _ => anyhow::bail!("unexpected response from daemon"),
            }
        }
        cli::Command::Daemon(daemon_cmd) => {
            daemon_ctl::handle(daemon_cmd)?;
        }
        cli::Command::Dashboard => {
            dashboard_ctl::handle()?;
        }
        cli::Command::Complete(complete_cmd) => {
            complete::run(complete_cmd.kind);
        }
    }

    Ok(())
}

/// If the event_id parses as a full UUID, use Get; otherwise use GetByPrefix.
fn resolve_get_request(event_id: &str) -> Request {
    match uuid::Uuid::parse_str(event_id) {
        Ok(id) => Request::Get { id },
        Err(_) => Request::GetByPrefix {
            prefix: event_id.to_string(),
        },
    }
}

/// If the event_id parses as a full UUID, use Delete; otherwise use DeleteByPrefix.
fn resolve_delete_request(event_id: &str) -> Request {
    match uuid::Uuid::parse_str(event_id) {
        Ok(id) => Request::Delete { id },
        Err(_) => Request::DeleteByPrefix {
            prefix: event_id.to_string(),
        },
    }
}
