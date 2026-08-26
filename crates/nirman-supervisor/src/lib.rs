//! Supervisor lifecycle primitives for M2.

#![forbid(unsafe_code)]

use nirman_domain::BackgroundContinuityState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupervisorState {
    Starting,
    Running,
    Reconnecting,
    Reconciling,
    UserRequired,
    SafelyFailed,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaseFence {
    pub lease_id: String,
    pub fence_token: u64,
    pub owner_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupervisorSnapshot {
    pub state: SupervisorState,
    pub continuity: BackgroundContinuityState,
    pub heartbeat_count: u64,
    pub latest_checkpoint_id: Option<String>,
    pub active_fence: Option<LeaseFence>,
}

#[derive(Debug)]
pub struct Supervisor {
    snapshot: SupervisorSnapshot,
}

impl Supervisor {
    pub fn start() -> Self {
        Self {
            snapshot: SupervisorSnapshot {
                state: SupervisorState::Starting,
                continuity: BackgroundContinuityState::ActiveBackground,
                heartbeat_count: 0,
                latest_checkpoint_id: None,
                active_fence: None,
            },
        }
    }

    pub fn heartbeat(&mut self) {
        self.snapshot.heartbeat_count = self.snapshot.heartbeat_count.saturating_add(1);
        self.snapshot.state = SupervisorState::Running;
    }

    pub fn register_lease(
        &mut self,
        lease_id: impl Into<String>,
        owner_id: impl Into<String>,
        fence_token: u64,
    ) {
        self.snapshot.active_fence = Some(LeaseFence {
            lease_id: lease_id.into(),
            owner_id: owner_id.into(),
            fence_token,
        });
    }

    pub fn reconcile_after_restart(&mut self, checkpoint_id: impl Into<String>) {
        self.snapshot.state = SupervisorState::Reconciling;
        self.snapshot.continuity = BackgroundContinuityState::Reconciling;
        self.snapshot.latest_checkpoint_id = Some(checkpoint_id.into());
    }

    pub fn mark_user_required(&mut self) {
        self.snapshot.state = SupervisorState::UserRequired;
        self.snapshot.continuity = BackgroundContinuityState::UserRequired;
    }

    pub fn safe_fail(&mut self) {
        self.snapshot.state = SupervisorState::SafelyFailed;
        self.snapshot.continuity = BackgroundContinuityState::SafelyFailed;
    }

    pub fn snapshot(&self) -> SupervisorSnapshot {
        self.snapshot.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_requires_reconciliation_before_running() {
        let mut supervisor = Supervisor::start();
        supervisor.register_lease("lease-1", "worker-1", 1);
        supervisor.reconcile_after_restart("checkpoint-1");
        assert_eq!(supervisor.snapshot().state, SupervisorState::Reconciling);
        assert_eq!(
            supervisor.snapshot().continuity,
            BackgroundContinuityState::Reconciling
        );
        supervisor.heartbeat();
        assert_eq!(supervisor.snapshot().state, SupervisorState::Running);
    }
}
