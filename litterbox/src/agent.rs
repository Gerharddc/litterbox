use eframe::egui;
use futures::Future;
use russh::keys::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::process::Command;

use crate::errors::LitterboxError;
use crate::extract_stdout;
use crate::files::SshSockFile;

#[derive(Clone)]
struct AskAgent {
    litterbox_path: String,
    agent_locked: Arc<AtomicBool>,
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
        use agent::server::MessageType;

        if !self.agent_locked.load(Ordering::SeqCst) {
            return true;
        }

        let confirmation_msg = match msg {
            MessageType::RequestKeys => "RequestKeys",
            MessageType::AddKeys => "AddKeys",
            MessageType::RemoveKeys => "RemoveKeys",
            MessageType::RemoveAllKeys => "RemoveAllKeys",
            MessageType::Sign => "Sign",
            MessageType::Lock => "Lock",
            MessageType::Unlock => "Unlock",
        };

        let output = Command::new(self.litterbox_path.clone())
            .args(["confirm", confirmation_msg])
            .output()
            .await
            .expect("Litterbox should return valid output to itself.");

        let stdout =
            extract_stdout(&output).expect("Litterbox should return valid output to itself.");

        // We ignore the last character which will be a newline
        match &stdout[..(stdout.len() - 1)] {
            USER_ACCEPTED => true,
            USER_DECLINED => false,
            _other => {
                log::error!("Unexpected confirmation response: {}", _other);
                false
            }
        }
    }
}

struct ConfirmationDialog<'a> {
    user_response: &'a mut bool,
    user_req_msg: &'a str,
}

impl<'a> ConfirmationDialog<'a> {
    fn new(user_response: &'a mut bool, user_req_msg: &'a str) -> Self {
        Self {
            user_response,
            user_req_msg,
        }
    }
}

impl eframe::App for ConfirmationDialog<'_> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Confirm SSH Request");

            ui.add(egui::Image::new(egui::include_image!("../assets/cat.svg")).max_width(400.0));

            ui.label(self.user_req_msg);

            ui.horizontal(|ui| {
                if ui.button("Yes").clicked() {
                    *self.user_response = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                if ui.button("No").clicked() {
                    *self.user_response = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

const USER_ACCEPTED: &str = "accepted";
const USER_DECLINED: &str = "declined";

pub async fn start_ssh_agent(
    lbx_name: &str,
    agent_locked: Arc<AtomicBool>,
) -> Result<PathBuf, LitterboxError> {
    let mut args = std::env::args();
    let litterbox_path = args.next().expect("Binary path should be defined.");

    let ssh_sock = SshSockFile::new(lbx_name, false)?;
    let agent_path = ssh_sock.path().to_owned();

    tokio::spawn(async move {
        log::debug!("Starting SSH agent server task");

        let listener =
            tokio::net::UnixListener::bind(ssh_sock.path()).expect("SSH socket should be bindable");

        russh::keys::agent::server::serve(
            tokio_stream::wrappers::UnixListenerStream::new(listener),
            AskAgent {
                litterbox_path,
                agent_locked,
            },
        )
        .await
    });

    Ok(agent_path)
}

pub fn prompt_confirmation(confirmation_msg: &str) {
    let mut user_accepted = false;

    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.inner_size = Some((250.0, 300.0).into());

    let run_result = eframe::run_native(
        "Litterbox",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(ConfirmationDialog::new(
                &mut user_accepted,
                confirmation_msg,
            )))
        }),
    );

    if let Err(e) = run_result {
        println!("Error running ConfirmationDialog: {:#?}", e);
    }

    let reponse = if user_accepted {
        USER_ACCEPTED
    } else {
        USER_DECLINED
    };
    println!("{}", reponse);
}
