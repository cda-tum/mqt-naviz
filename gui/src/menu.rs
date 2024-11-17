//! [MenuBar] to show a menu on the top.

use std::{
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
};

use egui::{Align2, Button, Grid, Window};
use git_version::git_version;

use crate::{export_dialog::ExportSettings, future_helper::FutureHelper, util::WEB};

type SendReceivePair<T> = (Sender<T>, Receiver<T>);

/// The menu bar struct which contains the state of the menu
pub struct MenuBar {
    file_open_channel: SendReceivePair<MenuEvent>,
    /// Whether to draw the about-window
    about_open: bool,
    /// The export-settings-dialog to show when the user wants to export a video
    export_settings: ExportSettings,
}

/// An event which is triggered on menu navigation.
/// Higher-Level than just button-clicks.
pub enum MenuEvent {
    /// A file of the specified [FileType] with the specified content was opened
    FileOpen(FileType, Vec<u8>),
    /// A video should be exported to the specified path with the specified resolution and fps
    ExportVideo {
        target: PathBuf,
        resolution: (u32, u32),
        fps: u32,
    },
}

/// The available FileTypes for opening
pub enum FileType {
    Instructions,
    Machine,
    Style,
}

/// Config options for what to show inside the menu
pub struct MenuConfig {
    /// Show export option
    pub export: bool,
}

impl FileType {
    pub fn name(&self) -> &'static str {
        match self {
            FileType::Instructions => "NAViz instructions",
            FileType::Machine => "NAViz machine",
            FileType::Style => "NAViz style",
        }
    }
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            FileType::Instructions => &["naviz"],
            FileType::Machine => &["namachine"],
            FileType::Style => &["nastyle"],
        }
    }
}

impl MenuBar {
    /// Create a new [MenuBar]
    pub fn new() -> Self {
        Self {
            file_open_channel: channel(),
            about_open: false,
            export_settings: Default::default(),
        }
    }

    /// Get the file open channel.
    ///
    /// Whenever a new file is opened,
    /// its content will be sent over this channel.
    pub fn events(&self) -> &Receiver<MenuEvent> {
        &self.file_open_channel.1
    }

    /// Draw the [MenuBar]
    pub fn draw(
        &mut self,
        config: MenuConfig,
        future_helper: &FutureHelper,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
    ) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open Instructions").clicked() {
                    self.choose_file(FileType::Instructions, future_helper);
                }
                if ui.button("Open Machine").clicked() {
                    self.choose_file(FileType::Machine, future_helper);
                }
                if ui.button("Open Style").clicked() {
                    self.choose_file(FileType::Style, future_helper);
                }

                if !WEB {
                    // Export only on native, as it requires a system-installed `ffmpeg` (for now)
                    ui.separator();
                    if ui
                        .add_enabled(config.export, Button::new("Export Video"))
                        .clicked()
                    {
                        self.export_settings.show();
                    }
                }

                if !WEB {
                    // Quit-button only on native
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    self.about_open = true;
                }
            });
        });

        if self.export_settings.draw(ctx) {
            self.export(future_helper);
        }

        self.draw_about_window(ctx);
    }

    /// Show the file-choosing dialog and read the file if a new file was selected
    fn choose_file(&self, file_type: FileType, future_helper: &FutureHelper) {
        future_helper.execute_maybe_to(
            async move {
                if let Some(path) = rfd::AsyncFileDialog::new()
                    .add_filter(file_type.name(), file_type.extensions())
                    .pick_file()
                    .await
                {
                    Some(MenuEvent::FileOpen(file_type, path.read().await))
                } else {
                    None
                }
            },
            self.file_open_channel.0.clone(),
        );
    }

    /// Show the file-saving dialog and get the path to export to if a file was selected
    fn export(&self, future_helper: &FutureHelper) {
        let resolution = self.export_settings.resolution();
        let fps = self.export_settings.fps();
        future_helper.execute_maybe_to(
            async move {
                rfd::AsyncFileDialog::new()
                    .save_file()
                    .await
                    .map(|handle| handle.path().to_path_buf())
                    .map(|target| MenuEvent::ExportVideo {
                        target,
                        resolution,
                        fps,
                    })
            },
            self.file_open_channel.0.clone(),
        );
    }

    /// Draws the about-window if [Self::about_open] is `true`
    fn draw_about_window(&mut self, ctx: &egui::Context) {
        Window::new("About NAViz")
            .anchor(Align2::CENTER_CENTER, (0., 0.))
            .resizable(false)
            .open(&mut self.about_open)
            .collapsible(false)
            .show(ctx, |ui| {
                Grid::new("about_window").num_columns(2).show(ui, |ui| {
                    ui.label("Version");
                    ui.label(env!("CARGO_PKG_VERSION"));
                    ui.end_row();

                    ui.label("Build");
                    ui.label(git_version!(
                        args = ["--always", "--dirty=+dev", "--match=naviz-gui@*"],
                        fallback = "unknown"
                    ));
                    ui.end_row();

                    ui.label("License");
                    ui.label(env!("CARGO_PKG_LICENSE"));
                    ui.end_row();

                    ui.label("Source Code");
                    ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));
                    ui.end_row();
                });
            });
    }
}
