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
             CREATE TABLE IF NOT EXISTS command_results (
                 command_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 idempotency_key TEXT,
                 request_fingerprint TEXT NOT NULL,
                 correlation_id TEXT NOT NULL,
                 snapshot_json TEXT NOT NULL,
                 UNIQUE(project_id, idempotency_key)
             );
             CREATE TABLE IF NOT EXISTS retention_floors (
                 project_id TEXT PRIMARY KEY,
                 first_available_sequence INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS provider_profiles (
                 project_id TEXT NOT NULL,
                 provider_id TEXT NOT NULL,
                 profile_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, provider_id)
             );
             CREATE TABLE IF NOT EXISTS android_construction_contracts (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 contract_id TEXT NOT NULL,
                 schema_version INTEGER NOT NULL,
                 contract_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id),
                 UNIQUE (project_id, contract_id)
             );
             CREATE TABLE IF NOT EXISTS android_toolchain_preflights (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 preflight_id TEXT NOT NULL,
                 status TEXT NOT NULL,
                 lock_hash TEXT,
                 environment_snapshot_id TEXT NOT NULL,
                 preflight_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id),
                 UNIQUE (project_id, preflight_id)
             );
             CREATE TABLE IF NOT EXISTS provider_usage (
                 request_id TEXT PRIMARY KEY,
                 correlation_id TEXT NOT NULL,
                 project_id TEXT NOT NULL,
                 provider_id TEXT NOT NULL,
                 model_id TEXT NOT NULL,
                 started_at_epoch_seconds INTEGER NOT NULL,
                 duration_ms INTEGER NOT NULL,
                 input_tokens INTEGER,
                 output_tokens INTEGER,
                 total_tokens INTEGER,
                 outcome TEXT NOT NULL
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

    pub fn commit_event_projection_and_command_and_provider_profile(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        provider_profile: Option<(&str, &str)>,
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
        transaction.execute(
            "INSERT INTO command_results (command_id, project_id, idempotency_key, request_fingerprint, correlation_id, snapshot_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                command_id,
                snapshot.project_id.0,
                idempotency_key,
                request_fingerprint,
                correlation_id,
                snapshot_json,
            ],
        )?;
        if let Some((provider_id, profile_json)) = provider_profile {
            transaction.execute(
                "INSERT INTO provider_profiles (project_id, provider_id, profile_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(project_id, provider_id) DO UPDATE SET profile_json = excluded.profile_json",
                params![snapshot.project_id.0, provider_id, profile_json],
            )?;
        }
        transaction.commit()
    }

    pub fn commit_event_projection_and_command_and_android_contract(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        contract: Option<(&str, &str, &str, u16)>,
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
        transaction.execute(
            "INSERT INTO command_results (command_id, project_id, idempotency_key, request_fingerprint, correlation_id, snapshot_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                command_id,
                snapshot.project_id.0,
                idempotency_key,
                request_fingerprint,
                correlation_id,
                snapshot_json,
            ],
        )?;
        if let Some((task_id, contract_id, contract_json, schema_version)) = contract {
            transaction.execute(
                "INSERT INTO android_construction_contracts (project_id, task_id, contract_id, schema_version, contract_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(project_id, task_id) DO UPDATE SET
                   contract_id = excluded.contract_id,
                   schema_version = excluded.schema_version,
                   contract_json = excluded.contract_json",
                params![snapshot.project_id.0, task_id, contract_id, schema_version, contract_json],
            )?;
        }
        transaction.commit()
    }

    pub fn commit_event_projection_and_command_and_android_toolchain_preflight(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        preflight: Option<(&str, &str, &str, &str, Option<&str>, &str)>,
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
        transaction.execute(
            "INSERT INTO command_results (command_id, project_id, idempotency_key, request_fingerprint, correlation_id, snapshot_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                command_id,
                snapshot.project_id.0,
                idempotency_key,
                request_fingerprint,
                correlation_id,
                snapshot_json,
            ],
        )?;
        if let Some((
            task_id,
            preflight_id,
            status,
            environment_snapshot_id,
            lock_hash,
            preflight_json,
        )) = preflight
        {
            transaction.execute(
                "INSERT INTO android_toolchain_preflights (project_id, task_id, preflight_id, status, lock_hash, environment_snapshot_id, preflight_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(project_id, task_id) DO UPDATE SET
                   preflight_id = excluded.preflight_id,
                   status = excluded.status,
                   lock_hash = excluded.lock_hash,
                   environment_snapshot_id = excluded.environment_snapshot_id,
                   preflight_json = excluded.preflight_json",
                params![
                    snapshot.project_id.0,
                    task_id,
                    preflight_id,
                    status,
                    lock_hash,
                    environment_snapshot_id,
                    preflight_json,
                ],
            )?;
            transaction.execute(
                "INSERT INTO checkpoints (checkpoint_id, project_id, projection_revision, source_revision, event_sequence)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(checkpoint_id) DO NOTHING",
                params![
                    preflight_id,
                    snapshot.project_id.0,
                    snapshot.projection_revision.0,
                    snapshot.current_source_revision.0,
                    snapshot.last_event_sequence,
                ],
            )?;
        }
        transaction.commit()
    }

    pub fn commit_event_projection_and_command(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
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
        transaction.execute(
            "INSERT INTO command_results (command_id, project_id, idempotency_key, request_fingerprint, correlation_id, snapshot_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                command_id,
                snapshot.project_id.0,
                idempotency_key,
                request_fingerprint,
                correlation_id,
                snapshot_json,
            ],
        )?;
        transaction.commit()
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

    pub fn load_command_result(
        &self,
        project_id: &ProjectId,
        command_id: &str,
        idempotency_key: Option<&str>,
    ) -> rusqlite::Result<Option<StoredCommandResult>> {
        self.connection
            .query_row(
                "SELECT command_id, idempotency_key, request_fingerprint, correlation_id, snapshot_json
                 FROM command_results
                 WHERE project_id = ?1 AND (command_id = ?2 OR (?3 IS NOT NULL AND idempotency_key = ?3))
                 LIMIT 1",
                params![project_id.0, command_id, idempotency_key],
                |row| {
                    Ok(StoredCommandResult {
                        command_id: row.get(0)?,
                        idempotency_key: row.get(1)?,
                        request_fingerprint: row.get(2)?,
                        correlation_id: row.get(3)?,
                        snapshot_json: row.get(4)?,
                    })
                },
            )
            .optional()
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

    pub fn set_retention_floor(
        &self,
        project_id: &ProjectId,
        first_available_sequence: u64,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO retention_floors (project_id, first_available_sequence)
             VALUES (?1, ?2)
             ON CONFLICT(project_id) DO UPDATE SET first_available_sequence = excluded.first_available_sequence",
            params![project_id.0, first_available_sequence],
        )?;
        Ok(())
    }

    pub fn retention_floor(&self, project_id: &ProjectId) -> rusqlite::Result<Option<u64>> {
        self.connection
            .query_row(
                "SELECT first_available_sequence FROM retention_floors WHERE project_id = ?1",
                params![project_id.0],
                |row| row.get(0),
            )
            .optional()
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

    pub fn save_android_construction_contract(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        contract_id: &str,
        schema_version: u16,
        contract_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO android_construction_contracts (project_id, task_id, contract_id, schema_version, contract_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(project_id, task_id) DO UPDATE SET
               contract_id = excluded.contract_id,
               schema_version = excluded.schema_version,
               contract_json = excluded.contract_json",
            params![project_id.0, task_id, contract_id, schema_version, contract_json],
        )?;
        Ok(())
    }

    pub fn load_android_construction_contract(
        &self,
        project_id: &ProjectId,
        task_id: &str,
    ) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT contract_json FROM android_construction_contracts WHERE project_id = ?1 AND task_id = ?2",
                params![project_id.0, task_id],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn load_android_toolchain_preflight(
        &self,
        project_id: &ProjectId,
        task_id: &str,
    ) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT preflight_json FROM android_toolchain_preflights WHERE project_id = ?1 AND task_id = ?2",
                params![project_id.0, task_id],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn save_provider_profile(
        &self,
        project_id: &ProjectId,
        provider_id: &str,
        profile_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO provider_profiles (project_id, provider_id, profile_json)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(project_id, provider_id) DO UPDATE SET profile_json = excluded.profile_json",
            params![project_id.0, provider_id, profile_json],
        )?;
        Ok(())
    }

    pub fn load_provider_profile(
        &self,
        project_id: &ProjectId,
        provider_id: &str,
    ) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT profile_json FROM provider_profiles WHERE project_id = ?1 AND provider_id = ?2",
                params![project_id.0, provider_id],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn record_provider_usage(
        &self,
        record: &nirman_domain::ProviderUsageRecord,
    ) -> rusqlite::Result<()> {
        if let Some(existing) = self.provider_usage(&record.request_id)? {
            if existing.correlation_id != record.correlation_id
                || existing.project_id != record.project_id
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "provider usage request identity conflict".into(),
                ));
            }
        }
        self.connection.execute(
            "INSERT INTO provider_usage (
                request_id, correlation_id, project_id, provider_id, model_id,
                started_at_epoch_seconds, duration_ms, input_tokens, output_tokens,
                total_tokens, outcome
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(request_id) DO UPDATE SET
                correlation_id = excluded.correlation_id,
                project_id = excluded.project_id,
                provider_id = excluded.provider_id,
                model_id = excluded.model_id,
                started_at_epoch_seconds = excluded.started_at_epoch_seconds,
                duration_ms = excluded.duration_ms,
                input_tokens = excluded.input_tokens,
                output_tokens = excluded.output_tokens,
                total_tokens = excluded.total_tokens,
                outcome = excluded.outcome",
            params![
                record.request_id,
                record.correlation_id,
                record.project_id.0,
                record.provider_id,
                record.model_id,
                record.started_at_epoch_seconds,
                record.duration_ms,
                record.input_tokens,
                record.output_tokens,
                record.total_tokens,
                record.outcome,
            ],
        )?;
        Ok(())
    }

    pub fn provider_usage(
        &self,
        request_id: &str,
    ) -> rusqlite::Result<Option<nirman_domain::ProviderUsageRecord>> {
        self.connection
            .query_row(
                "SELECT request_id, correlation_id, project_id, provider_id, model_id,
                        started_at_epoch_seconds, duration_ms, input_tokens, output_tokens,
                        total_tokens, outcome
                 FROM provider_usage WHERE request_id = ?1",
                params![request_id],
                |row| {
                    Ok(nirman_domain::ProviderUsageRecord {
                        request_id: row.get(0)?,
                        correlation_id: row.get(1)?,
                        project_id: ProjectId(row.get(2)?),
                        provider_id: row.get(3)?,
                        model_id: row.get(4)?,
                        started_at_epoch_seconds: row.get(5)?,
                        duration_ms: row.get(6)?,
                        input_tokens: row.get(7)?,
                        output_tokens: row.get(8)?,
                        total_tokens: row.get(9)?,
                        outcome: row.get(10)?,
                    })
                },
            )
            .optional()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCommandResult {
    pub command_id: String,
    pub idempotency_key: Option<String>,
    pub request_fingerprint: String,
    pub correlation_id: String,
    pub snapshot_json: String,
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
