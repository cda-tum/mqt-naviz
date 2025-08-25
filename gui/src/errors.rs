use std::{borrow::Cow, sync::atomic::AtomicU32};

use egui::{Color32, Id, RichText, ScrollArea, Window};

/// Display errors to the user.
///
/// Use [Default::default] to create.
#[derive(Default)]
pub struct Errors(Vec<Error>);

/// A counter for generating unique IDs for [Errors]
static ERRORS_ID_GEN: AtomicU32 = AtomicU32::new(0);

/// Get a new unique ID from [ERRORS_ID_GEN]
fn get_id() -> String {
    format!(
        "error_{}",
        ERRORS_ID_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

/// A single error that is displayed in the application
struct Error {
    /// The window-id
    id: String,
    /// The title of the window
    title: Cow<'static, str>,
    /// The body of the window
    message: String,
}

impl Error {
    /// Creates a new [Error] with an automatic id
    pub fn new(title: Cow<'static, str>, message: String) -> Self {
        Self {
            id: get_id(),
            title,
            message,
        }
    }
}

impl Errors {
    /// Adds a new error-window with the `title` and `body`
    pub fn add(&mut self, title: impl Into<Cow<'static, str>>, body: impl ToString) {
        let title = title.into();
        let body = body.to_string();
        log::error!("{title}:\n{body}");
        self.0.push(Error::new(title, body));
    }

    /// Adds an [Error][crate::error::Error]
    pub fn add_error(&mut self, error: crate::error::Error) {
        self.add(error.title(), error.body());
    }

    /// Draws all the contained error-windows.
    /// Any windows that are closed by the user are automatically cleaned up.
    pub fn draw(&mut self, ctx: &egui::Context) {
        self.0.retain_mut(|error| error.draw(ctx));
    }
}

impl Error {
    /// Draws this [Error].
    /// Will return `false` if this window was closed.
    fn draw(&self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut close_requested = false;

        Window::new(&*self.title)
            .id(Id::new(&self.id))
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .max_width(800.0)
            .show(ctx, |ui| {
                // Add an error icon
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⚠")
                            .color(Color32::from_rgb(255, 165, 0))
                            .size(20.0),
                    );
                    ui.label(RichText::new("Error Details").strong());
                });
                ui.separator();

                // Show the error message in a scrollable area
                ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    ui.label(&self.message);

                    // Add some spacing and helpful actions
                    ui.add_space(10.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Copy to Clipboard").clicked() {
                            ctx.copy_text(format!("{}: {}", self.title, self.message));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                close_requested = true;
                            }
                        });
                    });
                });
            });

        open && !close_requested
    }
}

/// Something that emits an error.
/// Mostly used to add [pipe][ErrorEmitter] to [Result].
pub trait ErrorEmitter {
    type Output;

    /// If this [ErrorEmitter] is an error,
    /// a new error will be added to `errors` and the return of `default` will be returned.
    /// Otherwise returns the contained `Output`.
    fn pipe(self, errors: &mut Errors, default: impl FnOnce() -> Self::Output) -> Self::Output;

    /// If this [ErrorEmitter] is an error,
    /// a new error will be added to `errors`.
    /// If a value is contained,
    /// it will be discarded.
    fn pipe_void(self, errors: &mut Errors);
}

impl<Output> ErrorEmitter for Result<Output, crate::error::Error> {
    type Output = Output;

    fn pipe(self, errors: &mut Errors, default: impl FnOnce() -> Self::Output) -> Self::Output {
        match self {
            Err(e) => {
                errors.add_error(e);
                default()
            }
            Ok(v) => v,
        }
    }

    fn pipe_void(self, errors: &mut Errors) {
        if let Err(e) = self {
            errors.add_error(e);
        }
    }
}
