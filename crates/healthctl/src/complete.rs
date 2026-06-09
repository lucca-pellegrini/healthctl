//! Machine-readable output for the `__complete` hidden subcommand.
//!
//! Everything here is consumed by the generated `_healthctl` zsh completion
//! function — never by humans. The guiding principles are:
//!
//!   * **Never fail loudly.** If the daemon is down or anything goes wrong we
//!     simply emit nothing and exit 0, so the completion menu stays empty
//!     instead of dumping errors over the user's prompt.
//!   * **Stable, parse-friendly format.** Events are printed as
//!     `short_id<TAB>description` (one per line); tags are one per line. Any
//!     stray tabs/newlines in a description are scrubbed so each record stays
//!     on a single line with a single delimiter.

use crate::cli::CompleteKind;
use crate::client;
use healthctl_lib::ipc::{Request, Response, ResponseData};

pub fn run(kind: CompleteKind) {
    match kind {
        CompleteKind::Events { prefix } => complete_events(prefix),
        CompleteKind::Tags => complete_tags(),
    }
}

fn complete_events(prefix: Option<String>) {
    let prefix = prefix.filter(|p| !p.is_empty());
    let request = Request::CompleteEvents {
        prefix,
        limit: None,
    };

    if let Ok(Response::Ok(ResponseData::Completions(candidates))) =
        client::try_send_request(request)
    {
        let mut out = String::new();
        for c in candidates {
            out.push_str(&sanitize(&c.short_id));
            out.push('\t');
            out.push_str(&sanitize(&c.description));
            out.push('\n');
        }
        print!("{out}");
    }
    // On any error: emit nothing (graceful, empty completion).
}

fn complete_tags() {
    let request = Request::CompleteTags { limit: None };

    if let Ok(Response::Ok(ResponseData::Tags(tags))) = client::try_send_request(request) {
        let mut out = String::new();
        for tag in tags {
            out.push_str(&sanitize(&tag));
            out.push('\n');
        }
        print!("{out}");
    }
}

/// Replace any tab/newline/carriage-return with a space so a record never
/// spills across the single-line, tab-delimited format the zsh side expects.
fn sanitize(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}
