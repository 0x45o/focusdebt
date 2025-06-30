use rusqlite::{Connection, Result as SqliteResult, OptionalExtension};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::time::Duration;
use std::path::PathBuf;
use dirs;

use crate::tracking::{FocusSession, ContextSwitch};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> SqliteResult<Self> {
        let db_path = Self::get_db_path()?;
        let conn = Connection::open(db_path)?;
        
        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS focus_sessions (
                id INTEGER PRIMARY KEY,
                start_time TEXT NOT NULL,
                end_time TEXT,
                app_name TEXT NOT NULL,
                window_title TEXT NOT NULL,
                duration_seconds INTEGER NOT NULL,
                is_focus_app BOOLEAN NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS context_switches (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                from_app TEXT NOT NULL,
                to_app TEXT NOT NULL,
                recovery_time_seconds INTEGER
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS focus_apps (
                id INTEGER PRIMARY KEY,
                app_name TEXT UNIQUE NOT NULL,
                added_at TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Database { conn })
    }

    fn get_db_path() -> SqliteResult<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| rusqlite::Error::InvalidPath("Could not find data directory".into()))?;
        
        let focusdebt_dir = data_dir.join("focusdebt");
        std::fs::create_dir_all(&focusdebt_dir)
            .map_err(|e| rusqlite::Error::InvalidPath(format!("Failed to create directory: {}", e).into()))?;
        
        Ok(focusdebt_dir.join("focusdebt.db"))
    }

    pub fn save_focus_session(&self, session: &FocusSession) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT INTO focus_sessions (start_time, end_time, app_name, window_title, duration_seconds, is_focus_app)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &session.start_time.to_rfc3339(),
                &session.end_time.as_ref().map(|t| t.to_rfc3339()),
                &session.app_name,
                &session.window_title,
                session.duration.as_secs() as i64,
                session.is_focus_app,
            ),
        )?;
        Ok(())
    }

    pub fn save_context_switch(&self, switch: &ContextSwitch) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT INTO context_switches (timestamp, from_app, to_app, recovery_time_seconds)
             VALUES (?1, ?2, ?3, ?4)",
            (
                &switch.timestamp.to_rfc3339(),
                &switch.from_app,
                &switch.to_app,
                &switch.recovery_time.map(|d| d.as_secs() as i64),
            ),
        )?;
        Ok(())
    }

    pub fn add_focus_app(&self, app_name: &str) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO focus_apps (app_name, added_at) VALUES (?1, ?2)",
            (app_name, &Utc::now().to_rfc3339()),
        )?;
        Ok(())
    }

    pub fn remove_focus_app(&self, app_name: &str) -> SqliteResult<()> {
        self.conn.execute(
            "DELETE FROM focus_apps WHERE app_name = ?1",
            (app_name,),
        )?;
        Ok(())
    }

    pub fn get_focus_apps(&self) -> SqliteResult<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT app_name FROM focus_apps ORDER BY app_name")?;
        let app_iter = stmt.query_map([], |row| {
            Ok(row.get(0)?)
        })?;

        let mut apps = Vec::new();
        for app in app_iter {
            apps.push(app?);
        }
        Ok(apps)
    }

    pub fn get_deep_focus_sessions(&self, min_duration_seconds: u64, date: DateTime<Utc>) -> SqliteResult<Vec<FocusSession>> {
        let start_of_day = date.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date.date_naive().and_hms_opt(23, 59, 59).unwrap();
        
        let start_str = DateTime::<Utc>::from_naive_utc_and_offset(start_of_day, Utc).to_rfc3339();
        let end_str = DateTime::<Utc>::from_naive_utc_and_offset(end_of_day, Utc).to_rfc3339();
        let min_duration_str = (min_duration_seconds as i64).to_string();

        let mut stmt = self.conn.prepare(
            "SELECT start_time, end_time, app_name, window_title, duration_seconds, is_focus_app
             FROM focus_sessions 
             WHERE start_time >= ?1 AND start_time <= ?2 
             AND is_focus_app = 1 
             AND duration_seconds >= ?3
             ORDER BY duration_seconds DESC"
        )?;

        let session_iter = stmt.query_map([&start_str, &end_str, &min_duration_str], |row| {
            let start_time: String = row.get(0)?;
            let end_time: Option<String> = row.get(1)?;
            let app_name: String = row.get(2)?;
            let window_title: String = row.get(3)?;
            let duration_seconds: i64 = row.get(4)?;
            let is_focus_app: bool = row.get(5)?;

            let start_time = DateTime::parse_from_rfc3339(&start_time)
                .map_err(|_| rusqlite::Error::InvalidParameterName("Invalid start_time".into()))?
                .with_timezone(&Utc);

            let end_time = end_time
                .map(|t| DateTime::parse_from_rfc3339(&t)
                    .map_err(|_| rusqlite::Error::InvalidParameterName("Invalid end_time".into()))
                    .map(|dt| dt.with_timezone(&Utc)))
                .transpose()?;

            Ok(FocusSession {
                start_time,
                end_time,
                app_name,
                window_title,
                duration: Duration::from_secs(duration_seconds as u64),
                is_focus_app,
            })
        })?;

        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(session?);
        }
        Ok(sessions)
    }

    pub fn get_average_recovery_time(&self, date: DateTime<Utc>) -> SqliteResult<Option<Duration>> {
        let start_of_day = date.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date.date_naive().and_hms_opt(23, 59, 59).unwrap();
        
        let start_str = DateTime::<Utc>::from_naive_utc_and_offset(start_of_day, Utc).to_rfc3339();
        let end_str = DateTime::<Utc>::from_naive_utc_and_offset(end_of_day, Utc).to_rfc3339();

        let mut stmt = self.conn.prepare(
            "SELECT AVG(recovery_time_seconds) 
             FROM context_switches 
             WHERE timestamp >= ?1 AND timestamp <= ?2 
             AND recovery_time_seconds IS NOT NULL"
        )?;

        let result: Option<i64> = stmt.query_row([&start_str, &end_str], |row| {
            Ok(row.get(0)?)
        }).optional()?;

        Ok(result.map(|seconds| Duration::from_secs(seconds as u64)))
    }

    pub fn get_most_distracting_apps(&self, date: DateTime<Utc>, limit: usize) -> SqliteResult<Vec<(String, Duration)>> {
        let start_of_day = date.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date.date_naive().and_hms_opt(23, 59, 59).unwrap();
        
        let start_str = DateTime::<Utc>::from_naive_utc_and_offset(start_of_day, Utc).to_rfc3339();
        let end_str = DateTime::<Utc>::from_naive_utc_and_offset(end_of_day, Utc).to_rfc3339();
        let limit_str = (limit as i64).to_string();

        let mut stmt = self.conn.prepare(
            "SELECT app_name, SUM(duration_seconds) as total_duration
             FROM focus_sessions 
             WHERE start_time >= ?1 AND start_time <= ?2 
             AND is_focus_app = 0
             GROUP BY app_name 
             ORDER BY total_duration DESC 
             LIMIT ?3"
        )?;

        let app_iter = stmt.query_map([&start_str, &end_str, &limit_str], |row| {
            let app_name: String = row.get(0)?;
            let duration_seconds: i64 = row.get(1)?;
            Ok((app_name, Duration::from_secs(duration_seconds as u64)))
        })?;

        let mut apps = Vec::new();
        for app in app_iter {
            apps.push(app?);
        }
        Ok(apps)
    }

    pub fn get_sessions_for_date(&self, date: DateTime<Utc>) -> SqliteResult<Vec<FocusSession>> {
        let start_of_day = date.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date.date_naive().and_hms_opt(23, 59, 59).unwrap();
        
        let start_str = DateTime::<Utc>::from_naive_utc_and_offset(start_of_day, Utc).to_rfc3339();
        let end_str = DateTime::<Utc>::from_naive_utc_and_offset(end_of_day, Utc).to_rfc3339();

        let mut stmt = self.conn.prepare(
            "SELECT start_time, end_time, app_name, window_title, duration_seconds, is_focus_app
             FROM focus_sessions 
             WHERE start_time >= ?1 AND start_time <= ?2
             ORDER BY start_time"
        )?;

        let session_iter = stmt.query_map([&start_str, &end_str], |row| {
            let start_time: String = row.get(0)?;
            let end_time: Option<String> = row.get(1)?;
            let app_name: String = row.get(2)?;
            let window_title: String = row.get(3)?;
            let duration_seconds: i64 = row.get(4)?;
            let is_focus_app: bool = row.get(5)?;

            let start_time = DateTime::parse_from_rfc3339(&start_time)
                .map_err(|_| rusqlite::Error::InvalidParameterName("Invalid start_time".into()))?
                .with_timezone(&Utc);

            let end_time = end_time
                .map(|t| DateTime::parse_from_rfc3339(&t)
                    .map_err(|_| rusqlite::Error::InvalidParameterName("Invalid end_time".into()))
                    .map(|dt| dt.with_timezone(&Utc)))
                .transpose()?;

            Ok(FocusSession {
                start_time,
                end_time,
                app_name,
                window_title,
                duration: Duration::from_secs(duration_seconds as u64),
                is_focus_app,
            })
        })?;

        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(session?);
        }
        Ok(sessions)
    }

    pub fn get_context_switches_for_date(&self, date: DateTime<Utc>) -> SqliteResult<Vec<ContextSwitch>> {
        let start_of_day = date.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date.date_naive().and_hms_opt(23, 59, 59).unwrap();
        
        let start_str = DateTime::<Utc>::from_naive_utc_and_offset(start_of_day, Utc).to_rfc3339();
        let end_str = DateTime::<Utc>::from_naive_utc_and_offset(end_of_day, Utc).to_rfc3339();

        let mut stmt = self.conn.prepare(
            "SELECT timestamp, from_app, to_app, recovery_time_seconds
             FROM context_switches 
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp"
        )?;

        let switch_iter = stmt.query_map([&start_str, &end_str], |row| {
            let timestamp: String = row.get(0)?;
            let from_app: String = row.get(1)?;
            let to_app: String = row.get(2)?;
            let recovery_time_seconds: Option<i64> = row.get(3)?;

            let timestamp = DateTime::parse_from_rfc3339(&timestamp)
                .map_err(|_| rusqlite::Error::InvalidParameterName("Invalid timestamp".into()))?
                .with_timezone(&Utc);

            let recovery_time = recovery_time_seconds.map(|s| Duration::from_secs(s as u64));

            Ok(ContextSwitch {
                timestamp,
                from_app,
                to_app,
                recovery_time,
            })
        })?;

        let mut switches = Vec::new();
        for switch in switch_iter {
            switches.push(switch?);
        }
        Ok(switches)
    }
} 