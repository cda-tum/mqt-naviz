//! [MenuBar] to show a menu on the top.

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::{
    path::Path,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use egui::{Align, Align2, Button, Grid, Id, Layout, ScrollArea, Window};
use export::ExportMenu;
use git_version::git_version;
use naviz_import::{ImportFormat, ImportOptions, IMPORT_FORMATS};
use rfd::FileHandle;

use crate::{
    app::AppState,
    drawable::Drawable,
    error::{Error, Result},
    errors::{ErrorEmitter, Errors},
    file_type::{FileFilter, FileType},
    future_helper::{FutureHelper, SendFuture},
    util::WEB,
};

type SendReceivePair<T> = (Sender<T>, Receiver<T>);

/// The menu bar struct which contains the state of the menu
pub struct MenuBar {
    /// Internal channel for async events
    event_channel: SendReceivePair<MenuEvent>,
    /// Whether to draw the about-window
    about_open: bool,
    /// Export interaction handling (menu, config, progress)
    export_menu: ExportMenu,
    /// Options to display for the current import (as started by the user).
    /// Also optionally contains a file if importing first opened a file
    /// (e.g., by dropping it onto the application).
    /// If no import is currently happening, this is [None].
    current_import_options: Option<(ImportOptions, Option<Arc<[u8]>>)>,
}

/// An event which can be triggered by asynchronous actions like the user choosing a file
enum MenuEvent {
    /// A file of the specified [FileType] with the specified content was opened
    FileOpen(FileType, Arc<[u8]>),
    /// A file should be imported
    FileImport(ImportOptions, Arc<[u8]>),
    /// The machine at the specified `path` should be imported
    #[cfg(not(target_arch = "wasm32"))]
    ImportMachine(PathBuf),
    /// The style at the specified `path` should be imported
    #[cfg(not(target_arch = "wasm32"))]
    ImportStyle(PathBuf),
}

impl MenuEvent {
    /// Creates a [MenuEvent::FileOpen] for [MenuBar::choose_file]
    async fn file_open(file_type: FileType, handle: FileHandle) -> Self {
        Self::FileOpen(file_type, handle.read().await.into())
    }

    /// Creates a [MenuEvent::ImportMachine] or [MenuEvent::ImportStyle] for [MenuBar::choose_file]
    #[cfg(not(target_arch = "wasm32"))]
    async fn file_import(file_type: FileType, handle: FileHandle) -> Self {
        match file_type {
            FileType::Instructions => panic!("Unable to import instructions"),
            FileType::Machine => Self::ImportMachine(handle.path().to_owned()),
            FileType::Style => Self::ImportStyle(handle.path().to_owned()),
        }
    }
}

impl MenuBar {
    /// Create a new [MenuBar]
    pub fn new() -> Self {
        Self {
            event_channel: channel(),
            about_open: false,
            export_menu: ExportMenu::new(),
            current_import_options: None,
        }
    }

    /// Loads a file while deducing its type by its extension.
    /// Handles the file-types defined in [FileType]
    /// and file-types defined in [IMPORT_FORMATS].
    /// Extension-collisions will simply pick the first match.
    fn load_file_by_extension(
        &mut self,
        name: &str,
        contents: Arc<[u8]>,
        state: &mut AppState,
    ) -> Result<()> {
        // Extract extension
        if let Some(extension) = Path::new(name).extension() {
            let extension = &*extension.to_string_lossy();

            // Internal formats
            for file_type in [FileType::Instructions, FileType::Machine, FileType::Style] {
                // File extension is known?
                if file_type.extensions().contains(&extension) {
                    return state.open_by_type(file_type, &contents);
                }
            }

            // Imported formats
            for import_format in IMPORT_FORMATS {
                // File extension is known by some import-format?
                if import_format.file_extensions().contains(&extension) {
                    // Set current import options to show dialog
                    self.current_import_options = Some((import_format.into(), Some(contents)));
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Handles any files dropped onto the application.
    /// Will use [Self::load_file_by_extension] to load the file.
    fn handle_file_drop(&mut self, ctx: &egui::Context, state: &mut AppState) -> Result<()> {
        for file in ctx.input_mut(|input| std::mem::take(&mut input.raw.dropped_files)) {
            if let Some(contents) = file.bytes {
                self.load_file_by_extension(&file.name, contents, state)?;
            }
        }
        Ok(())
    }

    /// Handles any pastes into the application.
    /// Will try to check if a file was pasted,
    /// and use [Self::load_file_by_extension] to load the file.
    /// If text was pasted,
    /// will load that text as instructions.
    fn handle_clipboard(&mut self, ctx: &egui::Context, state: &mut AppState, errors: &mut Errors) {
        if ctx.wants_keyboard_input() {
            // some widgets is listening for keyboard input,
            // therefore it also consumes paste events.
            // When something is pasted, it should only go to that widget
            // and not also try to load as instructions.
            return;
        }

        ctx.input_mut(|i| {
            i.events.retain_mut(|e| {
                if let egui::Event::Paste(text) = e {
                    // egui does not allow listening for file-paste-events directly:
                    // https://github.com/emilk/egui/issues/1167
                    // Instead, check if the pasted text is a file-path that exists.
                    // Don't do this on web, as web will not receive file-pastes this way.
                    if !WEB && Path::new(text).is_file() {
                        // A file exists at that path
                        #[allow(clippy::needless_borrows_for_generic_args)] // borrow is needed
                        if let Ok(contents) = std::fs::read(&text) {
                            self.load_file_by_extension(text, contents.into(), state)
                                .pipe_void(errors);
                        } else {
                            log::error!("Failed to read file");
                        }
                    } else {
                        // Pasted text-content directly
                        state.open(text.as_bytes()).pipe_void(errors);
                    }
                    false // event was handled => drop
                } else {
                    true // event was not handled => keep
                }
            })
        });
    }

    /// Processes all events from the [event_channel][MenuBar::event_channel].
    fn process_events(&mut self, state: &mut AppState, errors: &mut Errors) {
        while let Ok(event) = self.event_channel.1.try_recv() {
            match event {
                MenuEvent::FileOpen(file_type, data) => state.open_by_type(file_type, &data),
                MenuEvent::FileImport(import_options, data) => {
                    state.import(import_options, &data).map_err(Error::Import)
                }
                #[cfg(not(target_arch = "wasm32"))]
                MenuEvent::ImportMachine(path) => state.import_machine(&path),
                #[cfg(not(target_arch = "wasm32"))]
                MenuEvent::ImportStyle(path) => state.import_style(&path),
            }
            .pipe_void(errors);
        }
    }

    /// Draw the [MenuBar].
    /// State will be taken from the [AppState]
    /// and any interactions with the menu will update the [AppState].
    pub fn draw(
        &mut self,
        state: &mut AppState,
        errors: &mut Errors,
        future_helper: &FutureHelper,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
    ) {
        self.process_events(state, errors);

        self.export_menu.process_events(state);

        self.show_import_dialog(state, future_helper, ctx)
            .pipe_void(errors);

        self.handle_file_drop(ctx, state).pipe_void(errors);

        self.handle_clipboard(ctx, state, errors);

        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
                    self.choose_file(FileType::Instructions, future_helper, MenuEvent::file_open);
                    ui.close_menu();
                }

                ui.menu_button("Import", |ui| {
                    for import_format in IMPORT_FORMATS {
                        if ui.button(import_format.name()).clicked() {
                            self.current_import_options = Some((import_format.into(), None));
                            ui.close_menu();
                        }
                    }
                });

                self.export_menu
                    .draw_button(state.visualization_loaded(), ui);

                if !WEB {
                    // Quit-button only on native
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close_menu();
                    }
                }
            });

            // Machine selection
            selection_menu::draw::<selection_menu::Machine>(state, ui, future_helper, self, errors);

            // Style selection
            selection_menu::draw::<selection_menu::Style>(state, ui, future_helper, self, errors);

            // View-menu
            ui.menu_button("View", |ui| {
                // Zen-overwrite
                let mut force_zen = state.get_force_zen();
                if ui.checkbox(&mut force_zen, "Zen-Mode").changed() {
                    state.set_force_zen(force_zen);
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    self.about_open = true;
                    ui.close_menu();
                }
            });
        });

        self.export_menu.draw_windows(future_helper, ctx);

        self.draw_about_window(ctx);
    }

    /// Show the import dialog if [MenuBar::current_import_options] is `Some`
    /// and update the [AppState] if the import should be performed.
    fn show_import_dialog(
        &mut self,
        state: &mut AppState,
        future_helper: &FutureHelper,
        ctx: &egui::Context,
    ) -> Result<()> {
        if let Some((current_import_options, _)) = self.current_import_options.as_mut() {
            let mut open = true; // window open?
            let mut do_import = false; // ok button clicked?

            Window::new("Import")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    current_import_options.draw(ui);
                    do_import = ui
                        .vertical_centered_justified(|ui| ui.button("Ok"))
                        .inner
                        .clicked();
                });

            if do_import {
                let (options, import_file) = self.current_import_options.take().unwrap(); // Can unwrap because we are inside of `if let Some`
                if let Some(import_file) = import_file {
                    // An import-file was already opened => import that file
                    state.import(options, &import_file).map_err(Error::Import)?;
                } else {
                    // No import-file opened => LEt user choose file
                    self.choose_file(
                        ImportFormat::from(&options),
                        future_helper,
                        |_, file| async move {
                            MenuEvent::FileImport(options, file.read().await.into())
                        },
                    );
                }
            }

            if !open {
                self.current_import_options = None;
            }
        }

        Ok(())
    }

    /// Show the file-choosing dialog and read the file if a new file was selected
    fn choose_file<
        Arg: FileFilter + Send + 'static,
        EvFut,
        F: FnOnce(Arg, FileHandle) -> EvFut + Send + 'static,
    >(
        &self,
        file_type: Arg,
        future_helper: &FutureHelper,
        mk_event: F,
    ) where
        EvFut: SendFuture<MenuEvent>,
    {
        future_helper.execute_maybe_to(
            async move {
                if let Some(handle) = rfd::AsyncFileDialog::new()
                    .add_filter(file_type.name(), file_type.extensions())
                    .pick_file()
                    .await
                {
                    Some(mk_event(file_type, handle).await)
                } else {
                    None
                }
            },
            self.event_channel.0.clone(),
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
                    ui.label(VERSION);
                    ui.end_row();

                    ui.label("Build");
                    ui.label(git_version!(
                        args = ["--always", "--dirty=+dev", "--match="],
                        fallback = "unknown"
                    ));
                    ui.end_row();

                    ui.label("GUI-Version");
                    ui.label(env!("CARGO_PKG_VERSION"));
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

mod selection_menu {
    //! Module containing the top-menus for selecting styles or machines.
    //! The [Menu]-trait specifies what should be displayed and how updates should be handled.
    //! The menu can be drawn by passing one such struct to [draw].

    use super::*;

    /// Something that can be displayed as a selection-menu.
    /// Specifies how data can be retrieved and updated.
    pub trait Menu {
        const NAME: &str;
        const FILE_TYPE: FileType;
        fn set(state: &mut AppState, id: &str) -> Result<()>;
        fn remove(state: &mut AppState, id: &str) -> Result<()>;
        fn items(state: &AppState) -> impl Iterator<Item = (&str, &str, bool)>;
        fn selected(state: &AppState) -> Option<&str>;
    }

    /// Selection-menu for the machines.
    pub struct Machine;
    impl Menu for Machine {
        const NAME: &str = "Machines";
        const FILE_TYPE: FileType = FileType::Machine;
        fn set(state: &mut AppState, id: &str) -> Result<()> {
            state.set_machine(id)
        }
        #[cfg(not(target_arch = "wasm32"))]
        fn remove(state: &mut AppState, id: &str) -> Result<()> {
            state.remove_machine(id)
        }
        #[cfg(target_arch = "wasm32")]
        fn remove(_state: &mut AppState, _id: &str) -> Result<()> {
            unimplemented!("Cannot remove machines in web")
        }
        fn items(state: &AppState) -> impl Iterator<Item = (&str, &str, bool)> {
            state.get_machines()
        }
        fn selected(state: &AppState) -> Option<&str> {
            state.get_current_machine_id()
        }
    }

    /// Selection-menu for the styles.
    pub struct Style;
    impl Menu for Style {
        const NAME: &str = "Styles";
        const FILE_TYPE: FileType = FileType::Style;
        fn set(state: &mut AppState, id: &str) -> Result<()> {
            state.set_style(id)
        }
        #[cfg(not(target_arch = "wasm32"))]
        fn remove(state: &mut AppState, id: &str) -> Result<()> {
            state.remove_style(id)
        }
        #[cfg(target_arch = "wasm32")]
        fn remove(_state: &mut AppState, _id: &str) -> Result<()> {
            unimplemented!("Cannot remove styles in web")
        }
        fn items(state: &AppState) -> impl Iterator<Item = (&str, &str, bool)> {
            state.get_styles()
        }
        fn selected(state: &AppState) -> Option<&str> {
            state.get_current_style_id()
        }
    }

    /// Draws a selection-menu to the passed [Ui][egui::Ui],
    /// reacting to events by calling the functions of [MenuBar]
    /// or updating [AppState] directly.
    ///
    /// The [Menu] to display can be selected by specifying the generic parameter.
    #[inline(always)]
    pub fn draw<M: Menu>(
        state: &mut AppState,
        ui: &mut egui::Ui,
        future_helper: &FutureHelper,
        menu_bar: &MenuBar,
        errors: &mut Errors,
    ) {
        let selected = M::selected(state).map(ToString::to_string);

        /// Action to take.
        /// Required to collect all actions while iterating.
        enum Action {
            Set(String),
            Remove(String),
        }
        impl Action {
            pub fn execute<M: Menu>(self, state: &mut AppState) -> Result<()> {
                match self {
                    Action::Set(id) => M::set(state, &id),
                    Action::Remove(id) => M::remove(state, &id),
                }
            }
        }

        ui.menu_button(M::NAME, |ui| {
            if ui.button("Open").clicked() {
                menu_bar.choose_file(M::FILE_TYPE, future_helper, MenuEvent::file_open);
                ui.close_menu();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button("Import").clicked() {
                menu_bar.choose_file(M::FILE_TYPE, future_helper, MenuEvent::file_import);
                ui.close_menu();
            }

            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                // Store previous widths to layout

                // IDs
                let delete_width_id = Id::new(format!("menu.{}.width#delete", M::NAME));
                let full_width_id = Id::new(format!("menu.{}.width#full", M::NAME));
                let hovered_index_id = Id::new(format!("menu.{}.width#hovered", M::NAME));

                // Values
                // Width of delete button
                let delete_width: f32 = ui
                    .data(|data| data.get_temp(delete_width_id))
                    .unwrap_or_default();
                // Full width of menu
                let full_width: f32 = ui
                    .data(|data| data.get_temp(full_width_id))
                    .unwrap_or_default();
                // Currently hovered index
                let hovered: Option<usize> = ui.data(|data| data.get_temp(hovered_index_id));

                // New `delete_width`
                let mut delete_width_ = None;

                M::items(state)
                    .enumerate()
                    .flat_map(|(idx, (id, name, removable))| {
                        ui.horizontal(|ui| {
                            // List of actions to take for this entry
                            let mut actions = Vec::new();

                            let show_delete_button = removable && hovered == Some(idx);

                            // Width for the main button
                            let button_width = if show_delete_button {
                                full_width - delete_width
                            } else {
                                full_width
                            };

                            // Render select button
                            let select_button = ui
                                .with_layout(
                                    Layout::left_to_right(Align::Center)
                                        .with_main_align(Align::Min),
                                    |ui| {
                                        ui.add(
                                            Button::new(name)
                                                .selected(selected.as_deref() == Some(id))
                                                .min_size([button_width, 0.].into()),
                                        )
                                    },
                                )
                                .inner;
                            if select_button.clicked() {
                                actions.push(Action::Set(id.to_string()));
                                ui.close_menu();
                            }

                            // Render delete button
                            if show_delete_button {
                                let delete_button =
                                    ui.centered_and_justified(|ui| ui.button("\u{1F5D1}")).inner;
                                if delete_button.clicked() {
                                    actions.push(Action::Remove(id.to_string()));
                                }
                                delete_width_ =
                                    Some(delete_button.rect.right() - select_button.rect.right());
                            }

                            // Check if current entry is hovered
                            if ui.ui_contains_pointer() {
                                ui.data_mut(|data| data.insert_temp(hovered_index_id, idx));
                            }

                            actions
                        })
                        .inner
                    })
                    // First collect all actions and then act due to lifetime-issues when trying
                    // to use `state` inside the closure.
                    .collect::<Vec<_>>()
                    .into_iter()
                    .for_each(|a| a.execute::<M>(state).pipe_void(errors));

                // Cursor not over current ui => no element hovered
                if !ui.ui_contains_pointer() {
                    ui.data_mut(|data| data.remove_temp::<usize>(hovered_index_id));
                }
                // Update delete width
                if let Some(delete_width_) = delete_width_ {
                    ui.data_mut(|data| data.insert_temp(delete_width_id, delete_width_));
                }
                // Update full width
                ui.data_mut(|data| data.insert_temp(full_width_id, ui.min_rect().width()));
            });
        });
    }
}

/// The version of the full program, determined at compile-time.
/// Will end with a `~` if dirty,
/// a `+` if commits exist after the version,
/// or a `+~` if both are true.
const VERSION: &str = {
    // Get the exact version (if exists)
    const EXACT_VERSION: &str = git_version!(
        args = ["--dirty=~", "--abbrev=0", "--match=v*", "--exact"],
        fallback = ""
    );
    // The previous version or "unknown"
    const LATEST_VERSION: &str = git_version!(args = ["--abbrev=0", "--match=v*"], fallback = "");
    // Whether the build is dirty
    const DIRTY: bool = konst::string::ends_with(
        git_version!(args = ["--dirty=~", "--match=", "--always"], fallback = ""),
        "~",
    );
    // String-representation of `DIRTY`
    const DIRTY_STR: &str = if DIRTY { "~" } else { "" };

    #[allow(clippy::const_is_empty)] // Only empty without exact version
    match (EXACT_VERSION.is_empty(), LATEST_VERSION.is_empty()) {
        (false, _) => EXACT_VERSION,
        (true, false) => constcat::concat!(LATEST_VERSION, "+", DIRTY_STR),
        (true, true) => "unknown",
    }
};

#[cfg(not(target_arch = "wasm32"))]
pub mod export {
    //! Export-Menu on native

    use std::{path::PathBuf, sync::mpsc::channel};

    use egui::{Button, Context};

    use crate::{
        app::AppState,
        export_dialog::{ExportProgresses, ExportSettings},
        future_helper::FutureHelper,
    };

    use super::SendReceivePair;

    /// Menu components concerning export
    pub struct ExportMenu {
        /// Channel for selected export-settings
        export_channel: SendReceivePair<(PathBuf, (u32, u32), u32)>,
        /// The export-settings-dialog to show when the user wants to export a video
        export_settings: ExportSettings,
        /// The export-progress-dialogs to show
        export_progresses: ExportProgresses,
    }

    impl ExportMenu {
        /// Creates a new [ExportMenu]
        pub fn new() -> Self {
            Self {
                export_channel: channel(),
                export_settings: Default::default(),
                export_progresses: Default::default(),
            }
        }

        /// Processes events concerning export
        pub fn process_events(&mut self, state: &mut AppState) {
            if let Ok((target, resolution, fps)) = self.export_channel.1.try_recv() {
                state.export(target, resolution, fps, self.export_progresses.add());
            }
        }

        /// Draws the menu button concerning export
        pub fn draw_button(&mut self, enabled: bool, ui: &mut egui::Ui) {
            if ui
                .add_enabled(enabled, Button::new("Export Video"))
                .clicked()
            {
                self.export_settings.show();
                ui.close_menu();
            }
        }

        /// Draws the windows concerning video export
        pub fn draw_windows(&mut self, future_helper: &FutureHelper, ctx: &Context) {
            if self.export_settings.draw(ctx) {
                self.export(future_helper);
            }

            self.export_progresses.draw(ctx);
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
                        .map(|target| (target, resolution, fps))
                },
                self.export_channel.0.clone(),
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub mod export {
    //! Export-Menu-Stub for web (does not exist on web platforms)
    //!
    //! Signatures should match the export-module on native.
    //! See that module for documentation.

    use egui::Context;

    use crate::{app::AppState, future_helper::FutureHelper};

    pub struct ExportMenu {}

    impl ExportMenu {
        pub fn new() -> Self {
            Self {}
        }

        pub fn process_events(&mut self, _state: &mut AppState) {}

        pub fn draw_button(&mut self, _enabled: bool, _ui: &mut egui::Ui) {}

        pub fn draw_windows(&mut self, _future_helper: &FutureHelper, _ctx: &Context) {}
    }
}
