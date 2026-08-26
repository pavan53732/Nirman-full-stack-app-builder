#![forbid(unsafe_code)]

use nirman_control_plane::ControlPlane;
use nirman_domain::{CommandEnvelope, CommandKind, ProjectId, Revision};

fn main() {
    let mut control_plane = ControlPlane::new(ProjectId("project-0001".into()));
    let command = CommandEnvelope {
        command_id: "bootstrap-command".into(),
        project_id: ProjectId("project-0001".into()),
        task_id: None,
        kind: CommandKind::SubmitInstruction,
        payload: "Build an Android application from the user intent".into(),
        expected_projection_revision: Revision(0),
        idempotency_key: Some("bootstrap-command".into()),
    };

    match control_plane.accept(command) {
        Ok(snapshot) => println!(
            "Nirman control plane ready at projection revision {:?}",
            snapshot.projection_revision
        ),
        Err(error) => eprintln!("Nirman control plane rejected bootstrap command: {error}"),
    }
}
