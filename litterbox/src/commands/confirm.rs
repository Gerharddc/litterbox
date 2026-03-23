use anyhow::Result;
use clap::Args;
use eframe::egui;

use crate::agent::{UserRequest, UserResponse};

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

            ui.add(egui::Image::new(egui::include_image!("../../assets/cat.svg")).max_width(400.0));
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

/// Ask the user to confirm a request (for internal use)
#[derive(Args, Debug)]
pub struct Command {
    /// The request that the user needs to confirm
    #[arg(long)]
    request: String,

    /// The name of the litterbox sending the request
    #[arg(long)]
    lbx_name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        let mut native_options = eframe::NativeOptions::default();
        native_options.viewport.inner_size = Some((270.0, 340.0).into());

        let user_request = self
            .request
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
                    lbx_name: &self.lbx_name,
                }))
            }),
        );

        if let Err(e) = run_result {
            eprintln!("Error running ConfirmationDialog: {:#?}", e);
        }

        // Response is read by the agent
        print!("{user_response}");

        Ok(())
    }
}
