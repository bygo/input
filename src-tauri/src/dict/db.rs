use super::path::resolve_db_path;
use super::types::DictDb;
use rusqlite::Connection;
use std::path::Path;
use tauri::{AppHandle, Manager};

pub(super) fn open_at(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("打开词库失败: {e}"))
}

pub(super) fn open_db() -> Result<Connection, String> {
    open_at(&resolve_db_path()?)
}

pub(super) fn reset_conn(app: &AppHandle) {
    if let Ok(mut g) = app.state::<DictDb>().conn.lock() {
        *g = None;
    }
}

pub(super) fn with_db<T>(
    app: &AppHandle,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let state = app.state::<DictDb>();
    let mut guard = state.conn.lock().map_err(|_| "词库锁失败".to_string())?;
    if guard.is_none() {
        *guard = Some(open_db()?);
    }
    f(guard.as_ref().unwrap())
}
