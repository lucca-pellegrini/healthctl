use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use healthctl_lib::event::{Event, EventType, Exercise};
use healthctl_lib::ipc::ListFilter;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn open() -> Result<Self> {
        let db_path = Self::db_path();

        // Ensure parent directory exists.
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let options =
            SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=rwc", db_path.display()))?
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    fn db_path() -> std::path::PathBuf {
        if let Some(data_dir) = directories::ProjectDirs::from("", "", "healthctl") {
            data_dir.data_dir().join("healthctl.db")
        } else {
            std::path::PathBuf::from("healthctl.db")
        }
    }

    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                start_time TEXT,
                end_time TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS event_metrics (
                event_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value REAL NOT NULL,
                PRIMARY KEY (event_id, key),
                FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS event_tags (
                event_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (event_id, tag),
                FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS event_exercises (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id TEXT NOT NULL,
                name TEXT NOT NULL,
                sets INTEGER,
                reps INTEGER,
                weight_kg REAL,
                FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
            CREATE INDEX IF NOT EXISTS idx_events_start ON events(start_time);
            CREATE INDEX IF NOT EXISTS idx_events_end ON events(end_time);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_event(&self, event: &Event) -> Result<()> {
        let id = event.id.to_string();
        let event_type = serde_json::to_string(&event.event_type)?;
        let start_time = event.start_time.map(|t| t.to_rfc3339());
        let end_time = event.end_time.map(|t| t.to_rfc3339());
        let created_at = event.created_at.to_rfc3339();

        sqlx::query(
            "INSERT INTO events (id, event_type, start_time, end_time, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&event_type)
        .bind(&start_time)
        .bind(&end_time)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;

        // Insert metrics.
        for (key, value) in &event.metrics {
            sqlx::query("INSERT INTO event_metrics (event_id, key, value) VALUES (?, ?, ?)")
                .bind(&id)
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await?;
        }

        // Insert tags.
        for tag in &event.tags {
            sqlx::query("INSERT INTO event_tags (event_id, tag) VALUES (?, ?)")
                .bind(&id)
                .bind(tag)
                .execute(&self.pool)
                .await?;
        }

        // Insert exercises.
        for exercise in &event.exercises {
            sqlx::query(
                "INSERT INTO event_exercises (event_id, name, sets, reps, weight_kg)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&exercise.name)
            .bind(exercise.sets.map(|v| v as i64))
            .bind(exercise.reps.map(|v| v as i64))
            .bind(exercise.weight_kg)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn get_event(&self, id: Uuid) -> Result<Option<Event>> {
        let id_str = id.to_string();

        let row = sqlx::query_as::<_, EventRow>(
            "SELECT id, event_type, start_time, end_time, created_at
             FROM events WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let mut event = row.into_event()?;
                self.load_event_details(&mut event).await?;
                Ok(Some(event))
            }
            None => Ok(None),
        }
    }

    pub async fn get_event_by_prefix(&self, prefix: &str) -> Result<Option<Event>> {
        let pattern = format!("{prefix}%");

        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT id, event_type, start_time, end_time, created_at
             FROM events WHERE id LIKE ? LIMIT 2",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        match rows.len() {
            0 => Ok(None),
            1 => {
                let mut event = rows.into_iter().next().unwrap().into_event()?;
                self.load_event_details(&mut event).await?;
                Ok(Some(event))
            }
            _ => anyhow::bail!("prefix '{prefix}' is ambiguous (matches multiple events)"),
        }
    }

    pub async fn update_event(&self, event: &Event) -> Result<()> {
        let id = event.id.to_string();

        // Delete old related data.
        sqlx::query("DELETE FROM event_metrics WHERE event_id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM event_tags WHERE event_id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM event_exercises WHERE event_id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await?;

        // Update main row.
        let event_type = serde_json::to_string(&event.event_type)?;
        let start_time = event.start_time.map(|t| t.to_rfc3339());
        let end_time = event.end_time.map(|t| t.to_rfc3339());

        sqlx::query(
            "UPDATE events SET event_type = ?, start_time = ?, end_time = ?
             WHERE id = ?",
        )
        .bind(&event_type)
        .bind(&start_time)
        .bind(&end_time)
        .bind(&id)
        .execute(&self.pool)
        .await?;

        // Re-insert metrics, tags, exercises.
        for (key, value) in &event.metrics {
            sqlx::query("INSERT INTO event_metrics (event_id, key, value) VALUES (?, ?, ?)")
                .bind(&id)
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await?;
        }
        for tag in &event.tags {
            sqlx::query("INSERT INTO event_tags (event_id, tag) VALUES (?, ?)")
                .bind(&id)
                .bind(tag)
                .execute(&self.pool)
                .await?;
        }
        for exercise in &event.exercises {
            sqlx::query(
                "INSERT INTO event_exercises (event_id, name, sets, reps, weight_kg)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&exercise.name)
            .bind(exercise.sets.map(|v| v as i64))
            .bind(exercise.reps.map(|v| v as i64))
            .bind(exercise.weight_kg)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn list_events(&self, filter: &ListFilter) -> Result<Vec<Event>> {
        let mut sql = String::from(
            "SELECT id, event_type, start_time, end_time, created_at FROM events WHERE 1=1",
        );
        let mut binds: Vec<String> = Vec::new();

        if let Some(ref event_type) = filter.event_type {
            // Match on the JSON-serialized event_type containing the category.
            sql.push_str(" AND event_type LIKE ?");
            binds.push(format!("%{event_type}%"));
        }
        if let Some(from) = filter.from {
            sql.push_str(" AND (start_time >= ? OR end_time >= ?)");
            let ts = from.to_rfc3339();
            binds.push(ts.clone());
            binds.push(ts);
        }
        if let Some(to) = filter.to {
            sql.push_str(" AND (start_time <= ? OR end_time <= ?)");
            let ts = to.to_rfc3339();
            binds.push(ts.clone());
            binds.push(ts);
        }

        sql.push_str(" ORDER BY COALESCE(start_time, end_time, created_at) DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        } else {
            sql.push_str(" LIMIT 100");
        }

        let mut query = sqlx::query_as::<_, EventRow>(&sql);
        for bind in &binds {
            query = query.bind(bind);
        }

        let rows = query.fetch_all(&self.pool).await?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let mut event = row.into_event()?;
            self.load_event_details(&mut event).await?;

            // Post-filter by tags if needed.
            if !filter.tags.is_empty() {
                if !filter.tags.iter().all(|t| event.tags.contains(t)) {
                    continue;
                }
            }

            events.push(event);
        }

        Ok(events)
    }

    pub async fn get_status_summary(&self) -> Result<healthctl_lib::ipc::StatusSummary> {
        let today_start = chrono::Local::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let today_start_utc: DateTime<Utc> = chrono::Local
            .from_local_datetime(&today_start)
            .single()
            .unwrap()
            .with_timezone(&Utc);
        let ts = today_start_utc.to_rfc3339();

        let today_events: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM events WHERE COALESCE(start_time, end_time, created_at) >= ?",
        )
        .bind(&ts)
        .fetch_one(&self.pool)
        .await?;

        let today_calories: (f64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(em.value), 0.0) FROM event_metrics em
             JOIN events e ON em.event_id = e.id
             WHERE em.key = 'calories_kcal'
             AND COALESCE(e.start_time, e.end_time, e.created_at) >= ?",
        )
        .bind(&ts)
        .fetch_one(&self.pool)
        .await?;

        let today_active: (f64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(
                (julianday(end_time) - julianday(start_time)) * 86400.0
             ), 0.0) FROM events
             WHERE event_type NOT LIKE '%sleep%'
             AND start_time IS NOT NULL AND end_time IS NOT NULL
             AND COALESCE(start_time, end_time, created_at) >= ?",
        )
        .bind(&ts)
        .fetch_one(&self.pool)
        .await?;

        let week_start = today_start_utc - chrono::Duration::days(7);
        let ws = week_start.to_rfc3339();
        let week_events: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM events WHERE COALESCE(start_time, end_time, created_at) >= ?",
        )
        .bind(&ws)
        .fetch_one(&self.pool)
        .await?;

        Ok(healthctl_lib::ipc::StatusSummary {
            today_events: today_events.0 as u32,
            today_calories: today_calories.0,
            today_active_minutes: today_active.0 / 60.0,
            week_events: week_events.0 as u32,
            streak_days: 0, // TODO: calculate streak
        })
    }

    pub async fn get_report(
        &self,
        period: &healthctl_lib::ipc::ReportPeriod,
    ) -> Result<healthctl_lib::ipc::ReportData> {
        let now = Utc::now();
        let days = match period {
            healthctl_lib::ipc::ReportPeriod::Day => 1,
            healthctl_lib::ipc::ReportPeriod::Week => 7,
            healthctl_lib::ipc::ReportPeriod::Month => 30,
            healthctl_lib::ipc::ReportPeriod::Year => 365,
        };
        let from = now - chrono::Duration::days(days);
        let ts = from.to_rfc3339();

        let total_events: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM events WHERE COALESCE(start_time, end_time, created_at) >= ?",
        )
        .bind(&ts)
        .fetch_one(&self.pool)
        .await?;

        let total_calories: (f64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(em.value), 0.0) FROM event_metrics em
             JOIN events e ON em.event_id = e.id
             WHERE em.key = 'calories_kcal'
             AND COALESCE(e.start_time, e.end_time, e.created_at) >= ?",
        )
        .bind(&ts)
        .fetch_one(&self.pool)
        .await?;

        let total_active: (f64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(
                (julianday(end_time) - julianday(start_time)) * 86400.0
             ), 0.0) FROM events
             WHERE event_type NOT LIKE '%sleep%'
             AND start_time IS NOT NULL AND end_time IS NOT NULL
             AND COALESCE(start_time, end_time, created_at) >= ?",
        )
        .bind(&ts)
        .fetch_one(&self.pool)
        .await?;

        Ok(healthctl_lib::ipc::ReportData {
            period: period.clone(),
            total_events: total_events.0 as u32,
            total_calories: total_calories.0,
            total_active_minutes: total_active.0 / 60.0,
            avg_daily_calories: total_calories.0 / days as f64,
            avg_daily_active_minutes: (total_active.0 / 60.0) / days as f64,
        })
    }

    async fn load_event_details(&self, event: &mut Event) -> Result<()> {
        let id = event.id.to_string();

        // Load metrics.
        let metrics: Vec<(String, f64)> =
            sqlx::query_as("SELECT key, value FROM event_metrics WHERE event_id = ?")
                .bind(&id)
                .fetch_all(&self.pool)
                .await?;
        event.metrics = metrics.into_iter().collect();

        // Load tags.
        let tags: Vec<(String,)> = sqlx::query_as("SELECT tag FROM event_tags WHERE event_id = ?")
            .bind(&id)
            .fetch_all(&self.pool)
            .await?;
        event.tags = tags.into_iter().map(|(t,)| t).collect();

        // Load exercises.
        let exercises: Vec<ExerciseRow> = sqlx::query_as(
            "SELECT name, sets, reps, weight_kg FROM event_exercises WHERE event_id = ?",
        )
        .bind(&id)
        .fetch_all(&self.pool)
        .await?;
        event.exercises = exercises.into_iter().map(|r| r.into_exercise()).collect();

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: String,
    event_type: String,
    start_time: Option<String>,
    end_time: Option<String>,
    created_at: String,
}

impl EventRow {
    fn into_event(self) -> Result<Event> {
        let id = Uuid::parse_str(&self.id)?;
        let event_type: EventType = serde_json::from_str(&self.event_type)?;
        let start_time = self
            .start_time
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()?;
        let end_time = self
            .end_time
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()?;
        let created_at = DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&Utc);

        Ok(Event {
            id,
            event_type,
            start_time,
            end_time,
            metrics: HashMap::new(),
            tags: Vec::new(),
            exercises: Vec::new(),
            created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ExerciseRow {
    name: String,
    sets: Option<i64>,
    reps: Option<i64>,
    weight_kg: Option<f64>,
}

impl ExerciseRow {
    fn into_exercise(self) -> Exercise {
        Exercise {
            name: self.name,
            sets: self.sets.map(|v| v as u32),
            reps: self.reps.map(|v| v as u32),
            weight_kg: self.weight_kg,
        }
    }
}
