mod cli;
mod client;
mod daemon_ctl;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = cli::parse();
    match args.command {
        cli::Command::Add(add_cmd) => {
            let event = cli::build_event(add_cmd)?;
            let response = client::send_request(healthctl_lib::ipc::Request::Add(event))?;
            client::print_response(response);
        }
        cli::Command::Clone(clone_cmd) => {
            let request = healthctl_lib::ipc::Request::Clone {
                source_id: clone_cmd.event_id,
                overrides: clone_cmd.to_overrides()?,
            };
            let response = client::send_request(request)?;
            client::print_response(response);
        }
        cli::Command::List(list_cmd) => {
            let filter = list_cmd.to_filter()?;
            let response = client::send_request(healthctl_lib::ipc::Request::List(filter))?;
            client::print_response(response);
        }
        cli::Command::Edit { event_id } => {
            let response = client::send_request(healthctl_lib::ipc::Request::Get { id: event_id })?;
            match response {
                healthctl_lib::ipc::Response::Ok(healthctl_lib::ipc::ResponseData::Event(
                    event,
                )) => {
                    let updated = cli::edit_event(event)?;
                    let response =
                        client::send_request(healthctl_lib::ipc::Request::Update(updated))?;
                    client::print_response(response);
                }
                healthctl_lib::ipc::Response::Error { message } => {
                    anyhow::bail!("daemon error: {message}");
                }
                _ => anyhow::bail!("unexpected response from daemon"),
            }
        }
        cli::Command::Status => {
            let response = client::send_request(healthctl_lib::ipc::Request::Status)?;
            client::print_response(response);
        }
        cli::Command::Report { period } => {
            let period = cli::parse_report_period(&period)?;
            let response = client::send_request(healthctl_lib::ipc::Request::Report { period })?;
            client::print_response(response);
        }
        cli::Command::Daemon(daemon_cmd) => {
            daemon_ctl::handle(daemon_cmd)?;
        }
    }

    Ok(())
}
