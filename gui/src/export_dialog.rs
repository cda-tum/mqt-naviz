use std::{
    process::ExitStatus,
    sync::{
        atomic::AtomicU32,
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
};

use egui::{Align2, Context, DragValue, Grid, Id, Layout, ProgressBar, Spinner, Window};
use naviz_video::VideoProgress;

/// Settings-Dialog for the export
pub struct ExportSettings {
    /// Resolution to render at
    resolution: (u32, u32),
    /// FPS to render at
    fps: u32,
    /// Whether the export settings dialog is shown
    show: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            resolution: (1920, 1080),
            fps: 30,
            show: false,
        }
    }
}

impl ExportSettings {
    /// Shows these [ExportSettings] and resets to defaults
    pub fn show(&mut self) {
        *self = Default::default();
        self.show = true;
    }

    /// Draws these [ExportSettings] (if they are [shown][ExportSettings::show]).
    /// Returns `true` when the user accepted the settings.
    pub fn draw(&mut self, ctx: &Context) -> bool {
        let ok_clicked = Window::new("Export Video")
            .anchor(Align2::CENTER_CENTER, (0., 0.))
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .open(&mut self.show)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    let w = Grid::new("export_dialog")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Resolution:");
                            ui.horizontal(|ui| {
                                ui.add(DragValue::new(&mut self.resolution.0));
                                ui.label("x");
                                ui.add(DragValue::new(&mut self.resolution.1));
                            });
                            ui.end_row();

                            ui.label("FPS:");
                            ui.add(DragValue::new(&mut self.fps));
                            ui.end_row();
                        })
                        .response
                        .rect
                        .width();

                    ui.set_max_width(w);
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.button("Ok").clicked()
                    })
                    .inner
                })
                .inner
            })
            .and_then(|r| r.inner)
            .unwrap_or(false);
        if ok_clicked && self.show {
            self.show = false;
        }
        ok_clicked
    }

    /// Gets the currently selected resolution.
    /// Note: Changes with user-input when shown.
    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }

    /// Gets the currently selected fps.
    /// Note: Changes with user-input when shown.
    pub fn fps(&self) -> u32 {
        self.fps
    }
}

/// Container for multiple [ExportProgress]es
#[derive(Default)]
pub struct ExportProgresses {
    /// The progresses
    progresses: Vec<Arc<Mutex<ExportProgress>>>,
}

impl ExportProgresses {
    /// Adds a new progress-window and returns a channel to send [VideoProgress]-events over
    pub fn add(&mut self) -> Sender<VideoProgress> {
        let (progress_tx, progress_rx) = channel();
        let progress = Arc::new(Mutex::new(ExportProgress::new(progress_rx)));
        self.progresses.push(progress.clone());
        progress_tx
    }

    /// Draws all the [ExportProgress]es
    pub fn draw(&mut self, ctx: &Context) {
        self.progresses.retain_mut(|p| {
            p.lock()
                .map(|mut p| p.draw(ctx))
                .is_ok_and(|closed| !closed)
        });
    }
}

/// A counter for generating unique IDs for [ExportProgress]es
static EXPORT_PROGRESS_ID_GEN: AtomicU32 = AtomicU32::new(0);

enum ExportProgressState {
    /// The video exporter is being created
    Creating,
    /// The video is being exported
    Working {
        /// The render-progress (`current_time`, `duration`)
        render: (f32, f32),
        /// The encoding-progress (`current_time`, `duration`)
        encode: (f32, f32),
    },
    /// The video exporter is finished with the specified [ExitStatus]
    Done(ExitStatus),
    /// Channel dropped during export (while not [Done][ExportProgressState::Done]);
    /// Unknown what the current status is.
    /// Should not happen in normal operation.
    Unknown,
}

impl ExportProgressState {
    /// Received an `encode`-progress.
    /// Updates the state to contain the progress.
    /// If state was not [ExportProgressState::Working],
    /// will change change state and set max `render`-time to `max`.
    fn set_encode(&mut self, current: f32, max: f32) {
        if let Self::Working { encode, render: _ } = self {
            *encode = (current, max);
        } else {
            *self = Self::Working {
                encode: (current, max),
                render: (0., max),
            };
        }
    }

    /// Received an `render`-progress.
    /// Updates the state to contain the progress.
    /// If state was not [ExportProgressState::Working],
    /// will change change state and set max `encode`-time to `max`.
    fn set_render(&mut self, current: f32, max: f32) {
        if let Self::Working { encode: _, render } = self {
            *render = (current, max);
        } else {
            *self = Self::Working {
                encode: (0., max),
                render: (current, max),
            };
        }
    }
}

/// A dialog that displays the export progress
pub struct ExportProgress {
    /// The current state
    state: ExportProgressState,
    /// The receiver for [VideoProgress]-events
    receiver: Receiver<VideoProgress>,
    /// The unique id for the window (generated by [EXPORT_PROGRESS_ID_GEN])
    window_id: String,
    /// The unique id for the grid in the window (generated by [EXPORT_PROGRESS_ID_GEN])
    grid_id: String,
}

impl ExportProgress {
    /// Create a new [ExportProgress]-dialog that takes events from the `receiver`-channel
    fn new(receiver: Receiver<VideoProgress>) -> Self {
        let num = EXPORT_PROGRESS_ID_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            state: ExportProgressState::Creating,
            receiver,
            window_id: format!("export_progress_{num}"),
            grid_id: format!("export_progress_grid_{num}"),
        }
    }

    /// Draws this [ExportProgress].
    ///
    /// When the state is [Done][ExportProgressState::Done] or [Unknown][ExportProgressState::Unknown],
    /// the user may close the dialog.
    ///
    /// This function returns `true` when the user closed the window.
    /// This [ExportProgress] may then be disposed.
    fn draw(&mut self, ctx: &Context) -> bool {
        // Get next event
        match self.receiver.try_recv() {
            Ok(VideoProgress::Encode(cur, max)) => {
                self.state.set_encode(cur, max);
            }
            Ok(VideoProgress::Render(cur, max)) => {
                self.state.set_render(cur, max);
            }
            Ok(VideoProgress::Done(status)) => {
                self.state = ExportProgressState::Done(status);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => match self.state {
                ExportProgressState::Done(_) => {}
                _ => self.state = ExportProgressState::Unknown,
            },
        };

        // Draw window based on state
        let window = Window::new("Export Progress")
            .id(Id::new(&self.window_id))
            .resizable(false)
            .max_width(480.)
            .collapsible(false);
        match self.state {
            ExportProgressState::Creating => {
                window.show(ctx, |ui| {
                    ui.allocate_ui([128., 98.].into(), |ui| {
                        ui.centered_and_justified(|ui| ui.add(Spinner::new().size(76.)))
                    })
                });
                false
            }
            ExportProgressState::Working { encode, render } => {
                window.show(ctx, |ui| {
                    Grid::new(&self.grid_id).show(ui, |ui| {
                        ui.label("Render:");
                        ui.add(ProgressBar::new(render.0 / render.1).show_percentage());
                        ui.label(format!("{:.1}", render.0));
                        ui.end_row();
                        ui.label("Encode:");
                        ui.add(ProgressBar::new(encode.0 / encode.1).show_percentage());
                        ui.label(format!("{:.1}", encode.0));
                        ui.end_row();
                    });
                });
                false
            }
            ExportProgressState::Done(status) => {
                let mut open = true;
                window.open(&mut open).show(ctx, |ui| {
                    if status.success() {
                        ui.label("Finished exporting!");
                    } else if let Some(code) = status.code() {
                        ui.label(format!(
                            "Error during export! FFmpeg exited with code {code}."
                        ));
                    } else {
                        ui.label("Error during export!");
                    }
                });
                !open
            }
            ExportProgressState::Unknown => {
                let mut open = true;
                window.open(&mut open).show(ctx, |ui| {
                    ui.label("An error occurred while exporting!");
                });
                !open
            }
        }
    }
}
