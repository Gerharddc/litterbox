use eframe::egui;
use futures::Future;
use russh::keys::*;
use std::collections::HashSet;
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
    agent_locked: Arc<AtomicBool>,
    approved_for_session: HashSet<UserRequest>,
}

#[derive(Debug, EnumString, Display)]
enum UserResponse {
    Approved,
    Declined,
    ApprovedForSession,
}

#[derive(PartialEq, Eq, Hash, Display, Clone, Copy)]
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
        println!("TODO: Confirm private key!");
        Box::new(futures::future::ready((self, true)))
    }

    async fn confirm_request(&self, msg: agent::server::MessageType) -> bool {
        if !self.agent_locked.load(Ordering::SeqCst) {
            return true;
        }

        let request: UserRequest = msg.into();
        if self.approved_for_session.contains(&request) {
            log::info!("Request approved for session, approving: {request}");
            return true;
        }

        let output = Command::new(self.litterbox_path.clone())
            .args([
                "confirm",
                "--message",
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

                // FIXME: we need to store this approval
                UserResponse::ApprovedForSession => true,
            }
        } else {
            log::error!("Unexpected confirmation response: {}", resp_str);
            false
        }
    }
}

struct ConfirmationDialog<'a> {
    user_response: &'a mut UserResponse,
    confirmation_msg: &'a str,
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
                ui.label(egui::RichText::new(self.confirmation_msg).strong());
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

                if ui.button("Approve for Session").clicked() {
                    *self.user_response = UserResponse::ApprovedForSession;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

pub async fn start_ssh_agent(
    lbx_name: &str,
    agent_locked: Arc<AtomicBool>,
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
                agent_locked,
                approved_for_session: HashSet::new(),
            },
        )
        .await
    });

    Ok(agent_path)
}

pub fn prompt_confirmation(confirmation_msg: &str, lbx_name: &str) {
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.inner_size = Some((270.0, 340.0).into());

    let mut user_response = UserResponse::Declined;
    let run_result = eframe::run_native(
        "Litterbox",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(ConfirmationDialog {
                user_response: &mut user_response,
                confirmation_msg,
                lbx_name,
            }))
        }),
    );

    if let Err(e) = run_result {
        println!("Error running ConfirmationDialog: {:#?}", e);
    }

    println!("{user_response}");
}
