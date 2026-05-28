use agent_client_protocol::schema::{
    ContentBlock, InitializeRequest, ListSessionsRequest, ProtocolVersion,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionNotification, SessionUpdate,
};
use agent_client_protocol::{Client, ConnectionTo};
use anyhow::Result;
use goose_sdk::custom_requests::*;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Debug, Clone)]
pub enum AgentMessage {
    TextChunk(String),
    ToolCallStarted { title: String, id: String },
    ToolCallUpdate { id: String, status: String },
    ResponseComplete,
    Error(String),
    SessionCreated(String),
    SessionsList(Vec<SessionInfo>),
    ProvidersList(Vec<ProviderInfo>),
    ExtensionsList(Vec<ExtensionInfo>),
    Initialized,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SessionInfo {
    pub id: String,
    pub title: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub configured: bool,
    pub description: String,
    pub models: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExtensionInfo {
    pub name: String,
    pub enabled: bool,
    pub ext_type: String,
}

#[derive(Debug, Clone)]
pub enum ClientCommand {
    Initialize,
    CreateSession,
    SendPrompt(String),
    ListSessions,
    ListProviders,
    ListExtensions,
    SaveDefaults { provider: String, model: String },
    ToggleExtension { key: String, enabled: bool },
    Shutdown,
}

pub fn spawn_acp_client(
    goose_bin: PathBuf,
) -> (
    mpsc::UnboundedSender<ClientCommand>,
    mpsc::UnboundedReceiver<AgentMessage>,
) {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (msg_tx, msg_rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        if let Err(e) = run_client(goose_bin, cmd_rx, msg_tx.clone()).await {
            let _ = msg_tx.send(AgentMessage::Error(e.to_string()));
        }
    });

    (cmd_tx, msg_rx)
}

async fn run_client(
    goose_bin: PathBuf,
    mut cmd_rx: mpsc::UnboundedReceiver<ClientCommand>,
    msg_tx: mpsc::UnboundedSender<AgentMessage>,
) -> Result<()> {
    let mut child = tokio::process::Command::new(&goose_bin)
        .arg("acp")
        .arg("--with-builtin")
        .arg("developer")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let child_stdin = child.stdin.take().expect("stdin piped");
    let child_stdout = child.stdout.take().expect("stdout piped");

    let transport =
        agent_client_protocol::ByteStreams::new(child_stdin.compat_write(), child_stdout.compat());

    let msg_tx_notif = msg_tx.clone();
    let msg_tx_perm = msg_tx.clone();

    Client
        .builder()
        .name("goose-tui")
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                handle_notification(&notification, &msg_tx_notif);
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder, _cx| {
                let _ =
                    msg_tx_perm.send(AgentMessage::TextChunk("\n⚡ Auto-approving tool\n".into()));
                let option_id = request.options.first().map(|opt| opt.option_id.clone());
                match option_id {
                    Some(id) => responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(id)),
                    )),
                    None => responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    )),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(
            transport,
            async move |cx: ConnectionTo<agent_client_protocol::Agent>| {
                run_command_loop(cx, &mut cmd_rx, &msg_tx).await
            },
        )
        .await?;

    let _ = child.kill().await;
    Ok(())
}

fn handle_notification(
    notification: &SessionNotification,
    msg_tx: &mpsc::UnboundedSender<AgentMessage>,
) {
    match &notification.update {
        SessionUpdate::AgentMessageChunk(chunk) => {
            if let ContentBlock::Text(text) = &chunk.content {
                let _ = msg_tx.send(AgentMessage::TextChunk(text.text.clone()));
            }
        }
        SessionUpdate::ToolCall(tool_call) => {
            let _ = msg_tx.send(AgentMessage::ToolCallStarted {
                title: tool_call.title.clone(),
                id: tool_call.tool_call_id.to_string(),
            });
        }
        SessionUpdate::ToolCallUpdate(update) => {
            let status = format!("{:?}", update.fields.status);
            let _ = msg_tx.send(AgentMessage::ToolCallUpdate {
                id: update.tool_call_id.to_string(),
                status,
            });
        }
        _ => {}
    }
}

async fn run_command_loop(
    cx: ConnectionTo<agent_client_protocol::Agent>,
    cmd_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
    msg_tx: &mpsc::UnboundedSender<AgentMessage>,
) -> Result<(), agent_client_protocol::Error> {
    // Wait for init command first
    while let Some(cmd) = cmd_rx.recv().await {
        if matches!(cmd, ClientCommand::Initialize) {
            break;
        }
    }

    // Initialize
    let _init = cx
        .send_request(InitializeRequest::new(ProtocolVersion::LATEST))
        .block_task()
        .await?;
    let _ = msg_tx.send(AgentMessage::Initialized);

    // Process remaining commands — session creation uses run_until
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            ClientCommand::CreateSession => {
                // Create a session; the session stays active until
                // a new CreateSession or Shutdown is received.
                let _ = msg_tx.send(AgentMessage::SessionCreated("active".into()));

                // Enter session loop
                cx.build_session_cwd()
                    .map_err(|_| agent_client_protocol::Error::internal_error())?
                    .block_task()
                    .run_until(async |mut session| {
                        // Process prompts until we get a non-prompt command
                        loop {
                            let Some(cmd) = cmd_rx.recv().await else {
                                return Ok(());
                            };
                            match cmd {
                                ClientCommand::SendPrompt(prompt) => {
                                    session.send_prompt(&prompt)?;
                                    let _text = session.read_to_string().await?;
                                    let _ = msg_tx.send(AgentMessage::ResponseComplete);
                                }
                                ClientCommand::Shutdown => return Ok(()),
                                ClientCommand::CreateSession => {
                                    // Break out to create a new session
                                    return Ok(());
                                }
                                other => {
                                    handle_non_session_cmd(&cx, other, msg_tx).await?;
                                }
                            }
                        }
                    })
                    .await?;
            }
            ClientCommand::Shutdown => break,
            ClientCommand::Initialize => {
                let _ = msg_tx.send(AgentMessage::Initialized);
            }
            other => {
                handle_non_session_cmd(&cx, other, msg_tx).await?;
            }
        }
    }
    Ok(())
}

async fn handle_non_session_cmd(
    cx: &ConnectionTo<agent_client_protocol::Agent>,
    cmd: ClientCommand,
    msg_tx: &mpsc::UnboundedSender<AgentMessage>,
) -> Result<(), agent_client_protocol::Error> {
    match cmd {
        ClientCommand::ListSessions => {
            let resp = cx
                .send_request(ListSessionsRequest::default())
                .block_task()
                .await?;
            let sessions = resp
                .sessions
                .into_iter()
                .map(|s| SessionInfo {
                    id: s.session_id.to_string(),
                    title: s.title.unwrap_or_default(),
                    updated_at: s.updated_at.unwrap_or_default(),
                })
                .collect();
            let _ = msg_tx.send(AgentMessage::SessionsList(sessions));
        }
        ClientCommand::ListProviders => {
            let resp = cx
                .send_request(ListProvidersRequest::default())
                .block_task()
                .await?;
            let providers = resp
                .entries
                .into_iter()
                .map(|e| ProviderInfo {
                    id: e.provider_id,
                    name: e.provider_name,
                    configured: e.configured,
                    description: e.description,
                    models: e.models.into_iter().map(|m| m.id).collect(),
                })
                .collect();
            let _ = msg_tx.send(AgentMessage::ProvidersList(providers));
        }
        ClientCommand::ListExtensions => {
            let resp = cx
                .send_request(GetExtensionsRequest {})
                .block_task()
                .await?;
            let extensions = resp
                .extensions
                .into_iter()
                .filter_map(|v| {
                    let obj = v.as_object()?;
                    Some(ExtensionInfo {
                        name: obj.get("name")?.as_str()?.to_string(),
                        enabled: obj.get("enabled")?.as_bool()?,
                        ext_type: obj
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                    })
                })
                .collect();
            let _ = msg_tx.send(AgentMessage::ExtensionsList(extensions));
        }
        ClientCommand::SaveDefaults { provider, model } => {
            let _ = cx
                .send_request(DefaultsSaveRequest {
                    provider_id: provider,
                    model_id: Some(model),
                })
                .block_task()
                .await;
        }
        ClientCommand::ToggleExtension { key, enabled } => {
            let _ = cx
                .send_request(ToggleConfigExtensionRequest {
                    config_key: key,
                    enabled,
                })
                .block_task()
                .await;
        }
        _ => {}
    }
    Ok(())
}
