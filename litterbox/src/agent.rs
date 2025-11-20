use eframe::egui;
use futures::Future;
use russh::keys::*;
use std::path::PathBuf;
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use crate::errors::LitterboxError;
use crate::extract_stdout;
use crate::files::lbx_ssh_path;

type ConfReqSender = mpsc::Sender<(&'static str, oneshot::Sender<bool>)>;

#[derive(Clone)]
struct AskAgent {
    conf_req_tx: ConfReqSender,
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

        let user_req_msg = match msg {
            MessageType::RequestKeys => "RequestKeys",
            MessageType::AddKeys => "AddKeys",
            MessageType::RemoveKeys => "RemoveKeys",
            MessageType::RemoveAllKeys => "RemoveAllKeys",
            MessageType::Sign => "Sign",
            MessageType::Lock => "Lock",
            MessageType::Unlock => "Unlock",
        };

        let (user_resp_tx, user_resp_rx) = oneshot::channel();

        match self.conf_req_tx.send((user_req_msg, user_resp_tx)).await {
            Ok(_) => match user_resp_rx.await {
                Ok(user_resp) => user_resp,
                Err(e) => {
                    println!("Error receiving user response: {:#?}", e);
                    false
                }
            },
            Err(e) => {
                println!("Error sending user confirmation request: {:#?}", e);
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

pub async fn start_agent(lbx_name: &str) -> Result<PathBuf, LitterboxError> {
    let mut args = std::env::args();
    let litterbox_path = args.next().expect("Binary path should be defined.");

    let (conf_req_tx, mut conf_req_rx) = mpsc::channel(100);

    let agent_path = lbx_ssh_path(lbx_name)?;
    let agent_path_ = agent_path.clone();

    tokio::spawn(async move {
        log::debug!("Starting SSH agent server task");

        let listener = tokio::net::UnixListener::bind(&agent_path_).unwrap();
        russh::keys::agent::server::serve(
            tokio_stream::wrappers::UnixListenerStream::new(listener),
            AskAgent { conf_req_tx },
        )
        .await
    });

    // FIXME: just combine the two tasks!
    tokio::task::spawn(async move {
        log::debug!("Starting task to listen for confirmation requests");

        while let Some((confirmation_msg, user_resp_tx)) = conf_req_rx.recv().await {
            let output = Command::new(litterbox_path.clone())
                .args(["confirm", confirmation_msg])
                .output()
                .await
                .expect("Litterbox should return valid output to itself.");

            let stdout =
                extract_stdout(&output).expect("Litterbox should return valid output to itself.");

            // We ignore the last character which will be a newline
            let user_accepted = match &stdout[..(stdout.len() - 1)] {
                USER_ACCEPTED => true,
                USER_DECLINED => false,
                _other => {
                    log::error!("Unexpected confirmation response: {}", _other);
                    false
                }
            };

            if let Err(e) = user_resp_tx.send(user_accepted) {
                log::error!("Error sending user response to client: {:#?}.", e);
            }
        }
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
