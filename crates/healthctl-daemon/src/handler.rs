use healthctl_lib::event::Event;
use healthctl_lib::ipc::{ReportPeriod, Request, Response, ResponseData};
use healthctl_lib::validate::validate_event;
use uuid::Uuid;

use crate::db::Database;

pub async fn handle_request(request: Request, db: &Database) -> Response {
    match request {
        Request::Add(event) => handle_add(event, db).await,
        Request::Clone {
            source_id,
            overrides,
        } => handle_clone(source_id, overrides, db).await,
        Request::Get { id } => handle_get(id, db).await,
        Request::Update(event) => handle_update(event, db).await,
        Request::List(filter) => handle_list(filter, db).await,
        Request::Status => handle_status(db).await,
        Request::Report { period } => handle_report(period, db).await,
        Request::Shutdown => {
            // The shutdown signal is handled by the main loop; just ack here.
            Response::Ok(ResponseData::Ack)
        }
        Request::Ping => Response::Ok(ResponseData::Pong),
    }
}

async fn handle_add(event: Event, db: &Database) -> Response {
    if let Err(e) = validate_event(&event) {
        return Response::Error {
            message: e.to_string(),
        };
    }

    match db.insert_event(&event).await {
        Ok(()) => {
            tracing::info!(id = %event.id, event_type = ?event.event_type, "event added");
            Response::Ok(ResponseData::Event(event))
        }
        Err(e) => Response::Error {
            message: format!("database error: {e}"),
        },
    }
}

async fn handle_clone(source_id: Uuid, overrides: serde_json::Value, db: &Database) -> Response {
    let source = match db.get_event(source_id).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            return Response::Error {
                message: format!("event {source_id} not found"),
            };
        }
        Err(e) => {
            return Response::Error {
                message: format!("database error: {e}"),
            };
        }
    };

    // Serialize source to JSON, merge overrides, deserialize back.
    let mut source_json = serde_json::to_value(&source).unwrap();
    let mut duration_override: Option<f64> = None;

    if let (Some(base), Some(patch)) = (source_json.as_object_mut(), overrides.as_object()) {
        for (key, value) in patch {
            if key == "_duration_secs" {
                // Special: duration override needs to be resolved into start/end.
                duration_override = value.as_f64();
            } else if key == "metrics" {
                // Merge metrics rather than replacing entirely.
                if let (Some(base_metrics), Some(patch_metrics)) = (
                    base.get_mut("metrics").and_then(|v| v.as_object_mut()),
                    value.as_object(),
                ) {
                    for (k, v) in patch_metrics {
                        base_metrics.insert(k.clone(), v.clone());
                    }
                }
            } else {
                base.insert(key.clone(), value.clone());
            }
        }
    }

    // Give it a new ID.
    source_json["id"] = serde_json::Value::String(Uuid::new_v4().to_string());
    source_json["created_at"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());

    let mut cloned: Event = match serde_json::from_value(source_json) {
        Ok(e) => e,
        Err(e) => {
            return Response::Error {
                message: format!("failed to apply overrides: {e}"),
            };
        }
    };

    // Resolve times if duration was overridden.
    if duration_override.is_some() {
        cloned.resolve_times(duration_override);
    }

    if let Err(e) = validate_event(&cloned) {
        return Response::Error {
            message: e.to_string(),
        };
    }

    match db.insert_event(&cloned).await {
        Ok(()) => {
            tracing::info!(id = %cloned.id, source = %source_id, "event cloned");
            Response::Ok(ResponseData::Event(cloned))
        }
        Err(e) => Response::Error {
            message: format!("database error: {e}"),
        },
    }
}

async fn handle_get(id: Uuid, db: &Database) -> Response {
    match db.get_event(id).await {
        Ok(Some(event)) => Response::Ok(ResponseData::Event(event)),
        Ok(None) => Response::Error {
            message: format!("event {id} not found"),
        },
        Err(e) => Response::Error {
            message: format!("database error: {e}"),
        },
    }
}

async fn handle_update(event: Event, db: &Database) -> Response {
    if let Err(e) = validate_event(&event) {
        return Response::Error {
            message: e.to_string(),
        };
    }

    match db.update_event(&event).await {
        Ok(()) => {
            tracing::info!(id = %event.id, "event updated");
            Response::Ok(ResponseData::Event(event))
        }
        Err(e) => Response::Error {
            message: format!("database error: {e}"),
        },
    }
}

async fn handle_list(filter: healthctl_lib::ipc::ListFilter, db: &Database) -> Response {
    match db.list_events(&filter).await {
        Ok(events) => Response::Ok(ResponseData::Events(events)),
        Err(e) => Response::Error {
            message: format!("database error: {e}"),
        },
    }
}

async fn handle_status(db: &Database) -> Response {
    match db.get_status_summary().await {
        Ok(summary) => Response::Ok(ResponseData::Summary(summary)),
        Err(e) => Response::Error {
            message: format!("database error: {e}"),
        },
    }
}

async fn handle_report(period: ReportPeriod, db: &Database) -> Response {
    match db.get_report(&period).await {
        Ok(report) => Response::Ok(ResponseData::Report(report)),
        Err(e) => Response::Error {
            message: format!("database error: {e}"),
        },
    }
}
