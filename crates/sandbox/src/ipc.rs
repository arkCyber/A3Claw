//! IPC communication layer between the sandbox and UI processes.
//!
//! **Transport**: Unix Domain Socket (macOS / Linux) or Named Pipe (Windows).
//! **Protocol**: newline-delimited JSON (NDJSON) — one [`IpcFrame`] per line.
//!
//! Message directions:
//! - Sandbox → UI : [`IpcFrame::Event`] (sandbox event push)
//! - UI → Sandbox : [`IpcFrame::Control`] (control command)
//!
//! Socket path: `$TMPDIR/openclaw-plus/ipc.sock`

use anyhow::{Context, Result};
use openclaw_security::{ControlCommand, SandboxEvent};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};

/// Bidirectional IPC message frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcFrame {
    /// Sandbox → UI: a sandbox event to display in the monitoring UI.
    Event(SandboxEvent),
    /// UI → Sandbox: a control command (allow / deny / terminate).
    Control(ControlCommand),
    /// Keep-alive ping.
    Ping,
    /// Keep-alive pong reply.
    Pong,
}

/// Returns the platform IPC socket path (`$TMPDIR/openclaw-plus/ipc.sock`).
pub fn ipc_socket_path() -> PathBuf {
    std::env::temp_dir()
        .join("openclaw-plus")
        .join("ipc.sock")
}

/// IPC server running inside the sandbox process.
///
/// Listens for UI connections, receives control commands, and sends events.
pub struct IpcServer {
    socket_path: PathBuf,
    event_rx: flume::Receiver<SandboxEvent>,
    control_tx: flume::Sender<ControlCommand>,
}

impl IpcServer {
    pub fn new(
        event_rx: flume::Receiver<SandboxEvent>,
        control_tx: flume::Sender<ControlCommand>,
    ) -> Self {
        Self {
            socket_path: ipc_socket_path(),
            event_rx,
            control_tx,
        }
    }

    /// Starts the IPC server loop (run in a dedicated Tokio task).
    pub async fn serve(self) -> Result<()> {
        // Ensure the socket directory exists.
        if let Some(parent) = self.socket_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Remove any stale socket file from a previous run.
        #[cfg(unix)]
        {
            let _ = tokio::fs::remove_file(&self.socket_path).await;
        }

        #[cfg(unix)]
        {
            use tokio::net::UnixListener;
            let listener = UnixListener::bind(&self.socket_path)
                .context("Failed to bind IPC Unix socket")?;
            info!("IPC server listening: {:?}", self.socket_path);

            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        info!("UI process connected to IPC");
                        let event_rx = self.event_rx.clone();
                        let control_tx = self.control_tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_unix_connection(stream, event_rx, control_tx).await {
                                error!("IPC connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("IPC accept error: {}", e);
                    }
                }
            }
        }

        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::{PipeMode, ServerOptions};
            let pipe_name = r"\\.\pipe\openclaw-plus-ipc";
            info!("IPC Named Pipe server started: {}", pipe_name);
            loop {
                let server = ServerOptions::new()
                    .pipe_mode(PipeMode::Byte)
                    .create(pipe_name)
                    .context("Failed to create Named Pipe")?;
                server.connect().await.context("Failed to wait for Named Pipe connection")?;
                info!("UI process connected to IPC Named Pipe");
                let event_rx = self.event_rx.clone();
                let control_tx = self.control_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_windows_pipe(server, event_rx, control_tx).await {
                        error!("IPC Named Pipe error: {}", e);
                    }
                });
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            anyhow::bail!("IPC is not supported on this platform");
        }
    }
}

/// IPC client running inside the UI process.
///
/// Connects to the sandbox IPC server, receives events, and sends commands.
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    pub fn new() -> Self {
        Self {
            socket_path: ipc_socket_path(),
        }
    }

    /// Connects to the sandbox IPC server.
    ///
    /// Returns `(event_rx, control_tx)` for receiving events and sending commands.
    pub async fn connect(
        &self,
    ) -> Result<(
        flume::Receiver<SandboxEvent>,
        flume::Sender<ControlCommand>,
    )> {
        let (event_tx, event_rx) = flume::unbounded::<SandboxEvent>();
        let (control_tx, control_rx) = flume::unbounded::<ControlCommand>();

        #[cfg(unix)]
        {
            use tokio::net::UnixStream;

            // Retry until the sandbox process is ready.
            let mut retries = 0;
            let stream = loop {
                match UnixStream::connect(&self.socket_path).await {
                    Ok(s) => break s,
                    Err(e) if retries < 10 => {
                        warn!("IPC connection failed, retry {}/10: {}", retries + 1, e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                        retries += 1;
                    }
                    Err(e) => return Err(e).context("IPC connection failed"),
                }
            };

            info!("Connected to sandbox IPC: {:?}", self.socket_path);

            tokio::spawn(async move {
                if let Err(e) = run_client_unix(stream, event_tx, control_rx).await {
                    error!("IPC client error: {}", e);
                }
            });
        }

        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ClientOptions;
            let pipe_name = r"\\.\pipe\openclaw-plus-ipc";
            let mut retries = 0;
            let client = loop {
                match ClientOptions::new().open(pipe_name) {
                    Ok(c) => break c,
                    Err(e) if retries < 10 => {
                        warn!("Named Pipe connection failed, retry {}/10: {}", retries + 1, e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                        retries += 1;
                    }
                    Err(e) => return Err(e).context("Named Pipe connection failed"),
                }
            };
            tokio::spawn(async move {
                if let Err(e) = run_client_windows(client, event_tx, control_rx).await {
                    error!("IPC client error: {}", e);
                }
            });
        }

        Ok((event_rx, control_tx))
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new()
    }
}

// ── Unix connection handler ───────────────────────────────────

#[cfg(unix)]
async fn handle_unix_connection(
    stream: tokio::net::UnixStream,
    event_rx: flume::Receiver<SandboxEvent>,
    control_tx: flume::Sender<ControlCommand>,
) -> Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();

    // Receive control commands (UI → Sandbox).
    let control_tx_clone = control_tx.clone();
    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            match serde_json::from_str::<IpcFrame>(&line) {
                Ok(IpcFrame::Control(cmd)) => {
                    debug!("IPC received control command: {:?}", cmd);
                    let _ = control_tx_clone.send_async(cmd).await;
                }
                Ok(IpcFrame::Ping) => {
                    debug!("IPC Ping received");
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("IPC parse error: {} | raw: {}", e, line);
                }
            }
        }
    });

    // Forward sandbox events (Sandbox → UI).
    let write_task = tokio::spawn(async move {
        loop {
            match event_rx.recv_async().await {
                Ok(event) => {
                    let frame = IpcFrame::Event(event);
                    match serde_json::to_string(&frame) {
                        Ok(mut json) => {
                            json.push('\n');
                            if let Err(e) = writer.write_all(json.as_bytes()).await {
                                error!("IPC write error: {}", e);
                                break;
                            }
                        }
                        Err(e) => error!("IPC serialisation error: {}", e),
                    }
                }
                Err(_) => break,
            }
        }
    });

    let _ = tokio::join!(read_task, write_task);
    Ok(())
}

#[cfg(unix)]
async fn run_client_unix(
    stream: tokio::net::UnixStream,
    event_tx: flume::Sender<SandboxEvent>,
    control_rx: flume::Receiver<ControlCommand>,
) -> Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();

    // Receive sandbox events (Sandbox → UI).
    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            match serde_json::from_str::<IpcFrame>(&line) {
                Ok(IpcFrame::Event(event)) => {
                    let _ = event_tx.send_async(event).await;
                }
                Ok(IpcFrame::Pong) => debug!("IPC Pong received"),
                Ok(_) => {}
                Err(e) => warn!("IPC client parse error: {}", e),
            }
        }
    });

    // Send control commands (UI → Sandbox).
    let write_task = tokio::spawn(async move {
        loop {
            match control_rx.recv_async().await {
                Ok(cmd) => {
                    let frame = IpcFrame::Control(cmd);
                    match serde_json::to_string(&frame) {
                        Ok(mut json) => {
                            json.push('\n');
                            if let Err(e) = writer.write_all(json.as_bytes()).await {
                                error!("IPC client write error: {}", e);
                                break;
                            }
                        }
                        Err(e) => error!("IPC client serialisation error: {}", e),
                    }
                }
                Err(_) => break,
            }
        }
    });

    let _ = tokio::join!(read_task, write_task);
    Ok(())
}

// ── Windows Named Pipe handler (mirrors Unix structure) ──────

#[cfg(windows)]
async fn handle_windows_pipe(
    pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    event_rx: flume::Receiver<SandboxEvent>,
    control_tx: flume::Sender<ControlCommand>,
) -> Result<()> {
    let (reader, mut writer) = tokio::io::split(pipe);
    let mut lines = BufReader::new(reader).lines();

    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(IpcFrame::Control(cmd)) = serde_json::from_str::<IpcFrame>(&line) {
                let _ = control_tx.send_async(cmd).await;
            }
        }
    });

    let write_task = tokio::spawn(async move {
        loop {
            match event_rx.recv_async().await {
                Ok(event) => {
                    if let Ok(mut json) = serde_json::to_string(&IpcFrame::Event(event)) {
                        json.push('\n');
                        if writer.write_all(json.as_bytes()).await.is_err() { break; }
                    }
                }
                Err(_) => break,
            }
        }
    });

    let _ = tokio::join!(read_task, write_task);
    Ok(())
}

#[cfg(windows)]
async fn run_client_windows(
    client: tokio::net::windows::named_pipe::NamedPipeClient,
    event_tx: flume::Sender<SandboxEvent>,
    control_rx: flume::Receiver<ControlCommand>,
) -> Result<()> {
    let (reader, mut writer) = tokio::io::split(client);
    let mut lines = BufReader::new(reader).lines();

    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(IpcFrame::Event(event)) = serde_json::from_str::<IpcFrame>(&line) {
                let _ = event_tx.send_async(event).await;
            }
        }
    });

    let write_task = tokio::spawn(async move {
        loop {
            match control_rx.recv_async().await {
                Ok(cmd) => {
                    if let Ok(mut json) = serde_json::to_string(&IpcFrame::Control(cmd)) {
                        json.push('\n');
                        if writer.write_all(json.as_bytes()).await.is_err() { break; }
                    }
                }
                Err(_) => break,
            }
        }
    });

    let _ = tokio::join!(read_task, write_task);
    Ok(())
}
