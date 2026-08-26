//! Durable SQLite ledger for Nirman commands, events, and projections.

#![forbid(unsafe_code)]

use nirman_domain::{
    BackgroundContinuityState, ControlEvent, PreviewTruth, ProductLifecycleState, ProjectId,
    ProjectionSnapshot, Revision, TaskId,
};
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
             CREATE TABLE IF NOT EXISTS checkpoints (
                 checkpoint_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 projection_revision INTEGER NOT NULL,
                 source_revision INTEGER NOT NULL,
                 event_sequence INTEGER NOT NULL
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

    pub fn commit_event_and_projection(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
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
        transaction.execute(
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
        transaction.commit()
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

    pub fn save_checkpoint(
        &self,
        project_id: &ProjectId,
        checkpoint_id: &str,
        snapshot: &ProjectionSnapshot,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO checkpoints (checkpoint_id, project_id, projection_revision, source_revision, event_sequence)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(checkpoint_id) DO UPDATE SET
               projection_revision = excluded.projection_revision,
               source_revision = excluded.source_revision,
               event_sequence = excluded.event_sequence",
            params![
                checkpoint_id,
                project_id.0,
                snapshot.projection_revision.0,
                snapshot.current_source_revision.0,
                snapshot.last_event_sequence,
            ],
        )?;
        Ok(())
    }

    pub fn latest_checkpoint_id(&self, project_id: &ProjectId) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT checkpoint_id FROM checkpoints WHERE project_id = ?1 ORDER BY event_sequence DESC, checkpoint_id DESC LIMIT 1",
                params![project_id.0],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn load_projection(
        &self,
        project_id: &ProjectId,
    ) -> rusqlite::Result<Option<ProjectionSnapshot>> {
        let row = self
            .connection
            .query_row(
                "SELECT projection_revision, task_state, continuity_state, preview_truth, source_revision, last_event_sequence, last_known_good_ref
                 FROM projections WHERE project_id = ?1",
                params![project_id.0],
                |row| {
                    Ok((
                        row.get::<_, u64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, u64>(4)?,
                        row.get::<_, u64>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .optional()?;

        row.map(
            |(
                projection_revision,
                task_state,
                continuity_state,
                preview_truth,
                source_revision,
                last_event_sequence,
                last_known_good_ref,
            )| {
                Ok(ProjectionSnapshot {
                    project_id: project_id.clone(),
                    projection_revision: Revision(projection_revision),
                    task_state: parse_task_state(&task_state)?,
                    continuity_state: parse_continuity_state(&continuity_state)?,
                    preview_truth: parse_preview_truth(&preview_truth)?,
                    current_source_revision: Revision(source_revision),
                    last_event_sequence,
                    last_known_good_ref,
                })
            },
        )
        .transpose()
    }

    pub fn events_after(
        &self,
        project_id: &ProjectId,
        sequence: u64,
    ) -> rusqlite::Result<Vec<ControlEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT event_id, sequence, task_id, kind, payload, source_revision
             FROM events WHERE project_id = ?1 AND sequence > ?2 ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map(params![project_id.0, sequence], |row| {
            Ok(ControlEvent {
                event_id: row.get(0)?,
                sequence: row.get(1)?,
                project_id: project_id.clone(),
                task_id: row.get::<_, Option<String>>(2)?.map(TaskId),
                kind: row.get(3)?,
                payload: row.get(4)?,
                source_revision: Revision(row.get(5)?),
            })
        })?;
        rows.collect()
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

fn parse_task_state(value: &str) -> rusqlite::Result<ProductLifecycleState> {
    match value {
        "Created" => Ok(ProductLifecycleState::Created),
        "Planning" => Ok(ProductLifecycleState::Planning),
        "Implementing" => Ok(ProductLifecycleState::Implementing),
        "Paused" => Ok(ProductLifecycleState::Paused),
        "Previewing" => Ok(ProductLifecycleState::Previewing),
        "Validating" => Ok(ProductLifecycleState::Validating),
        "Recovering" => Ok(ProductLifecycleState::Recovering),
        "Packaging" => Ok(ProductLifecycleState::Packaging),
        "Completed" => Ok(ProductLifecycleState::Completed),
        "UserRequired" => Ok(ProductLifecycleState::UserRequired),
        "SafelyFailed" => Ok(ProductLifecycleState::SafelyFailed),
        "Cancelled" => Ok(ProductLifecycleState::Cancelled),
        other => Err(rusqlite::Error::InvalidParameterName(format!(
            "unknown task state: {other}"
        ))),
    }
}

fn parse_continuity_state(value: &str) -> rusqlite::Result<BackgroundContinuityState> {
    match value {
        "ActiveBackground" => Ok(BackgroundContinuityState::ActiveBackground),
        "UiDisconnected" => Ok(BackgroundContinuityState::UiDisconnected),
        "HostSuspended" => Ok(BackgroundContinuityState::HostSuspended),
        "HostOffline" => Ok(BackgroundContinuityState::HostOffline),
        "DeviceUnavailable" => Ok(BackgroundContinuityState::DeviceUnavailable),
        "ProviderUnavailable" => Ok(BackgroundContinuityState::ProviderUnavailable),
        "Recovering" => Ok(BackgroundContinuityState::Recovering),
        "Reconciling" => Ok(BackgroundContinuityState::Reconciling),
        "UserRequired" => Ok(BackgroundContinuityState::UserRequired),
        "SafelyFailed" => Ok(BackgroundContinuityState::SafelyFailed),
        "Completed" => Ok(BackgroundContinuityState::Completed),
        other => Err(rusqlite::Error::InvalidParameterName(format!(
            "unknown continuity state: {other}"
        ))),
    }
}

fn parse_preview_truth(value: &str) -> rusqlite::Result<PreviewTruth> {
    match value {
        "Predicted" => Ok(PreviewTruth::Predicted),
        "Requested" => Ok(PreviewTruth::Requested),
        "Observed" => Ok(PreviewTruth::Observed),
        "Verified" => Ok(PreviewTruth::Verified),
        "Stale" => Ok(PreviewTruth::Stale),
        "Invalidated" => Ok(PreviewTruth::Invalidated),
        other => Err(rusqlite::Error::InvalidParameterName(format!(
            "unknown preview truth: {other}"
        ))),
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
