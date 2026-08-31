//! Durable SQLite ledger for Nirman commands, events, and projections.

#![forbid(unsafe_code)]

use nirman_domain::{
    BackgroundContinuityState, ControlEvent, MutationTransactionRecord, PreviewTruth,
    ProductLifecycleState, ProjectId, ProjectionSnapshot, ProviderExecutionRecord, Revision,
    TaskId,
};
use nirman_policy::PolicyDecision;
use nirman_supervisor::BackgroundRunRecord;
use nirman_workers::{
    CoordinationTask, M8ReconciliationCheckpoint, WorkerExecutionRecord,
    WorkerHandoffAcknowledgement, WorkerHandoffRecord, WorkerTaskClaim,
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
             CREATE TABLE IF NOT EXISTS android_synthesis_builds (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 source_revision INTEGER NOT NULL,
                 project_fingerprint TEXT NOT NULL,
                 contract_id TEXT NOT NULL,
                 plan_json TEXT NOT NULL,
                 build_request_json TEXT NOT NULL,
                 toolchain_lock_hash TEXT NOT NULL,
                 environment_snapshot_id TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id, source_revision)
             );
             CREATE TABLE IF NOT EXISTS android_build_observations (
                 execution_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 source_revision INTEGER NOT NULL,
                 project_fingerprint TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 UNIQUE(project_id, task_id, source_revision)
             );
             CREATE TABLE IF NOT EXISTS android_device_observations (
                 observation_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 source_revision INTEGER NOT NULL,
                 device_identity TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 UNIQUE(project_id, task_id, source_revision, device_identity)
             );
             CREATE TABLE IF NOT EXISTS android_artifact_exports (
                 export_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 source_revision INTEGER NOT NULL,
                 destination_path TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 UNIQUE(project_id, task_id, source_revision)
             );
             CREATE TABLE IF NOT EXISTS apk_delivery_records (
                 delivery_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 source_revision INTEGER NOT NULL,
                 state TEXT NOT NULL,
                 record_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS android_requirement_manifests (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 manifest_id TEXT NOT NULL,
                 source_revision INTEGER NOT NULL,
                 project_fingerprint TEXT NOT NULL,
                 manifest_json TEXT NOT NULL,
                 repair_selection_json TEXT,
                 PRIMARY KEY (project_id, task_id, source_revision),
                 UNIQUE (project_id, manifest_id)
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
             CREATE TABLE IF NOT EXISTS provider_executions (
                 execution_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 request_id TEXT NOT NULL,
                 correlation_id TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 UNIQUE(project_id, request_id)
             );
             CREATE TABLE IF NOT EXISTS mutation_transactions (
                 transaction_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 command_id TEXT NOT NULL UNIQUE,
                 operation_id TEXT NOT NULL UNIQUE,
                 task_id TEXT NOT NULL,
                 state TEXT NOT NULL,
                 record_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS preview_revisions (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 preview_revision_id TEXT NOT NULL,
                 project_revision_id TEXT NOT NULL,
                 source_fingerprint TEXT NOT NULL,
                 lifecycle_state TEXT NOT NULL,
                 revision_json TEXT NOT NULL,
                 selection_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, preview_revision_id)
             );
             CREATE TABLE IF NOT EXISTS m108_preview_sync_events (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 event_sequence INTEGER NOT NULL,
                 event_id TEXT NOT NULL,
                 event_json TEXT NOT NULL,
                 evidence_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id, event_sequence),
                 UNIQUE (project_id, task_id, event_id)
             );
             CREATE TABLE IF NOT EXISTS m108_preview_sync_evidence (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 evidence_id TEXT NOT NULL,
                 event_sequence INTEGER NOT NULL,
                 evidence_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id, evidence_id)
             );
             CREATE TABLE IF NOT EXISTS m108_preview_sync_records (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 projection_json TEXT NOT NULL,
                 evidence_json TEXT NOT NULL,
                 last_event_sequence INTEGER NOT NULL,
                 PRIMARY KEY (project_id, task_id)
             );
             CREATE TABLE IF NOT EXISTS m7_background_runs (
                 run_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 worker_id TEXT NOT NULL,
                 state TEXT NOT NULL,
                 record_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS m5_worker_execution_records (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 worker_id TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id)
             );
             CREATE TABLE IF NOT EXISTS m6_policy_events (
                 decision_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 worker_id TEXT NOT NULL,
                 request_id TEXT NOT NULL,
                 outcome TEXT NOT NULL,
                 decision_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS m8_reconciliation_checkpoints (
                 project_id TEXT NOT NULL,
                 checkpoint_id TEXT NOT NULL,
                 status TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, checkpoint_id)
             );
             CREATE TABLE IF NOT EXISTS m8_worker_task_claims (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 worker_id TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id)
             );
             CREATE TABLE IF NOT EXISTS m8_coordination_tasks (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id)
             );
             CREATE TABLE IF NOT EXISTS m8_worker_handoffs (
                 project_id TEXT NOT NULL,
                 message_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 worker_id TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, message_id)
             );
             CREATE TABLE IF NOT EXISTS m8_worker_handoff_acknowledgements (
                 project_id TEXT NOT NULL,
                 acknowledgement_id TEXT NOT NULL,
                 message_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 worker_id TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, acknowledgement_id),
                 UNIQUE (project_id, message_id)
             );
             CREATE TABLE IF NOT EXISTS preview_projections (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 projection_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id)
             );
             CREATE TABLE IF NOT EXISTS android_project_scaffolds (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 source_revision INTEGER NOT NULL,
                 scaffold_id TEXT NOT NULL,
                 contract_id TEXT NOT NULL,
                 scaffold_fingerprint TEXT NOT NULL,
                 resulting_project_fingerprint TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id, source_revision),
                 UNIQUE (project_id, scaffold_id)
             );
             CREATE TABLE IF NOT EXISTS agent_loop_records (
                 loop_id TEXT PRIMARY KEY,
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 state TEXT NOT NULL,
                 updated_at_epoch_seconds INTEGER NOT NULL,
                 record_json TEXT NOT NULL
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
             );
             CREATE TABLE IF NOT EXISTS m118_platform_preflights (
                 project_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 environment_id TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, task_id)
             );
             CREATE TABLE IF NOT EXISTS m118_platform_gate_records (
                 project_id TEXT NOT NULL,
                 gate_id TEXT NOT NULL,
                 stage TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, gate_id)
             );
             CREATE TABLE IF NOT EXISTS m118_platform_blocked_decisions (
                 project_id TEXT NOT NULL,
                 decision_id TEXT NOT NULL,
                 task_id TEXT NOT NULL,
                 stage TEXT NOT NULL,
                 record_json TEXT NOT NULL,
                 PRIMARY KEY (project_id, decision_id, task_id)
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

    pub fn commit_event_projection_and_command_and_preview_revision(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        preview: Option<(&str, &str, &str, &str, &str, &str)>,
        projection: Option<(&str, &str, &str)>,
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
            preview_revision_id,
            project_revision_id,
            source_fingerprint,
            revision_json,
            selection_json,
        )) = preview
        {
            transaction.execute(
                "INSERT INTO preview_revisions (project_id, task_id, preview_revision_id, project_revision_id, source_fingerprint, lifecycle_state, revision_json, selection_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'REQUEST_AUTHORIZED', ?6, ?7)
                 ON CONFLICT(project_id, preview_revision_id) DO UPDATE SET
                   task_id = excluded.task_id,
                   project_revision_id = excluded.project_revision_id,
                   source_fingerprint = excluded.source_fingerprint,
                   lifecycle_state = excluded.lifecycle_state,
                   revision_json = excluded.revision_json,
                   selection_json = excluded.selection_json",
                params![
                    snapshot.project_id.0,
                    task_id,
                    preview_revision_id,
                    project_revision_id,
                    source_fingerprint,
                    revision_json,
                    selection_json,
                ],
            )?;
        }
        if let Some((task_id, projection_json, _projection_revision)) = projection {
            transaction.execute(
                "INSERT INTO preview_projections (project_id, task_id, projection_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(project_id, task_id) DO UPDATE SET projection_json = excluded.projection_json",
                params![snapshot.project_id.0, task_id, projection_json],
            )?;
        }
        transaction.commit()
    }

    pub fn save_preview_revision(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        preview_revision_id: &str,
        project_revision_id: &str,
        source_fingerprint: &str,
        lifecycle_state: &str,
        revision_json: &str,
        selection_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO preview_revisions (project_id, task_id, preview_revision_id, project_revision_id, source_fingerprint, lifecycle_state, revision_json, selection_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(project_id, preview_revision_id) DO UPDATE SET
               task_id = excluded.task_id,
               project_revision_id = excluded.project_revision_id,
               source_fingerprint = excluded.source_fingerprint,
               lifecycle_state = excluded.lifecycle_state,
               revision_json = excluded.revision_json,
               selection_json = excluded.selection_json",
            params![project_id.0, task_id, preview_revision_id, project_revision_id, source_fingerprint, lifecycle_state, revision_json, selection_json],
        )?;
        Ok(())
    }

    pub fn load_preview_revision(
        &self,
        project_id: &ProjectId,
        preview_revision_id: &str,
    ) -> rusqlite::Result<Option<(String, String)>> {
        self.connection
            .query_row(
                "SELECT revision_json, selection_json FROM preview_revisions
                 WHERE project_id = ?1 AND preview_revision_id = ?2",
                params![project_id.0, preview_revision_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
    }

    pub fn save_preview_projection(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        projection_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO preview_projections (project_id, task_id, projection_json)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(project_id, task_id) DO UPDATE SET projection_json = excluded.projection_json",
            params![project_id.0, task_id, projection_json],
        )?;
        Ok(())
    }

    pub fn load_preview_projection(
        &self,
        project_id: &ProjectId,
        task_id: &str,
    ) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT projection_json FROM preview_projections
                 WHERE project_id = ?1 AND task_id = ?2",
                params![project_id.0, task_id],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn commit_event_projection_and_command_and_m4(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        m4: (&str, u64, &str, &str, &str, &str, &str, &str),
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute("INSERT INTO events (sequence,event_id,project_id,task_id,kind,payload,source_revision) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![event.sequence,event.event_id,event.project_id.0,event.task_id.as_ref().map(|id| id.0.as_str()),event.kind,event.payload,event.source_revision.0])?;
        transaction.execute("INSERT INTO projections (project_id,projection_revision,task_state,continuity_state,preview_truth,source_revision,last_event_sequence,last_known_good_ref) VALUES (?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(project_id) DO UPDATE SET projection_revision=excluded.projection_revision,task_state=excluded.task_state,continuity_state=excluded.continuity_state,preview_truth=excluded.preview_truth,source_revision=excluded.source_revision,last_event_sequence=excluded.last_event_sequence,last_known_good_ref=excluded.last_known_good_ref", params![snapshot.project_id.0,snapshot.projection_revision.0,format!("{:?}",snapshot.task_state),format!("{:?}",snapshot.continuity_state),format!("{:?}",snapshot.preview_truth),snapshot.current_source_revision.0,snapshot.last_event_sequence,snapshot.last_known_good_ref])?;
        transaction.execute("INSERT INTO command_results (command_id,project_id,idempotency_key,request_fingerprint,correlation_id,snapshot_json) VALUES (?1,?2,?3,?4,?5,?6)", params![command_id,snapshot.project_id.0,idempotency_key,request_fingerprint,correlation_id,snapshot_json])?;
        let (
            task_id,
            source_revision,
            fingerprint,
            contract_id,
            plan_json,
            build_json,
            lock_hash,
            environment_id,
        ) = m4;
        transaction.execute("INSERT INTO android_synthesis_builds (project_id,task_id,source_revision,project_fingerprint,contract_id,plan_json,build_request_json,toolchain_lock_hash,environment_snapshot_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(project_id,task_id,source_revision) DO UPDATE SET project_fingerprint=excluded.project_fingerprint,contract_id=excluded.contract_id,plan_json=excluded.plan_json,build_request_json=excluded.build_request_json,toolchain_lock_hash=excluded.toolchain_lock_hash,environment_snapshot_id=excluded.environment_snapshot_id", params![snapshot.project_id.0,task_id,source_revision,fingerprint,contract_id,plan_json,build_json,lock_hash,environment_id])?;
        transaction.commit()
    }

    pub fn save_android_build_observation(
        &self,
        execution_id: &str,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
        project_fingerprint: &str,
        record_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO android_build_observations (execution_id, project_id, task_id, source_revision, project_fingerprint, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(execution_id) DO UPDATE SET record_json=excluded.record_json
             ON CONFLICT(project_id, task_id, source_revision) DO UPDATE SET execution_id=excluded.execution_id, project_fingerprint=excluded.project_fingerprint, record_json=excluded.record_json",
            params![execution_id, project_id.0, task_id, source_revision, project_fingerprint, record_json],
        )?;
        Ok(())
    }

    pub fn load_android_build_observation(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
    ) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT record_json FROM android_build_observations WHERE project_id=?1 AND task_id=?2 AND source_revision=?3",
                params![project_id.0, task_id, source_revision],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn save_android_artifact_export(
        &self,
        export_id: &str,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
        destination_path: &str,
        record_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO android_artifact_exports (export_id, project_id, task_id, source_revision, destination_path, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(project_id, task_id, source_revision) DO UPDATE SET export_id=excluded.export_id, destination_path=excluded.destination_path, record_json=excluded.record_json",
            params![export_id, project_id.0, task_id, source_revision, destination_path, record_json],
        )?;
        Ok(())
    }

    pub fn load_android_artifact_export(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
    ) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT record_json FROM android_artifact_exports WHERE project_id=?1 AND task_id=?2 AND source_revision=?3",
                params![project_id.0, task_id, source_revision],
                |row| row.get(0),
            )
            .optional()
    }

    /// Persists an APK delivery record (spec §74.3) so the delivery
    /// projection and later reconciliation reads survive host restarts.
    pub fn save_apk_delivery_record(
        &self,
        delivery_id: &str,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
        state: &str,
        record_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO apk_delivery_records (delivery_id, project_id, task_id, source_revision, state, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(delivery_id) DO UPDATE SET state=excluded.state, record_json=excluded.record_json",
            params![delivery_id, project_id.0, task_id, source_revision, state, record_json],
        )?;
        Ok(())
    }

    /// Most recently persisted APK delivery record for the project
    /// (task id, source revision, serialized ApkDeliveryRecord).
    pub fn latest_apk_delivery_record(
        &self,
        project_id: &ProjectId,
    ) -> rusqlite::Result<Option<(String, u64, String)>> {
        self.connection
            .query_row(
                "SELECT task_id, source_revision, record_json FROM apk_delivery_records
                 WHERE project_id=?1 ORDER BY rowid DESC LIMIT 1",
                params![project_id.0],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
    }

    /// Most recently recorded Android build observation for the project
    /// (task id, source revision, serialized AndroidBuildObservation).
    pub fn latest_android_build_observation(
        &self,
        project_id: &ProjectId,
    ) -> rusqlite::Result<Option<(String, u64, String)>> {
        self.connection
            .query_row(
                "SELECT task_id, source_revision, record_json FROM android_build_observations
                 WHERE project_id=?1 ORDER BY rowid DESC LIMIT 1",
                params![project_id.0],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
    }

    /// Identity of the most recent device observation plus the project-wide
    /// observation count for the evidence projection.
    pub fn latest_device_observation_identity(
        &self,
        project_id: &ProjectId,
    ) -> rusqlite::Result<Option<(String, String)>> {
        self.connection
            .query_row(
                "SELECT observation_id, device_identity FROM android_device_observations
                 WHERE project_id=?1 ORDER BY rowid DESC LIMIT 1",
                params![project_id.0],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
    }

    /// Project-wide durable evidence census for the evidence projection.
    pub fn evidence_census(&self, project_id: &ProjectId) -> rusqlite::Result<(u32, u32, u32)> {
        let m108_event_count = self.connection.query_row(
            "SELECT COUNT(*) FROM m108_preview_sync_events WHERE project_id=?1",
            params![project_id.0],
            |row| row.get::<_, i64>(0),
        )? as u32;
        let m108_evidence_count = self.connection.query_row(
            "SELECT COUNT(*) FROM m108_preview_sync_evidence WHERE project_id=?1",
            params![project_id.0],
            |row| row.get::<_, i64>(0),
        )? as u32;
        let device_observation_count = self.connection.query_row(
            "SELECT COUNT(*) FROM android_device_observations WHERE project_id=?1",
            params![project_id.0],
            |row| row.get::<_, i64>(0),
        )? as u32;
        Ok((
            m108_event_count,
            m108_evidence_count,
            device_observation_count,
        ))
    }

    pub fn save_android_device_observation(
        &self,
        observation_id: &str,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
        device_identity: &str,
        record_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO android_device_observations (observation_id, project_id, task_id, source_revision, device_identity, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(project_id, task_id, source_revision, device_identity) DO UPDATE SET observation_id=excluded.observation_id, record_json=excluded.record_json",
            params![observation_id, project_id.0, task_id, source_revision, device_identity, record_json],
        )?;
        Ok(())
    }

    pub fn load_android_device_observation(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
        device_identity: &str,
    ) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT record_json FROM android_device_observations WHERE project_id=?1 AND task_id=?2 AND source_revision=?3 AND device_identity=?4",
                params![project_id.0, task_id, source_revision, device_identity],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn load_android_device_observation_for_source(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
    ) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT record_json FROM android_device_observations WHERE project_id=?1 AND task_id=?2 AND source_revision=?3 ORDER BY observation_id DESC LIMIT 1",
                params![project_id.0, task_id, source_revision],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn load_android_synthesis_build(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
    ) -> rusqlite::Result<Option<(String, String, String, String, String)>> {
        self.connection.query_row("SELECT plan_json,build_request_json,toolchain_lock_hash,environment_snapshot_id,project_fingerprint FROM android_synthesis_builds WHERE project_id=?1 AND task_id=?2 AND source_revision=?3", params![project_id.0,task_id,source_revision], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))).optional()
    }

    pub fn save_android_synthesis_build(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
        project_fingerprint: &str,
        contract_id: &str,
        plan_json: &str,
        build_request_json: &str,
        toolchain_lock_hash: &str,
        environment_snapshot_id: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO android_synthesis_builds
                 (project_id, task_id, source_revision, project_fingerprint, contract_id,
                  plan_json, build_request_json, toolchain_lock_hash, environment_snapshot_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(project_id, task_id, source_revision) DO UPDATE SET
               project_fingerprint = excluded.project_fingerprint,
               contract_id = excluded.contract_id,
               plan_json = excluded.plan_json,
               build_request_json = excluded.build_request_json,
               toolchain_lock_hash = excluded.toolchain_lock_hash,
               environment_snapshot_id = excluded.environment_snapshot_id",
            params![
                project_id.0,
                task_id,
                source_revision,
                project_fingerprint,
                contract_id,
                plan_json,
                build_request_json,
                toolchain_lock_hash,
                environment_snapshot_id
            ],
        )?;
        Ok(())
    }

    pub fn save_android_project_scaffold(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
        scaffold_id: &str,
        contract_id: &str,
        scaffold_fingerprint: &str,
        resulting_project_fingerprint: &str,
        record_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO android_project_scaffolds
                 (project_id, task_id, source_revision, scaffold_id, contract_id,
                  scaffold_fingerprint, resulting_project_fingerprint, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(project_id, task_id, source_revision) DO UPDATE SET
               scaffold_id = excluded.scaffold_id,
               contract_id = excluded.contract_id,
               scaffold_fingerprint = excluded.scaffold_fingerprint,
               resulting_project_fingerprint = excluded.resulting_project_fingerprint,
               record_json = excluded.record_json",
            params![
                project_id.0,
                task_id,
                source_revision,
                scaffold_id,
                contract_id,
                scaffold_fingerprint,
                resulting_project_fingerprint,
                record_json
            ],
        )?;
        Ok(())
    }

    pub fn load_android_project_scaffold(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
    ) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT record_json FROM android_project_scaffolds
                 WHERE project_id = ?1 AND task_id = ?2 AND source_revision = ?3",
                params![project_id.0, task_id, source_revision],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn save_agent_loop_record(
        &self,
        loop_id: &str,
        project_id: &ProjectId,
        task_id: &str,
        state: &str,
        updated_at_epoch_seconds: u64,
        record_json: &str,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO agent_loop_records
                 (loop_id, project_id, task_id, state, updated_at_epoch_seconds, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(loop_id) DO UPDATE SET
               state = excluded.state,
               updated_at_epoch_seconds = excluded.updated_at_epoch_seconds,
               record_json = excluded.record_json",
            params![
                loop_id,
                project_id.0,
                task_id,
                state,
                updated_at_epoch_seconds,
                record_json
            ],
        )?;
        Ok(())
    }

    pub fn load_agent_loop_record(&self, loop_id: &str) -> rusqlite::Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT record_json FROM agent_loop_records WHERE loop_id = ?1",
                params![loop_id],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn commit_event_projection_and_command_and_android_requirement_manifest(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        manifest: Option<(&str, &str, u64, &str, &str, Option<&str>)>,
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
            manifest_id,
            source_revision,
            project_fingerprint,
            manifest_json,
            repair_selection_json,
        )) = manifest
        {
            transaction.execute(
                "INSERT INTO android_requirement_manifests (project_id, task_id, manifest_id, source_revision, project_fingerprint, manifest_json, repair_selection_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(project_id, task_id, source_revision) DO UPDATE SET
                   manifest_id = excluded.manifest_id,
                   project_fingerprint = excluded.project_fingerprint,
                   manifest_json = excluded.manifest_json,
                   repair_selection_json = excluded.repair_selection_json",
                params![
                    snapshot.project_id.0,
                    task_id,
                    manifest_id,
                    source_revision,
                    project_fingerprint,
                    manifest_json,
                    repair_selection_json,
                ],
            )?;
        }
        transaction.commit()
    }

    pub fn save_android_requirement_manifest(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        manifest_id: &str,
        source_revision: u64,
        project_fingerprint: &str,
        manifest_json: &str,
        repair_selection_json: Option<&str>,
    ) -> rusqlite::Result<()> {
        self.connection.execute(
            "INSERT INTO android_requirement_manifests (project_id, task_id, manifest_id, source_revision, project_fingerprint, manifest_json, repair_selection_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(project_id, task_id, source_revision) DO UPDATE SET
               manifest_id = excluded.manifest_id,
               project_fingerprint = excluded.project_fingerprint,
               manifest_json = excluded.manifest_json,
               repair_selection_json = excluded.repair_selection_json",
            params![project_id.0, task_id, manifest_id, source_revision, project_fingerprint, manifest_json, repair_selection_json],
        )?;
        Ok(())
    }

    pub fn load_android_requirement_manifest(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        source_revision: u64,
    ) -> rusqlite::Result<Option<(String, String, String)>> {
        self.connection
            .query_row(
                "SELECT manifest_json, project_fingerprint, COALESCE(repair_selection_json, '')
                 FROM android_requirement_manifests
                 WHERE project_id = ?1 AND task_id = ?2 AND source_revision = ?3",
                params![project_id.0, task_id, source_revision],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
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

    pub fn commit_event_projection_and_command_and_worker_execution(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        record: &WorkerExecutionRecord,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(record).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO events (sequence, event_id, project_id, task_id, kind, payload, source_revision) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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
            "INSERT INTO projections (project_id, projection_revision, task_state, continuity_state, preview_truth, source_revision, last_event_sequence, last_known_good_ref) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(project_id) DO UPDATE SET projection_revision=excluded.projection_revision, continuity_state=excluded.continuity_state, preview_truth=excluded.preview_truth, source_revision=excluded.source_revision, last_event_sequence=excluded.last_event_sequence, last_known_good_ref=excluded.last_known_good_ref",
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
            "INSERT INTO command_results (command_id, project_id, idempotency_key, request_fingerprint, correlation_id, snapshot_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                command_id,
                snapshot.project_id.0,
                idempotency_key,
                request_fingerprint,
                correlation_id,
                snapshot_json,
            ],
        )?;
        transaction.execute(
            "INSERT INTO m5_worker_execution_records (project_id, task_id, worker_id, record_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(project_id, task_id) DO UPDATE SET worker_id=excluded.worker_id, record_json=excluded.record_json",
            params![
                snapshot.project_id.0,
                record.task_id().0,
                record.worker_id(),
                record_json,
            ],
        )?;
        transaction.commit()
    }

    pub fn commit_event_projection_and_command_and_background_run(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        run: &BackgroundRunRecord,
    ) -> rusqlite::Result<()> {
        let run_json = serde_json::to_string(run).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        run.validate().map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO events (sequence, event_id, project_id, task_id, kind, payload, source_revision) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![event.sequence, event.event_id, event.project_id.0, event.task_id.as_ref().map(|id| id.0.as_str()), event.kind, event.payload, event.source_revision.0],
        )?;
        transaction.execute(
            "INSERT INTO projections (project_id, projection_revision, task_state, continuity_state, preview_truth, source_revision, last_event_sequence, last_known_good_ref) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(project_id) DO UPDATE SET projection_revision=excluded.projection_revision, task_state=excluded.task_state, continuity_state=excluded.continuity_state, preview_truth=excluded.preview_truth, source_revision=excluded.source_revision, last_event_sequence=excluded.last_event_sequence, last_known_good_ref=excluded.last_known_good_ref",
            params![snapshot.project_id.0, snapshot.projection_revision.0, format!("{:?}", snapshot.task_state), format!("{:?}", snapshot.continuity_state), format!("{:?}", snapshot.preview_truth), snapshot.current_source_revision.0, snapshot.last_event_sequence, snapshot.last_known_good_ref],
        )?;
        transaction.execute(
            "INSERT INTO command_results (command_id, project_id, idempotency_key, request_fingerprint, correlation_id, snapshot_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![command_id, snapshot.project_id.0, idempotency_key, request_fingerprint, correlation_id, snapshot_json],
        )?;
        transaction.execute(
            "INSERT INTO m7_background_runs (run_id, project_id, task_id, worker_id, state, record_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(run_id) DO UPDATE SET project_id=excluded.project_id, task_id=excluded.task_id, worker_id=excluded.worker_id, state=excluded.state, record_json=excluded.record_json",
            params![run.run_id, run.project_id, run.task_id, run.worker_id, serde_json::to_string(&run.state).unwrap_or_else(|_| "null".into()), run_json],
        )?;
        transaction.commit()
    }

    pub fn commit_event_projection_command_and_mutation_transaction(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        record: &MutationTransactionRecord,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(record).map_err(|_| {
            rusqlite::Error::InvalidParameterName(
                "mutation transaction serialization failed".into(),
            )
        })?;
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
        transaction.execute(
            "INSERT INTO mutation_transactions (
                transaction_id, project_id, command_id, operation_id, task_id, state, record_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.transaction_id,
                record.project_id.0,
                record.command_id,
                record.operation_id,
                record.task_id.0,
                record.state,
                record_json,
            ],
        )?;
        transaction.execute(
            "INSERT INTO checkpoints (checkpoint_id, project_id, projection_revision, source_revision, event_sequence)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(checkpoint_id) DO NOTHING",
            params![
                record.checkpoint_id,
                snapshot.project_id.0,
                snapshot.projection_revision.0,
                snapshot.current_source_revision.0,
                snapshot.last_event_sequence,
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

    pub fn load_checkpoint(
        &self,
        project_id: &ProjectId,
        checkpoint_id: &str,
    ) -> rusqlite::Result<Option<(Revision, Revision, u64)>> {
        self.connection
            .query_row(
                "SELECT projection_revision, source_revision, event_sequence
                 FROM checkpoints WHERE project_id = ?1 AND checkpoint_id = ?2",
                params![project_id.0, checkpoint_id],
                |row| {
                    Ok((
                        Revision(row.get::<_, u64>(0)?),
                        Revision(row.get::<_, u64>(1)?),
                        row.get::<_, u64>(2)?,
                    ))
                },
            )
            .optional()
    }

    pub fn checkpoint_exists(
        &self,
        project_id: &ProjectId,
        checkpoint_id: &str,
    ) -> rusqlite::Result<bool> {
        self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM checkpoints WHERE project_id = ?1 AND checkpoint_id = ?2)",
            params![project_id.0, checkpoint_id],
            |row| row.get(0),
        )
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
                    // Typed projection summaries are derived from the durable
                    // record tables, not from the persisted core projection;
                    // the durable control plane re-attaches them on read.
                    worker_projection: None,
                    artifact_projection: None,
                    evidence_projection: None,
                    delivery_projection: None,
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

    pub fn record_mutation_transaction(
        &self,
        record: &MutationTransactionRecord,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(record).map_err(|_| {
            rusqlite::Error::InvalidParameterName(
                "mutation transaction serialization failed".into(),
            )
        })?;
        self.connection.execute(
            "INSERT INTO mutation_transactions (
                transaction_id, project_id, command_id, operation_id, task_id, state, record_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(transaction_id) DO UPDATE SET state = excluded.state, record_json = excluded.record_json",
            params![
                record.transaction_id,
                record.project_id.0,
                record.command_id,
                record.operation_id,
                record.task_id.0,
                record.state,
                record_json,
            ],
        )?;
        Ok(())
    }

    pub fn mutation_transaction(
        &self,
        transaction_id: &str,
    ) -> rusqlite::Result<Option<MutationTransactionRecord>> {
        let record_json = self
            .connection
            .query_row(
                "SELECT record_json FROM mutation_transactions WHERE transaction_id = ?1",
                params![transaction_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        record_json
            .map(|json| {
                serde_json::from_str(&json).map_err(|_| {
                    rusqlite::Error::InvalidParameterName(
                        "mutation transaction record is corrupt".into(),
                    )
                })
            })
            .transpose()
    }

    pub fn record_provider_execution(
        &self,
        record: &ProviderExecutionRecord,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(record).map_err(|_| {
            rusqlite::Error::InvalidParameterName("provider execution serialization failed".into())
        })?;
        self.connection.execute(
            "INSERT INTO provider_executions (
                execution_id, project_id, task_id, request_id, correlation_id, record_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(execution_id) DO UPDATE SET record_json = excluded.record_json",
            params![
                record.execution_id,
                record.project_id.0,
                record.task_id.0,
                record.request_id,
                record.correlation_id,
                record_json,
            ],
        )?;
        Ok(())
    }

    pub fn provider_execution(
        &self,
        execution_id: &str,
    ) -> rusqlite::Result<Option<ProviderExecutionRecord>> {
        self.connection
            .query_row(
                "SELECT record_json FROM provider_executions WHERE execution_id = ?1",
                params![execution_id],
                |row| {
                    let record_json: String = row.get(0)?;
                    serde_json::from_str(&record_json).map_err(|_| rusqlite::Error::InvalidQuery)
                },
            )
            .optional()
    }

    pub fn append_m108_event_and_projection(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        event_sequence: u64,
        event_id: &str,
        event_json: &str,
        evidence_id: &str,
        evidence_json: &str,
        projection_json: &str,
        last_event_sequence: u64,
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO m108_preview_sync_events (project_id, task_id, event_sequence, event_id, event_json, evidence_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![project_id.0, task_id, event_sequence, event_id, event_json, evidence_json],
        )?;
        transaction.execute(
            "INSERT INTO m108_preview_sync_evidence (project_id, task_id, evidence_id, event_sequence, evidence_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![project_id.0, task_id, evidence_id, event_sequence, evidence_json],
        )?;
        transaction.execute(
            "INSERT INTO m108_preview_sync_records (project_id,task_id,projection_json,evidence_json,last_event_sequence) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(project_id,task_id) DO UPDATE SET projection_json=excluded.projection_json,evidence_json=excluded.evidence_json,last_event_sequence=excluded.last_event_sequence",
            params![project_id.0, task_id, projection_json, evidence_json, last_event_sequence],
        )?;
        transaction.commit()
    }

    pub fn save_m8_reconciliation_checkpoint(
        &self,
        project_id: &str,
        checkpoint: &M8ReconciliationCheckpoint,
    ) -> rusqlite::Result<()> {
        checkpoint.validate().map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        let record_json = serde_json::to_string(checkpoint).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        self.connection.execute(
            "INSERT INTO m8_reconciliation_checkpoints (project_id, checkpoint_id, status, record_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(project_id, checkpoint_id) DO UPDATE SET status = excluded.status, record_json = excluded.record_json",
            params![project_id, checkpoint.checkpoint_id, format!("{:?}", checkpoint.status), record_json],
        )?;
        Ok(())
    }

    pub fn load_m8_reconciliation_checkpoint(
        &self,
        project_id: &str,
        checkpoint_id: &str,
    ) -> rusqlite::Result<Option<M8ReconciliationCheckpoint>> {
        self.connection
            .query_row(
                "SELECT record_json FROM m8_reconciliation_checkpoints WHERE project_id = ?1 AND checkpoint_id = ?2",
                params![project_id, checkpoint_id],
                |row| {
                    let record_json: String = row.get(0)?;
                    serde_json::from_str(&record_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
                },
            )
            .optional()
    }

    pub fn save_worker_task_claim(
        &self,
        project_id: &str,
        claim: &WorkerTaskClaim,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(claim).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        self.connection.execute(
            "INSERT INTO m8_worker_task_claims (project_id, task_id, worker_id, record_json)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id, task_id) DO UPDATE SET worker_id = excluded.worker_id, record_json = excluded.record_json",
            params![project_id, claim.task_id, claim.lease.worker_id, record_json],
        )?;
        Ok(())
    }

    pub fn load_worker_task_claim(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> rusqlite::Result<Option<WorkerTaskClaim>> {
        self.connection
            .query_row(
                "SELECT record_json FROM m8_worker_task_claims WHERE project_id = ?1 AND task_id = ?2",
                params![project_id, task_id],
                |row| {
                    let record_json: String = row.get(0)?;
                    serde_json::from_str(&record_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
                },
            )
            .optional()
    }

    pub fn load_worker_task_claims(
        &self,
        project_id: &str,
    ) -> rusqlite::Result<Vec<WorkerTaskClaim>> {
        let mut statement = self.connection.prepare(
            "SELECT record_json FROM m8_worker_task_claims WHERE project_id = ?1 ORDER BY task_id",
        )?;
        let rows = statement.query_map(params![project_id], |row| {
            let record_json: String = row.get(0)?;
            serde_json::from_str(&record_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })?;
        rows.collect()
    }

    pub fn load_coordination_tasks(
        &self,
        project_id: &str,
    ) -> rusqlite::Result<Vec<CoordinationTask>> {
        let mut statement = self.connection.prepare(
            "SELECT record_json FROM m8_coordination_tasks WHERE project_id = ?1 ORDER BY task_id",
        )?;
        let rows = statement.query_map(params![project_id], |row| {
            let record_json: String = row.get(0)?;
            serde_json::from_str(&record_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })?;
        rows.collect()
    }

    pub fn load_worker_handoffs(
        &self,
        project_id: &str,
    ) -> rusqlite::Result<Vec<WorkerHandoffRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT record_json FROM m8_worker_handoffs WHERE project_id = ?1 ORDER BY message_id",
        )?;
        let rows = statement.query_map(params![project_id], |row| {
            let record_json: String = row.get(0)?;
            serde_json::from_str(&record_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })?;
        rows.collect()
    }

    pub fn load_worker_handoff_acknowledgements(
        &self,
        project_id: &str,
    ) -> rusqlite::Result<Vec<WorkerHandoffAcknowledgement>> {
        let mut statement = self.connection.prepare(
            "SELECT record_json FROM m8_worker_handoff_acknowledgements WHERE project_id = ?1 ORDER BY acknowledgement_id",
        )?;
        let rows = statement.query_map(params![project_id], |row| {
            let record_json: String = row.get(0)?;
            serde_json::from_str(&record_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })?;
        rows.collect()
    }

    pub fn commit_event_projection_and_command_and_m8(
        &self,
        event: &ControlEvent,
        snapshot: &ProjectionSnapshot,
        command_id: &str,
        idempotency_key: Option<&str>,
        request_fingerprint: &str,
        correlation_id: &str,
        snapshot_json: &str,
        checkpoint: Option<(&str, &str, &str)>,
        task: Option<(&str, &str)>,
        claim: Option<(&str, &str, &str)>,
        handoff: Option<(&str, &str, &str, &str)>,
        acknowledgement: Option<(&str, &str, &str, &str, &str)>,
    ) -> rusqlite::Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO events (sequence, event_id, project_id, task_id, kind, payload, source_revision) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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
            "INSERT INTO projections (project_id, projection_revision, task_state, continuity_state, preview_truth, source_revision, last_event_sequence, last_known_good_ref) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(project_id) DO UPDATE SET projection_revision = excluded.projection_revision, task_state = excluded.task_state, continuity_state = excluded.continuity_state, preview_truth = excluded.preview_truth, source_revision = excluded.source_revision, last_event_sequence = excluded.last_event_sequence, last_known_good_ref = excluded.last_known_good_ref",
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
            "INSERT INTO command_results (command_id, project_id, idempotency_key, request_fingerprint, correlation_id, snapshot_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![command_id, snapshot.project_id.0, idempotency_key, request_fingerprint, correlation_id, snapshot_json],
        )?;
        if let Some((checkpoint_id, status, record_json)) = checkpoint {
            transaction.execute(
                "INSERT INTO m8_reconciliation_checkpoints (project_id, checkpoint_id, status, record_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(project_id, checkpoint_id) DO UPDATE SET status = excluded.status, record_json = excluded.record_json",
                params![snapshot.project_id.0, checkpoint_id, status, record_json],
            )?;
        }
        if let Some((task_id, record_json)) = task {
            transaction.execute(
                "INSERT INTO m8_coordination_tasks (project_id, task_id, record_json) VALUES (?1, ?2, ?3) ON CONFLICT(project_id, task_id) DO UPDATE SET record_json = excluded.record_json",
                params![snapshot.project_id.0, task_id, record_json],
            )?;
        }
        if let Some((task_id, worker_id, record_json)) = claim {
            transaction.execute(
                "INSERT INTO m8_worker_task_claims (project_id, task_id, worker_id, record_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(project_id, task_id) DO UPDATE SET worker_id = excluded.worker_id, record_json = excluded.record_json",
                params![snapshot.project_id.0, task_id, worker_id, record_json],
            )?;
        }
        if let Some((message_id, task_id, worker_id, record_json)) = handoff {
            transaction.execute(
                "INSERT INTO m8_worker_handoffs (project_id, message_id, task_id, worker_id, record_json) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(project_id, message_id) DO UPDATE SET task_id = excluded.task_id, worker_id = excluded.worker_id, record_json = excluded.record_json",
                params![snapshot.project_id.0, message_id, task_id, worker_id, record_json],
            )?;
        }
        if let Some((acknowledgement_id, message_id, task_id, worker_id, record_json)) =
            acknowledgement
        {
            transaction.execute(
                "INSERT INTO m8_worker_handoff_acknowledgements (project_id, acknowledgement_id, message_id, task_id, worker_id, record_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![snapshot.project_id.0, acknowledgement_id, message_id, task_id, worker_id, record_json],
            )?;
        }
        transaction.commit()
    }

    pub fn save_coordination_task(
        &self,
        project_id: &str,
        task: &CoordinationTask,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(task).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        self.connection.execute(
            "INSERT INTO m8_coordination_tasks (project_id, task_id, record_json)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(project_id, task_id) DO UPDATE SET record_json = excluded.record_json",
            params![project_id, task.task_id, record_json],
        )?;
        Ok(())
    }

    pub fn load_coordination_task(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> rusqlite::Result<Option<CoordinationTask>> {
        self.connection
            .query_row(
                "SELECT record_json FROM m8_coordination_tasks WHERE project_id = ?1 AND task_id = ?2",
                params![project_id, task_id],
                |row| {
                    let record_json: String = row.get(0)?;
                    serde_json::from_str(&record_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
                },
            )
            .optional()
    }

    pub fn save_worker_handoff(
        &self,
        project_id: &str,
        handoff: &WorkerHandoffRecord,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(handoff).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        self.connection.execute(
            "INSERT INTO m8_worker_handoffs (project_id, message_id, task_id, worker_id, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(project_id, message_id) DO UPDATE SET record_json = excluded.record_json",
            params![project_id, handoff.message_id, handoff.task_id, handoff.worker_id, record_json],
        )?;
        Ok(())
    }

    pub fn save_worker_handoff_acknowledgement(
        &self,
        project_id: &str,
        acknowledgement: &WorkerHandoffAcknowledgement,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(acknowledgement).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        self.connection.execute(
            "INSERT INTO m8_worker_handoff_acknowledgements
             (project_id, acknowledgement_id, message_id, task_id, worker_id, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(project_id, acknowledgement_id) DO UPDATE SET record_json = excluded.record_json",
            params![
                project_id,
                acknowledgement.acknowledgement_id,
                acknowledgement.message_id,
                acknowledgement.task_id,
                acknowledgement.worker_id,
                record_json,
            ],
        )?;
        Ok(())
    }

    pub fn load_worker_handoff_acknowledgement(
        &self,
        project_id: &str,
        acknowledgement_id: &str,
    ) -> rusqlite::Result<Option<WorkerHandoffAcknowledgement>> {
        self.connection
            .query_row(
                "SELECT record_json FROM m8_worker_handoff_acknowledgements
                 WHERE project_id = ?1 AND acknowledgement_id = ?2",
                params![project_id, acknowledgement_id],
                |row| {
                    let record_json: String = row.get(0)?;
                    serde_json::from_str(&record_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
                },
            )
            .optional()
    }

    pub fn load_worker_handoff(
        &self,
        project_id: &str,
        message_id: &str,
    ) -> rusqlite::Result<Option<WorkerHandoffRecord>> {
        self.connection
            .query_row(
                "SELECT record_json FROM m8_worker_handoffs WHERE project_id = ?1 AND message_id = ?2",
                params![project_id, message_id],
                |row| {
                    let record_json: String = row.get(0)?;
                    serde_json::from_str(&record_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
                },
            )
            .optional()
    }

    pub fn save_m6_policy_event(
        &self,
        project_id: &ProjectId,
        decision: &PolicyDecision,
    ) -> rusqlite::Result<()> {
        let decision_json = serde_json::to_string(decision).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        self.connection.execute(
            "INSERT INTO m6_policy_events (decision_id, project_id, worker_id, request_id, outcome, decision_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(decision_id) DO UPDATE SET outcome=excluded.outcome, decision_json=excluded.decision_json",
            params![
                decision.decision_id,
                project_id.0,
                decision.worker_id,
                decision.request_id,
                format!("{:?}", decision.outcome),
                decision_json,
            ],
        )?;
        Ok(())
    }

    pub fn load_m6_policy_events(
        &self,
        project_id: &ProjectId,
    ) -> rusqlite::Result<Vec<PolicyDecision>> {
        let mut statement = self.connection.prepare(
            "SELECT decision_json FROM m6_policy_events WHERE project_id = ?1 ORDER BY decision_id",
        )?;
        let rows = statement.query_map(params![project_id.0], |row| {
            let decision_json: String = row.get(0)?;
            serde_json::from_str(&decision_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })?;
        rows.collect()
    }

    pub fn save_worker_execution_record(
        &self,
        project_id: &ProjectId,
        record: &WorkerExecutionRecord,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(record).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        self.connection.execute(
            "INSERT INTO m5_worker_execution_records (project_id, task_id, worker_id, record_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(project_id, task_id) DO UPDATE SET worker_id = excluded.worker_id, record_json = excluded.record_json",
            params![project_id.0, record.task_id().0, record.worker_id(), record_json],
        )?;
        Ok(())
    }

    pub fn load_worker_execution_record(
        &self,
        project_id: &ProjectId,
        task_id: &str,
    ) -> rusqlite::Result<Option<WorkerExecutionRecord>> {
        self.connection
            .query_row(
                "SELECT record_json FROM m5_worker_execution_records WHERE project_id = ?1 AND task_id = ?2",
                params![project_id.0, task_id],
                |row| {
                    let record_json: String = row.get(0)?;
                    serde_json::from_str(&record_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
                },
            )
            .optional()
    }

    pub fn save_background_run(&self, record: &BackgroundRunRecord) -> rusqlite::Result<()> {
        record.validate().map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        let record_json = serde_json::to_string(record).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        self.connection.execute(
            "INSERT INTO m7_background_runs (run_id, project_id, task_id, worker_id, state, record_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(run_id) DO UPDATE SET
               project_id = excluded.project_id,
               task_id = excluded.task_id,
               worker_id = excluded.worker_id,
               state = excluded.state,
               record_json = excluded.record_json",
            params![
                record.run_id,
                record.project_id,
                record.task_id,
                record.worker_id,
                serde_json::to_string(&record.state).unwrap_or_else(|_| "null".into()),
                record_json,
            ],
        )?;
        Ok(())
    }

    pub fn load_background_run(
        &self,
        run_id: &str,
    ) -> rusqlite::Result<Option<BackgroundRunRecord>> {
        self.connection
            .query_row(
                "SELECT record_json FROM m7_background_runs WHERE run_id = ?1",
                params![run_id],
                |row| {
                    let record_json: String = row.get(0)?;
                    serde_json::from_str(&record_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
                },
            )
            .optional()
    }

    pub fn save_m108_sync_record(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        projection_json: &str,
        evidence_json: &str,
        last_event_sequence: u64,
    ) -> rusqlite::Result<()> {
        self.connection.execute("INSERT INTO m108_preview_sync_records (project_id,task_id,projection_json,evidence_json,last_event_sequence) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(project_id,task_id) DO UPDATE SET projection_json=excluded.projection_json,evidence_json=excluded.evidence_json,last_event_sequence=excluded.last_event_sequence", params![project_id.0,task_id,projection_json,evidence_json,last_event_sequence])?;
        Ok(())
    }

    pub fn load_m108_sync_record(
        &self,
        project_id: &ProjectId,
        task_id: &str,
    ) -> rusqlite::Result<Option<(String, String, u64)>> {
        self.connection.query_row("SELECT projection_json,evidence_json,last_event_sequence FROM m108_preview_sync_records WHERE project_id=?1 AND task_id=?2", params![project_id.0,task_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()
    }

    pub fn load_m108_event_jsons(
        &self,
        project_id: &ProjectId,
        task_id: &str,
    ) -> rusqlite::Result<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT event_json FROM m108_preview_sync_events WHERE project_id = ?1 AND task_id = ?2 ORDER BY event_sequence ASC",
        )?;
        let rows = statement.query_map(params![project_id.0, task_id], |row| row.get(0))?;
        rows.collect()
    }

    pub fn load_m108_evidence_jsons(
        &self,
        project_id: &ProjectId,
        task_id: &str,
    ) -> rusqlite::Result<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT evidence_json FROM m108_preview_sync_evidence WHERE project_id = ?1 AND task_id = ?2 ORDER BY event_sequence ASC",
        )?;
        let rows = statement.query_map(params![project_id.0, task_id], |row| row.get(0))?;
        rows.collect()
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

    // ─────────────────────────── M118 platform records ───────────────────

    pub fn save_platform_preflight(
        &self,
        project_id: &ProjectId,
        task_id: &str,
        record: &nirman_domain::EnvironmentCapabilityRecord,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(record).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        self.connection.execute(
            "INSERT INTO m118_platform_preflights (project_id, task_id, environment_id, record_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(project_id, task_id) DO UPDATE SET environment_id = excluded.environment_id, record_json = excluded.record_json",
            params![project_id.0, task_id, record.environment_id, record_json],
        )?;
        Ok(())
    }

    pub fn load_platform_preflight(
        &self,
        project_id: &ProjectId,
        task_id: &str,
    ) -> rusqlite::Result<Option<nirman_domain::EnvironmentCapabilityRecord>> {
        self.connection
            .query_row(
                "SELECT record_json FROM m118_platform_preflights WHERE project_id = ?1 AND task_id = ?2",
                params![project_id.0, task_id],
                |row| {
                    let json: String = row.get(0)?;
                    serde_json::from_str(&json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::other(error.to_string())),
                        )
                    })
                },
            )
            .optional()
    }

    pub fn save_platform_gate_record(
        &self,
        project_id: &ProjectId,
        record: &nirman_domain::BuildGateRecord,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(record).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        let stage = serde_json::to_string(&record.stage)
            .expect("BuildGateStage serialization is infallible")
            .trim_matches('"')
            .to_string();
        self.connection.execute(
            "INSERT INTO m118_platform_gate_records (project_id, gate_id, stage, record_json) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(project_id, gate_id) DO UPDATE SET stage = excluded.stage, record_json = excluded.record_json",
            params![project_id.0, record.gate_id, stage, record_json],
        )?;
        Ok(())
    }

    pub fn load_platform_gate_records(
        &self,
        project_id: &ProjectId,
    ) -> rusqlite::Result<Vec<nirman_domain::BuildGateRecord>> {
        let mut statement = self
            .connection
            .prepare("SELECT record_json FROM m118_platform_gate_records WHERE project_id = ?1 ORDER BY gate_id")?;
        let rows = statement
            .query_map(params![project_id.0], |row| {
                let json: String = row.get(0)?;
                serde_json::from_str(&json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::other(error.to_string())),
                    )
                })
            })
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn save_platform_blocked_decision(
        &self,
        project_id: &ProjectId,
        decision: &nirman_domain::PlatformBlockedDecision,
    ) -> rusqlite::Result<()> {
        let record_json = serde_json::to_string(decision).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        let stage = serde_json::to_string(&decision.stage)
            .expect("BuildGateStage serialization is infallible")
            .trim_matches('"')
            .to_string();
        self.connection.execute(
            "INSERT INTO m118_platform_blocked_decisions (project_id, decision_id, task_id, stage, record_json) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(project_id, decision_id, task_id) DO UPDATE SET stage = excluded.stage, record_json = excluded.record_json",
            params![project_id.0, decision.decision_id, decision.task_id, stage, record_json],
        )?;
        Ok(())
    }

    pub fn load_platform_blocked_decisions(
        &self,
        project_id: &ProjectId,
        task_id: &str,
    ) -> rusqlite::Result<Vec<nirman_domain::PlatformBlockedDecision>> {
        let mut statement = self
            .connection
            .prepare("SELECT record_json FROM m118_platform_blocked_decisions WHERE project_id = ?1 AND task_id = ?2 ORDER BY decision_id")?;
        let rows = statement
            .query_map(params![project_id.0, task_id], |row| {
                let json: String = row.get(0)?;
                serde_json::from_str(&json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::other(error.to_string())),
                    )
                })
            })
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut decisions = Vec::new();
        for row in rows {
            decisions.push(row?);
        }
        Ok(decisions)
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
        "Synthesizing" => Ok(ProductLifecycleState::Synthesizing),
        "Implementing" => Ok(ProductLifecycleState::Implementing),
        "Paused" => Ok(ProductLifecycleState::Paused),
        "Previewing" => Ok(ProductLifecycleState::Previewing),
        "Validating" => Ok(ProductLifecycleState::Validating),
        "Recovering" => Ok(ProductLifecycleState::Recovering),
        "Packaging" => Ok(ProductLifecycleState::Packaging),
        "Completed" => Ok(ProductLifecycleState::Completed),
        "Blocked" => Ok(ProductLifecycleState::Blocked),
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
        "Simulated" => Ok(PreviewTruth::Simulated),
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
            worker_projection: None,
            artifact_projection: None,
            evidence_projection: None,
            delivery_projection: None,
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

#[cfg(test)]
mod m7_tests {
    use super::*;
    use nirman_supervisor::{BackgroundRunRecord, BackgroundRunState, M7_SCHEMA_VERSION};

    #[test]
    fn m7_atomic_lifecycle_transaction_rolls_back_on_event_conflict() {
        let ledger = Ledger::open_in_memory().expect("ledger");
        let project_id = ProjectId("project-m7-atomic".into());
        let snapshot = ProjectionSnapshot {
            project_id: project_id.clone(),
            projection_revision: Revision(1),
            task_state: ProductLifecycleState::Planning,
            continuity_state: BackgroundContinuityState::ActiveBackground,
            preview_truth: PreviewTruth::Predicted,
            current_source_revision: Revision(0),
            last_event_sequence: 1,
            last_known_good_ref: None,
            worker_projection: None,
            artifact_projection: None,
            evidence_projection: None,
            delivery_projection: None,
        };
        let event = ControlEvent {
            event_id: "event-m7-atomic".into(),
            sequence: 1,
            project_id: project_id.clone(),
            task_id: Some(TaskId("task-m7-atomic".into())),
            kind: "TaskStarted".into(),
            payload: "start".into(),
            source_revision: Revision(0),
        };
        let record = BackgroundRunRecord {
            schema_version: M7_SCHEMA_VERSION,
            run_id: "run-project-m7-atomic-task-m7-atomic".into(),
            project_id: project_id.0.clone(),
            task_id: "task-m7-atomic".into(),
            worker_id: "worker-single".into(),
            checkpoint_id: None,
            state: BackgroundRunState::Running,
            last_heartbeat_epoch_seconds: 10,
            attempt: 1,
            recovery_action: None,
            failure_fingerprint: None,
            notification_kind: None,
        };
        ledger
            .commit_event_projection_and_command_and_background_run(
                &event,
                &snapshot,
                "command-m7-atomic",
                Some("idempotency-m7-atomic"),
                "fingerprint-m7-atomic",
                "correlation-m7-atomic",
                &serde_json::to_string(&snapshot).expect("snapshot json"),
                &record,
            )
            .expect("first atomic commit");
        let error = ledger
            .commit_event_projection_and_command_and_background_run(
                &event,
                &snapshot,
                "command-m7-atomic-2",
                Some("idempotency-m7-atomic-2"),
                "fingerprint-m7-atomic-2",
                "correlation-m7-atomic-2",
                &serde_json::to_string(&snapshot).expect("snapshot json"),
                &record,
            )
            .expect_err("duplicate event sequence must fail");
        assert!(matches!(error, rusqlite::Error::SqliteFailure(_, _)));
        assert_eq!(ledger.event_count().expect("event count"), 1);
        assert_eq!(ledger.latest_sequence().expect("latest sequence"), 1);
        assert_eq!(
            ledger.projection_revision(&project_id).expect("revision"),
            Some(Revision(1))
        );
        assert_eq!(
            ledger
                .load_background_run(&record.run_id)
                .expect("run lookup"),
            Some(record)
        );
        assert!(ledger
            .load_command_result(
                &project_id,
                "command-m7-atomic-2",
                Some("idempotency-m7-atomic-2")
            )
            .expect("command lookup")
            .is_none());
    }

    #[test]
    fn background_run_record_round_trips_through_sqlite() {
        let ledger = Ledger::open_in_memory().expect("ledger");
        let record = BackgroundRunRecord {
            schema_version: M7_SCHEMA_VERSION,
            run_id: "run-m7-1".into(),
            project_id: "project-m7-1".into(),
            task_id: "task-m7-1".into(),
            worker_id: "worker-m7-1".into(),
            checkpoint_id: Some("checkpoint-m7-1".into()),
            state: BackgroundRunState::Recovering,
            last_heartbeat_epoch_seconds: 100,
            attempt: 2,
            recovery_action: Some(nirman_supervisor::RecoveryAction::ResumeFromCheckpoint),
            failure_fingerprint: Some("fingerprint-m7".into()),
            notification_kind: Some("worker-stale".into()),
        };
        ledger.save_background_run(&record).expect("save");
        assert_eq!(
            ledger.load_background_run("run-m7-1").expect("load"),
            Some(record)
        );
    }
}

#[cfg(test)]
mod m8_tests {
    use super::*;
    use nirman_domain::Revision;
    use nirman_workers::{
        CoordinationTask, HandoffStatus, WorkerHandoffAcknowledgement, WorkerRole,
        WorkspaceIsolation, M8_SCHEMA_VERSION,
    };

    #[test]
    fn coordination_task_and_acknowledgement_round_trip_through_sqlite() {
        let ledger = Ledger::open_in_memory().expect("ledger");
        let task = CoordinationTask {
            schema_version: M8_SCHEMA_VERSION,
            task_id: "task-m8-a".into(),
            parent_task_id: "root-task".into(),
            worker_id: "worker-a".into(),
            role: WorkerRole::Implementation,
            capability_ceiling: vec!["android.build".into()],
            workspace_root: "/workspace/a".into(),
            parent_workspace_root: "/workspace/root".into(),
            isolation: WorkspaceIsolation::GitWorktree,
            dependencies: vec![],
            expected_source_revision: Revision(3),
            required_evidence: vec!["build-evidence".into()],
        };
        let acknowledgement = WorkerHandoffAcknowledgement {
            acknowledgement_id: "ack-m8-a".into(),
            message_id: "message-m8-a".into(),
            task_id: task.task_id.clone(),
            worker_id: task.worker_id.clone(),
            status: HandoffStatus::Accepted,
            reconciliation_checkpoint: None,
            reason: "accepted by scoped coordinator".into(),
        };
        ledger
            .save_coordination_task("project-m8", &task)
            .expect("save task");
        ledger
            .save_worker_handoff_acknowledgement("project-m8", &acknowledgement)
            .expect("save acknowledgement");
        assert_eq!(
            ledger
                .load_coordination_task("project-m8", "task-m8-a")
                .expect("load task"),
            Some(task)
        );
        assert_eq!(
            ledger
                .load_worker_handoff_acknowledgement("project-m8", "ack-m8-a")
                .expect("load acknowledgement"),
            Some(acknowledgement)
        );
    }
}
