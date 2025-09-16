use async_tempfile::TempDir;
use eframe::egui;
use futures::Future;
use russh::keys::*;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

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

async fn start_ssh_agent(da_sender: ConfReqSender) -> JoinHandle<Result<(), Error>> {
    env_logger::try_init().unwrap_or(());

    let dir = TempDir::new().await.unwrap();
    let agent_path = dir.dir_path().join("agent");
    println!("agent_path: {:#?}", agent_path);

    let agent_path_ = agent_path.clone();

    let server_handle = tokio::spawn(async move {
        let _keep_dir_alive = dir; // We need a handle here other the dir gets dropped

        let listener = tokio::net::UnixListener::bind(&agent_path_).unwrap();
        russh::keys::agent::server::serve(
            tokio_stream::wrappers::UnixListenerStream::new(listener),
            AskAgent {
                conf_req_tx: da_sender,
            },
        )
        .await
    });
    server_handle
}

struct ConfirmationDialog<'a> {
    user_response: &'a mut bool,
    user_req_msg: &'static str,
}

impl<'a> ConfirmationDialog<'a> {
    fn new<'b>(user_response: &'a mut bool, user_req_msg: &'static str) -> Self {
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

            ui.label(self.user_req_msg);

            if ui.button("Yes").clicked() {
                *self.user_response = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            if ui.button("No").clicked() {
                *self.user_response = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}

const USER_ACCEPTED: i32 = 123;
const USER_DECLINED: i32 = 100;

#[tokio::main]
async fn main() {
    use nix::{
        sys::wait::{WaitStatus, waitpid},
        unistd::{ForkResult, fork},
    };

    let (conf_req_tx, mut conf_req_rx) = mpsc::channel(100);
    start_ssh_agent(conf_req_tx).await;

    while let Some((confirmation_msg, user_resp_tx)) = conf_req_rx.recv().await {
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child, .. }) => {
                println!("Awaiting confirmation from dialog process.");

                let user_accepted = match waitpid(child, None) {
                    Ok(WaitStatus::Exited(_pid, exit_code)) => exit_code == USER_ACCEPTED,
                    _other => {
                        println!("Unexpected result from dialog process: {:#?}.", _other);
                        false
                    }
                };

                if let Err(e) = user_resp_tx.send(user_accepted) {
                    println!("Error sending user response to client: {:#?}.", e);
                }
            }
            Ok(ForkResult::Child) => {
                let mut user_accepted = false;
                let native_options = eframe::NativeOptions::default();

                let run_result = eframe::run_native(
                    "Ask-agent Confirmation",
                    native_options,
                    Box::new(|_cc| {
                        Ok(Box::new(ConfirmationDialog::new(
                            &mut user_accepted,
                            confirmation_msg,
                        )))
                    }),
                );

                if let Err(e) = run_result {
                    println!("Error running ConfirmationDialog: {:#?}", e);
                }

                let code = if user_accepted {
                    USER_ACCEPTED
                } else {
                    USER_DECLINED
                };
                println!("code: {}", code);
                std::process::exit(code);
            }
            Err(_) => {
                println!("Forking to open ConfirmationDialog failed");

                if let Err(e) = user_resp_tx.send(false) {
                    println!("Error sending user response to client: {:#?}.", e);
                }
            }
        }
    }
}
