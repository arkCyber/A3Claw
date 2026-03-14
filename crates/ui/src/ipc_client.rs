#![allow(dead_code, unused_imports)]
//! IPC client manager for the UI process.
//!
//! When the UI runs as a separate process (detached from the sandbox), this
//! module connects to the sandbox IPC server to receive events and send
//! control commands.
//!
//! When the UI and sandbox run in the same process (embedded mode), a direct
//! `flume` channel is used instead of IPC.

#[allow(unused_imports)]
use anyhow::Result;
use openclaw_sandbox::ipc::IpcClient;
use openclaw_security::{ControlCommand, SandboxEvent};
use tracing::{info, warn};

/// Describes how the UI is connected to the sandbox.
pub enum ConnectionMode {
    /// Embedded mode: UI and sandbox share the same process; channels are used directly.
    Embedded {
        event_rx: flume::Receiver<SandboxEvent>,
        control_tx: flume::Sender<ControlCommand>,
    },
    /// IPC mode: UI and sandbox are separate processes communicating over a Unix socket or Named Pipe.
    Ipc {
        event_rx: flume::Receiver<SandboxEvent>,
        control_tx: flume::Sender<ControlCommand>,
    },
}

impl ConnectionMode {
    /// Returns the event receiver channel.
    pub fn event_rx(&self) -> &flume::Receiver<SandboxEvent> {
        match self {
            ConnectionMode::Embedded { event_rx, .. } => event_rx,
            ConnectionMode::Ipc { event_rx, .. } => event_rx,
        }
    }

    /// Returns the control command sender channel.
    pub fn control_tx(&self) -> &flume::Sender<ControlCommand> {
        match self {
            ConnectionMode::Embedded { control_tx, .. } => control_tx,
            ConnectionMode::Ipc { control_tx, .. } => control_tx,
        }
    }
}

/// Tries to connect to an external sandbox process (IPC mode).
///
/// Falls back to embedded mode if the socket does not exist or the connection fails.
pub async fn connect_or_embed(
    embedded_event_rx: flume::Receiver<SandboxEvent>,
    embedded_control_tx: flume::Sender<ControlCommand>,
) -> ConnectionMode {
    let socket_path = openclaw_sandbox::ipc::ipc_socket_path();
    if socket_path.exists() {
        info!("Sandbox IPC socket detected, connecting: {:?}", socket_path);
        let client = IpcClient::new();
        match client.connect().await {
            Ok((event_rx, control_tx)) => {
                info!("IPC connection established — using separate-process mode");
                return ConnectionMode::Ipc { event_rx, control_tx };
            }
            Err(e) => {
                warn!("IPC connection failed, falling back to embedded mode: {}", e);
            }
        }
    } else {
        info!("No sandbox IPC socket found — using embedded mode");
    }

    ConnectionMode::Embedded {
        event_rx: embedded_event_rx,
        control_tx: embedded_control_tx,
    }
}
