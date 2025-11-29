use eframe::egui;
use futures::Future;
use russh::keys::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use strum_macros::{Display, EnumString};
use tokio::process::Command;

use crate::errors::LitterboxError;
use crate::extract_stdout;
use crate::files::SshSockFile;

#[derive(Clone)]
struct AskAgent {
    lbx_name: String,
    litterbox_path: String,
    agent_state: Arc<AgentState>,
}

#[derive(Debug, EnumString, Display)]
enum UserResponse {
    Approved,
    Declined,
    ApprovedForSession,
}

#[derive(PartialEq, Eq, Hash, Display, Clone, Copy, EnumString)]
pub enum UserRequest {
    RequestKeys,
    AddKeys,
    RemoveKeys,
    RemoveAllKeys,
    Sign,
    Lock,
    Unlock,
}

impl From<agent::server::MessageType> for UserRequest {
    fn from(value: agent::server::MessageType) -> Self {
        use agent::server::MessageType;

        match value {
            MessageType::RequestKeys => UserRequest::RequestKeys,
            MessageType::AddKeys => UserRequest::AddKeys,
            MessageType::RemoveKeys => UserRequest::RemoveKeys,
            MessageType::RemoveAllKeys => UserRequest::RemoveAllKeys,
            MessageType::Sign => UserRequest::Sign,
            MessageType::Lock => UserRequest::Lock,
            MessageType::Unlock => UserRequest::Unlock,
        }
    }
}

impl agent::server::Agent for AskAgent {
    fn confirm(
        self,
        _: std::sync::Arc<PrivateKey>,
    ) -> Box<dyn Future<Output = (Self, bool)> + Send + Unpin> {
        todo!("Confirm private key")
    }

    async fn confirm_request(&self, msg: agent::server::MessageType) -> bool {
        let request: UserRequest = msg.into();

        if !self.agent_state.locked.load(Ordering::SeqCst) {
            log::debug!(
                "Agent not locked, request automatically approved: {}",
                request
            );
            return true;
        }

        if request == UserRequest::RequestKeys
            && self.agent_state.approved_for_session.load(Ordering::SeqCst)
        {
            log::info!("RequestKeys approved for session, not prompting.");
            return true;
        }

        let output = Command::new(self.litterbox_path.clone())
            .args([
                "confirm",
                "--request",
                &request.to_string(),
                "--lbx-name",
                &self.lbx_name,
            ])
            .output()
            .await
            .expect("Litterbox should return valid output to itself.");

        let stdout =
            extract_stdout(&output).expect("Litterbox should return valid output to itself.");

        // We ignore the last character which will be a newline
        let resp_str = &stdout[..(stdout.len() - 1)];

        if let Ok(resp) = resp_str.parse() {
            match resp {
                UserResponse::Approved => true,
                UserResponse::Declined => false,
                UserResponse::ApprovedForSession => {
                    self.agent_state
                        .approved_for_session
                        .store(true, Ordering::SeqCst);
                    true
                }
            }
        } else {
            log::error!("Unexpected confirmation response: {}", resp_str);
            false
        }
    }
}

struct ConfirmationDialog<'a> {
    user_response: &'a mut UserResponse,
    user_request: &'a UserRequest,
    lbx_name: &'a str,
}

impl eframe::App for ConfirmationDialog<'_> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("New SSH Request");
            ui.horizontal(|ui| {
                ui.label("From Litterbox:");
                ui.label(egui::RichText::new(self.lbx_name).strong());
            });

            ui.add(egui::Image::new(egui::include_image!("../assets/cat.svg")).max_width(400.0));
            ui.horizontal(|ui| {
                ui.label("Request:");
                ui.label(egui::RichText::new(self.user_request.to_string()).strong());
            });

            ui.horizontal(|ui| {
                if ui.button("Approve").clicked() {
                    *self.user_response = UserResponse::Approved;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                if ui.button("Decline").clicked() {
                    *self.user_response = UserResponse::Declined;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                let may_approve_for_session = *self.user_request == UserRequest::RequestKeys;
                if may_approve_for_session && ui.button("Approve for Session").clicked() {
                    *self.user_response = UserResponse::ApprovedForSession;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

pub struct AgentState {
    /// When the agent is locked, uesrs will need to approve requests
    pub locked: AtomicBool,

    /// When set, users no longer need to approve requests to list keys
    pub approved_for_session: AtomicBool,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            locked: AtomicBool::new(false),
            approved_for_session: AtomicBool::new(false),
        }
    }
}

pub async fn start_ssh_agent(
    lbx_name: &str,
    agent_state: Arc<AgentState>,
) -> Result<PathBuf, LitterboxError> {
    let mut args = std::env::args();
    let litterbox_path = args.next().expect("Binary path should be defined.");

    let ssh_sock = SshSockFile::new(lbx_name, false)?;
    let agent_path = ssh_sock.path().to_owned();

    let ssh_sock_path = ssh_sock.path();
    log::debug!("Binding SSH socket: {:#?}", ssh_sock_path);
    let listener =
        tokio::net::UnixListener::bind(ssh_sock_path).expect("SSH socket should be bindable");

    let lbx_name = lbx_name.to_string();
    tokio::spawn(async move {
        log::debug!("Starting SSH agent server task");

        // We need to keep the socket object alive to prevent the file from getting deleted
        let _ssh_sock = ssh_sock;

        russh::keys::agent::server::serve(
            tokio_stream::wrappers::UnixListenerStream::new(listener),
            AskAgent {
                lbx_name,
                litterbox_path,
                agent_state,
            },
        )
        .await
    });

    Ok(agent_path)
}

pub fn prompt_confirmation(request: &str, lbx_name: &str) {
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.inner_size = Some((270.0, 340.0).into());

    let user_request = request
        .parse()
        .expect("User request input should be valid.");
    let mut user_response = UserResponse::Declined;

    let run_result = eframe::run_native(
        "Litterbox",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(ConfirmationDialog {
                user_response: &mut user_response,
                user_request: &user_request,
                lbx_name,
            }))
        }),
    );

    if let Err(e) = run_result {
        println!("Error running ConfirmationDialog: {:#?}", e);
    }

    println!("{user_response}");
}
