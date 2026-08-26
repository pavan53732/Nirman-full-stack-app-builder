//! Durable SQLite ledger for Nirman commands, events, and projections.

#![forbid(unsafe_code)]

use nirman_domain::{ControlEvent, ProjectId, ProjectionSnapshot, Revision};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

#[derive(Debug)]
pub struct Ledger {
    connection: Connection,
}

impl Ledger {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        let ledger = Self { connection };
        ledger.migrate()?;
        Ok(ledger)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let connection = Connection::open_in_memory()?;
        let ledger = Self { connection };
        ledger.migrate()?;
        Ok(ledger)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        self.connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS events (
                 sequence INTEGER PRIMARY KEY,
                 event_id TEXT NOT NULL UNIQUE,
                 project_id TEXT NOT NULL,
                 task_id TEXT,
                 kind TEXT NOT NULL,
                 payload TEXT NOT NULL,
                 source_revision INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS projections (
                 project_id TEXT PRIMARY KEY,
                 projection_revision INTEGER NOT NULL,
                 task_state TEXT NOT NULL,
                 continuity_state TEXT NOT NULL,
                 preview_truth TEXT NOT NULL,
                 source_revision INTEGER NOT NULL,
                 last_event_sequence INTEGER NOT NULL,
                 last_known_good_ref TEXT
             );",
        )
    }

    pub fn append_event(&self, event: &ControlEvent) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO events (sequence, event_id, project_id, task_id, kind, payload, source_revision)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.sequence,
                event.event_id,
                event.project_id.0,
                event.task_id.as_ref().map(|id| id.0.as_str()),
                event.kind,
                event.payload,
                event.source_revision.0,
            ],
        )?;
        Ok(())
    }

    pub fn event_count(&self) -> rusqlite::Result<u64> {
        self.connection
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
    }

    pub fn latest_sequence(&self) -> rusqlite::Result<u64> {
        self.connection
            .query_row("SELECT COALESCE(MAX(sequence), 0) FROM events", [], |row| {
                row.get(0)
            })
    }

    pub fn save_projection(&self, snapshot: &ProjectionSnapshot) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO projections (project_id, projection_revision, task_state, continuity_state, preview_truth, source_revision, last_event_sequence, last_known_good_ref)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(project_id) DO UPDATE SET
               projection_revision = excluded.projection_revision,
               task_state = excluded.task_state,
               continuity_state = excluded.continuity_state,
               preview_truth = excluded.preview_truth,
               source_revision = excluded.source_revision,
               last_event_sequence = excluded.last_event_sequence,
               last_known_good_ref = excluded.last_known_good_ref",
            params![
                snapshot.project_id.0,
                snapshot.projection_revision.0,
                format!("{:?}", snapshot.task_state),
                format!("{:?}", snapshot.continuity_state),
                format!("{:?}", snapshot.preview_truth),
                snapshot.current_source_revision.0,
                snapshot.last_event_sequence,
                snapshot.last_known_good_ref,
            ],
        )?;
        Ok(())
    }

    pub fn projection_revision(
        &self,
        project_id: &ProjectId,
    ) -> rusqlite::Result<Option<Revision>> {
        self.connection
            .query_row(
                "SELECT projection_revision FROM projections WHERE project_id = ?1",
                params![project_id.0],
                |row| row.get::<_, u64>(0).map(Revision),
            )
            .optional()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nirman_domain::{BackgroundContinuityState, PreviewTruth, ProductLifecycleState};

    #[test]
    fn ledger_migrates_and_persists_event_and_projection() {
        let ledger = Ledger::open_in_memory().expect("ledger");
        let project_id = ProjectId("project-0001".into());
        let snapshot = ProjectionSnapshot {
            project_id: project_id.clone(),
            projection_revision: Revision(1),
            task_state: ProductLifecycleState::Planning,
            continuity_state: BackgroundContinuityState::ActiveBackground,
            preview_truth: PreviewTruth::Requested,
            current_source_revision: Revision(1),
            last_event_sequence: 1,
            last_known_good_ref: None,
        };
        let event = ControlEvent {
            event_id: "event-1".into(),
            sequence: 1,
            project_id: project_id.clone(),
            task_id: None,
            kind: "InstructionAccepted".into(),
            payload: "Build an Android app".into(),
            source_revision: Revision(1),
        };
        ledger.append_event(&event).expect("event");
        ledger.save_projection(&snapshot).expect("projection");
        assert_eq!(ledger.event_count().expect("count"), 1);
        assert_eq!(ledger.latest_sequence().expect("sequence"), 1);
        assert_eq!(
            ledger.projection_revision(&project_id).expect("revision"),
            Some(Revision(1))
        );
    }
}
