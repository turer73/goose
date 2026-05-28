use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::time::Duration;
use tokio::sync::mpsc;

use super::acp_client::{spawn_acp_client, AgentMessage, ClientCommand};
use super::event::{Event, EventHandler};
use super::views::chat::{ChatMessage, ChatState, MessageRole, ToolCallDisplay};
use super::views::extensions::ExtensionsState;
use super::views::onboarding::OnboardingState;
use super::views::sessions::SessionsState;

#[derive(Clone, PartialEq)]
enum View {
    Splash,
    Onboarding,
    Chat,
    Sessions,
    Extensions,
}

struct App {
    view: View,
    tick: u64,
    chat: ChatState,
    onboarding: OnboardingState,
    sessions: SessionsState,
    extensions: ExtensionsState,
    cmd_tx: mpsc::UnboundedSender<ClientCommand>,
    msg_rx: mpsc::UnboundedReceiver<AgentMessage>,
    should_quit: bool,
    session_active: bool,
}

impl App {
    fn new(
        cmd_tx: mpsc::UnboundedSender<ClientCommand>,
        msg_rx: mpsc::UnboundedReceiver<AgentMessage>,
    ) -> Self {
        Self {
            view: View::Splash,
            tick: 0,
            chat: ChatState::new(),
            onboarding: OnboardingState::new(),
            sessions: SessionsState::new(),
            extensions: ExtensionsState::new(),
            cmd_tx,
            msg_rx,
            should_quit: false,
            session_active: false,
        }
    }

    fn handle_agent_message(&mut self, msg: AgentMessage) {
        match msg {
            AgentMessage::Initialized => {
                let _ = self.cmd_tx.send(ClientCommand::ListProviders);
            }
            AgentMessage::ProvidersList(providers) => {
                let has_configured = providers.iter().any(|p| p.configured);
                self.onboarding.providers = providers;
                if has_configured {
                    self.start_session();
                } else {
                    self.view = View::Onboarding;
                }
            }
            AgentMessage::SessionCreated(_id) => {
                self.session_active = true;
                self.view = View::Chat;
                self.chat.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "Session started. How can I help?".to_string(),
                });
            }
            AgentMessage::TextChunk(text) => {
                self.chat.is_streaming = true;
                self.chat.streaming_text.push_str(&text);
            }
            AgentMessage::ToolCallStarted { title, id } => {
                self.chat.is_streaming = true;
                self.chat.tool_calls.push(ToolCallDisplay {
                    title,
                    id,
                    status: "running".to_string(),
                });
            }
            AgentMessage::ToolCallUpdate { id, status } => {
                if let Some(tc) = self.chat.tool_calls.iter_mut().find(|t| t.id == id) {
                    tc.status = status;
                }
            }
            AgentMessage::ResponseComplete => {
                self.chat.finalize_stream();
            }
            AgentMessage::SessionsList(sessions) => {
                self.sessions.sessions = sessions;
                self.view = View::Sessions;
            }
            AgentMessage::ExtensionsList(extensions) => {
                self.extensions.extensions = extensions;
                self.view = View::Extensions;
            }
            AgentMessage::Error(err) => {
                self.chat.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Error: {err}"),
                });
            }
        }
    }

    fn start_session(&mut self) {
        let _ = self.cmd_tx.send(ClientCommand::CreateSession);
    }

    fn handle_slash_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
        match parts[0] {
            "/help" => {
                self.chat.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: [
                        "Available commands:",
                        "  /sessions  - list and resume sessions",
                        "  /extensions - manage extensions",
                        "  /provider  - change provider/model",
                        "  /clear     - clear chat history",
                        "  /new       - start a new session",
                        "  /quit      - exit goose",
                    ]
                    .join("\n"),
                });
            }
            "/sessions" => {
                let _ = self.cmd_tx.send(ClientCommand::ListSessions);
            }
            "/extensions" => {
                let _ = self.cmd_tx.send(ClientCommand::ListExtensions);
            }
            "/provider" => {
                let _ = self.cmd_tx.send(ClientCommand::ListProviders);
                self.view = View::Onboarding;
            }
            "/clear" => {
                self.chat.messages.clear();
                self.chat.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: "Chat cleared.".to_string(),
                });
            }
            "/new" => {
                self.chat = ChatState::new();
                self.start_session();
            }
            "/quit" => {
                self.should_quit = true;
            }
            _ => {
                self.chat.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Unknown command: {}. Type /help", parts[0]),
                });
            }
        }
    }

    fn handle_key_chat(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match (code, modifiers) {
            (KeyCode::Enter, KeyModifiers::NONE) => {
                let input = self.chat.take_input();
                if input.is_empty() {
                    return;
                }
                if input.starts_with('/') {
                    self.handle_slash_command(&input);
                } else {
                    self.chat.push_user_message(input.clone());
                    self.chat.is_streaming = true;
                    let _ = self.cmd_tx.send(ClientCommand::SendPrompt(input));
                }
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.handle_slash_command("/clear");
            }
            (KeyCode::Backspace, _) => self.chat.input_backspace(),
            (KeyCode::Delete, _) => self.chat.input_delete(),
            (KeyCode::Left, _) => self.chat.input_left(),
            (KeyCode::Right, _) => self.chat.input_right(),
            (KeyCode::Home, _) => self.chat.input_home(),
            (KeyCode::End, _) => self.chat.input_end(),
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.chat.input_insert(c);
            }
            _ => {}
        }
    }

    fn handle_key_onboarding(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Up => self.onboarding.move_up(),
            KeyCode::Down => self.onboarding.move_down(),
            KeyCode::Enter => {
                let filtered = self.onboarding.filtered_providers();
                if let Some(provider) = filtered.get(self.onboarding.selected_idx) {
                    let provider_id = provider.id.clone();
                    let model = provider.models.first().cloned().unwrap_or_default();
                    let _ = self.cmd_tx.send(ClientCommand::SaveDefaults {
                        provider: provider_id,
                        model,
                    });
                    self.start_session();
                }
            }
            KeyCode::Esc => {
                self.start_session();
            }
            KeyCode::Backspace => {
                self.onboarding.search_query.pop();
                self.onboarding.selected_idx = 0;
            }
            KeyCode::Char(c) => {
                self.onboarding.search_query.push(c);
                self.onboarding.selected_idx = 0;
            }
            _ => {}
        }
    }

    fn handle_key_sessions(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Up => self.sessions.move_up(),
            KeyCode::Down => self.sessions.move_down(),
            KeyCode::Enter => {
                if let Some(_session) = self.sessions.sessions.get(self.sessions.selected_idx) {
                    // Resume by creating a new session (TODO: load existing)
                    self.chat = ChatState::new();
                    self.start_session();
                }
            }
            KeyCode::Char('n') => {
                self.chat = ChatState::new();
                self.start_session();
            }
            KeyCode::Esc => {
                self.view = View::Chat;
            }
            _ => {}
        }
    }

    fn handle_key_extensions(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Up => self.extensions.move_up(),
            KeyCode::Down => self.extensions.move_down(),
            KeyCode::Char(' ') => {
                if let Some(ext) = self.extensions.extensions.get(self.extensions.selected_idx) {
                    let key = ext.name.clone();
                    let new_enabled = !ext.enabled;
                    let _ = self.cmd_tx.send(ClientCommand::ToggleExtension {
                        key,
                        enabled: new_enabled,
                    });
                    // Optimistic update
                    if let Some(e) = self
                        .extensions
                        .extensions
                        .get_mut(self.extensions.selected_idx)
                    {
                        e.enabled = new_enabled;
                    }
                }
            }
            KeyCode::Esc => {
                self.view = View::Chat;
            }
            _ => {}
        }
    }
}

pub async fn run_tui() -> Result<()> {
    // Find the goose binary
    let goose_bin = std::env::current_exe()?;

    // Spawn ACP client
    let (cmd_tx, msg_rx) = spawn_acp_client(goose_bin);

    // Initialize terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(cmd_tx.clone(), msg_rx);
    let mut events = EventHandler::new(Duration::from_millis(50));

    // Kick off initialization
    let _ = cmd_tx.send(ClientCommand::Initialize);

    // Main loop
    loop {
        // Draw
        terminal.draw(|f| {
            let area = f.area();
            match app.view {
                View::Splash => {
                    super::views::splash::render_splash(f, area, app.tick);
                }
                View::Onboarding => {
                    super::views::onboarding::render_onboarding(f, area, &app.onboarding);
                }
                View::Chat => {
                    super::views::chat::render_chat(f, area, &app.chat, app.tick);
                }
                View::Sessions => {
                    super::views::sessions::render_sessions(f, area, &app.sessions);
                }
                View::Extensions => {
                    super::views::extensions::render_extensions(f, area, &app.extensions);
                }
            }
        })?;

        if app.should_quit {
            break;
        }

        // Handle events
        tokio::select! {
            event = events.next() => {
                match event? {
                    Event::Key(key) => {
                        // Global quit
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            app.should_quit = true;
                            continue;
                        }
                        match app.view {
                            View::Splash => {} // no input during splash
                            View::Onboarding => {
                                app.handle_key_onboarding(key.code, key.modifiers);
                            }
                            View::Chat => {
                                app.handle_key_chat(key.code, key.modifiers);
                            }
                            View::Sessions => {
                                app.handle_key_sessions(key.code, key.modifiers);
                            }
                            View::Extensions => {
                                app.handle_key_extensions(key.code, key.modifiers);
                            }
                        }
                    }
                    Event::Tick => {
                        app.tick += 1;
                    }
                    _ => {}
                }
            }
            msg = app.msg_rx.recv() => {
                if let Some(msg) = msg {
                    app.handle_agent_message(msg);
                }
            }
        }
    }

    // Cleanup
    let _ = cmd_tx.send(ClientCommand::Shutdown);
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
