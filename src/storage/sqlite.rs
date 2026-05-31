use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use rusqlite::Connection;

use super::migrations::MIGRATIONS;

#[derive(Debug, Clone)]
pub struct SqliteStore {
    path: PathBuf,
}

impl SqliteStore {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create sqlite directory {}", parent.display())
                })?;
            }
        }
        let store = Self { path };
        store.health_check()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn migrate(&self) -> anyhow::Result<()> {
        let conn = self.open_connection()?;
        for migration in MIGRATIONS {
            conn.execute_batch(migration)
                .context("failed to run sqlite migration")?;
        }
        Ok(())
    }

    pub fn health_check(&self) -> anyhow::Result<()> {
        let conn = self.open_connection()?;
        conn.query_row("SELECT 1", [], |_row| Ok(()))
            .context("sqlite health check failed")?;
        Ok(())
    }

    pub fn with_connection<T, F>(&self, op: F) -> anyhow::Result<T>
    where
        F: FnOnce(&Connection) -> anyhow::Result<T>,
    {
        let conn = self.open_connection()?;
        op(&conn)
    }

    fn open_connection(&self) -> anyhow::Result<Connection> {
        Connection::open(&self.path)
            .with_context(|| format!("failed to open sqlite {}", self.path.display()))
    }
}
