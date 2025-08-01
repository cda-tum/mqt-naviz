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
#[cfg(not(target_arch = "wasm32"))]
use naviz_video::VideoProgress;
use rfd::FileHandle;

use crate::{
    drawable::Drawable,
    future_helper::{FutureHelper, SendFuture},
    util::WEB,
};

type SendReceivePair<T> = (Sender<T>, Receiver<T>);

/// The menu bar struct which contains the state of the menu
pub struct MenuBar {
    event_channel: SendReceivePair<MenuEvent>,
    /// Whether to draw the about-window
    about_open: bool,
    /// Export interaction handling (menu, config, progress)
    export_menu: ExportMenu,
    /// List of machines: `(id, name, removable)`
    machines: Vec<(String, String, bool)>,
    /// List of styles: `(id, name, removable)`
    styles: Vec<(String, String, bool)>,
    /// The currently selected machine
    selected_machine: Option<String>,
    /// The currently selected style
    selected_style: Option<String>,
    /// Options to display for the current import (as started by the user).
    /// Also optionally contains a file if importing first opened a file
    /// (e.g., by dropping it onto the application).
    /// If no import is currently happening, this is [None].
    current_import_options: Option<(ImportOptions, Option<Arc<[u8]>>)>,
}

/// An event which is triggered on menu navigation.
/// Higher-Level than just button-clicks.
pub enum MenuEvent {
    /// A file of the specified [FileType] with the specified content was opened
    FileOpen(FileType, Arc<[u8]>),
    /// A file should be imported
    FileImport(ImportOptions, Arc<[u8]>),
    /// A video should be exported to the specified path with the specified resolution and fps
    #[cfg(not(target_arch = "wasm32"))]
    ExportVideo {
        target: PathBuf,
        resolution: (u32, u32),
        fps: u32,
        /// Channel for progress updates
        progress: Sender<VideoProgress>,
    },
    /// A new machine with the specified `id` was selected
    SetMachine(String),
    /// A new style with the specified `id` was selected
    SetStyle(String),
    /// The machine at the specified `path` should be imported
    #[cfg(not(target_arch = "wasm32"))]
    ImportMachine(PathBuf),
    /// The style at the specified `path` should be imported
    #[cfg(not(target_arch = "wasm32"))]
    ImportStyle(PathBuf),
    /// The machine with the specified `id` should be removed
    #[cfg(not(target_arch = "wasm32"))]
    RemoveMachine(String),
    /// The style with the specified `id` should be removed
    #[cfg(not(target_arch = "wasm32"))]
    RemoveStyle(String),
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

/// The available FileTypes for opening
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileType {
    Instructions,
    Machine,
    Style,
}

/// Something which can be used to filter files by extension
pub trait FileFilter {
    /// The name of this filter
    fn name(&self) -> &str;
    /// Allowed extensions
    fn extensions(&self) -> &[&str];
}

/// Config options for what to show inside the menu
pub struct MenuConfig {
    /// Show export option
    pub export: bool,
}

impl FileFilter for FileType {
    fn name(&self) -> &'static str {
        match self {
            FileType::Instructions => "NAViz instructions",
            FileType::Machine => "NAViz machine",
            FileType::Style => "NAViz style",
        }
    }
    fn extensions(&self) -> &'static [&'static str] {
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
            event_channel: channel(),
            about_open: false,
            export_menu: ExportMenu::new(),
            machines: vec![],
            styles: vec![],
            selected_machine: None,
            selected_style: None,
            current_import_options: None,
        }
    }

    /// Update the machine-list.
    /// Machines are `(id, name, removable)`.
    pub fn update_machines(&mut self, machines: Vec<(String, String, bool)>) {
        self.machines = machines;
        self.machines.sort_by(|(_, a, _), (_, b, _)| a.cmp(b));
    }

    /// Move the compatible machines to the top of the list
    pub fn set_compatible_machines(&mut self, machines: &[String]) {
        // Sort by containment in machines, then by name
        self.machines
            .sort_by(|(id_a, name_a, _), (id_b, name_b, _)| {
                machines
                    .contains(id_a)
                    .cmp(&machines.contains(id_b))
                    .reverse()
                    .then_with(|| name_a.cmp(name_b))
            });
    }

    /// Sets the currently selected machine to the passed id.
    /// Used to display feedback in the selection.
    pub fn set_selected_machine(&mut self, selected_machine: Option<String>) {
        self.selected_machine = selected_machine;
    }

    /// Update the style-list.
    /// Styles are `(id, name, removable)`.
    pub fn update_styles(&mut self, styles: Vec<(String, String, bool)>) {
        self.styles = styles;
        self.styles.sort_by(|(_, a, _), (_, b, _)| a.cmp(b));
    }

    /// Sets the currently selected style to the passed id.
    /// Used to display feedback in the selection.
    pub fn set_selected_style(&mut self, selected_style: Option<String>) {
        self.selected_style = selected_style;
    }

    /// Get the file open channel.
    ///
    /// Whenever a new file is opened,
    /// its content will be sent over this channel.
    pub fn events(&self) -> &Receiver<MenuEvent> {
        &self.event_channel.1
    }

    /// Loads a file while deducing its type by its extension.
    /// Handles the file-types defined in [FileType]
    /// and file-types defined in [IMPORT_FORMATS].
    /// Extension-collisions will simply pick the first match.
    fn load_file_by_extension(&mut self, name: &str, contents: Arc<[u8]>) {
        // Extract extension
        if let Some(extension) = Path::new(name).extension() {
            let extension = &*extension.to_string_lossy();

            // Internal formats
            for file_type in [FileType::Instructions, FileType::Machine, FileType::Style] {
                // File extension is known?
                if file_type.extensions().contains(&extension) {
                    let _ = self
                        .event_channel
                        .0
                        .send(MenuEvent::FileOpen(file_type, contents));
                    return;
                }
            }

            // Imported formats
            for import_format in IMPORT_FORMATS {
                // File extension is known by some import-format?
                if import_format.file_extensions().contains(&extension) {
                    self.current_import_options = Some((import_format.into(), Some(contents)));
                    return;
                }
            }
        }
    }

    /// Handles any files dropped onto the application.
    /// Will use [Self::load_file_by_extension] to load the file.
    fn handle_file_drop(&mut self, ctx: &egui::Context) {
        for file in ctx.input_mut(|input| std::mem::take(&mut input.raw.dropped_files)) {
            if let Some(contents) = file.bytes {
                self.load_file_by_extension(&file.name, contents);
            }
        }
    }

    /// Handles any pastes into the application.
    /// Will try to check if a file was pasted,
    /// and use [Self::load_file_by_extension] to load the file.
    /// If text was pasted,
    /// will load that text as instructions.
    fn handle_clipboard(&mut self, ctx: &egui::Context) {
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
                            self.load_file_by_extension(text, contents.into());
                        } else {
                            log::error!("Failed to read file");
                        }
                    } else {
                        // Pasted text-content directly
                        let _ = self.event_channel.0.send(MenuEvent::FileOpen(
                            FileType::Instructions,
                            std::mem::take(text).into_bytes().into(),
                        ));
                    }
                    false // event was handled => drop
                } else {
                    true // event was not handled => keep
                }
            })
        });
    }

    /// Draw the [MenuBar]
    pub fn draw(
        &mut self,
        config: MenuConfig,
        future_helper: &FutureHelper,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
    ) {
        self.export_menu.process_events(&mut self.event_channel.0);

        self.show_import_dialog(future_helper, ctx);

        self.handle_file_drop(ctx);

        self.handle_clipboard(ctx);

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

                self.export_menu.draw_button(config.export, ui);

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
            SelectionMenu::machines(self).draw(ui, future_helper, self);

            // Style selection
            SelectionMenu::styles(self).draw(ui, future_helper, self);

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

    /// Show the import dialog if [MenuBar::current_import_options] is `Some`.
    fn show_import_dialog(&mut self, future_helper: &FutureHelper, ctx: &egui::Context) {
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
                    let _ = self
                        .event_channel
                        .0
                        .send(MenuEvent::FileImport(options, import_file));
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

/// Top menu allowing to select styles or machines.
/// Use [SelectionMenu::machines] or [SelectionMenu::styles] to get the respective menus
/// and [SelectionMenu::draw] to draw the menus.
struct SelectionMenu<
    'a,
    SET: Fn(String) -> MenuEvent + Copy,
    REMOVE: Fn(String) -> MenuEvent + Copy,
> {
    /// Name of the button
    name: &'static str,
    /// [FileType] this menu is for
    file_type: FileType,
    /// [MenuEvent] to fire when the machine should be set
    set_event: SET,
    /// [MenuEvent] to fire when the machine should be removed
    remove_event: REMOVE,
    /// Items to display: `(id, name, removable)`
    items: &'a [(String, String, bool)],
    /// Which id is currently selected
    selected: &'a Option<String>,
}

impl<'a> SelectionMenu<'a, fn(String) -> MenuEvent, fn(String) -> MenuEvent> {
    /// Create a new [SelectionMenu] that allows selecting the [MenuBar]'s machines.
    #[inline(always)]
    fn machines(menu_bar: &'a MenuBar) -> Self {
        Self {
            name: "Machines",
            file_type: FileType::Machine,
            set_event: MenuEvent::SetMachine,
            #[cfg(not(target_arch = "wasm32"))]
            remove_event: MenuEvent::RemoveMachine,
            #[cfg(target_arch = "wasm32")]
            remove_event: |_| unreachable!("No imported configs on web"),
            items: &menu_bar.machines,
            selected: &menu_bar.selected_machine,
        }
    }

    /// Create a new [SelectionMenu] that allows selecting the [MenuBar]'s styles.
    #[inline(always)]
    fn styles(menu_bar: &'a MenuBar) -> Self {
        SelectionMenu {
            name: "Styles",
            file_type: FileType::Style,
            set_event: MenuEvent::SetStyle,
            #[cfg(not(target_arch = "wasm32"))]
            remove_event: MenuEvent::RemoveStyle,
            #[cfg(target_arch = "wasm32")]
            remove_event: |_| unreachable!("No imported configs on web"),
            items: &menu_bar.styles,
            selected: &menu_bar.selected_style,
        }
    }
}

impl<SET: Fn(String) -> MenuEvent + Copy, REMOVE: Fn(String) -> MenuEvent + Copy>
    SelectionMenu<'_, SET, REMOVE>
{
    /// Draws this [SelectionMenu] to the passed [Ui][egui::Ui],
    /// reacting to events by calling the functions of [MenuBar]
    /// or sending events through its channel.
    #[inline(always)]
    fn draw(&self, ui: &mut egui::Ui, future_helper: &FutureHelper, menu_bar: &MenuBar) {
        let &Self {
            name,
            file_type,
            set_event,
            remove_event,
            items,
            selected,
        } = self;

        ui.menu_button(name, |ui| {
            if ui.button("Open").clicked() {
                menu_bar.choose_file(file_type, future_helper, MenuEvent::file_open);
                ui.close_menu();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button("Import").clicked() {
                menu_bar.choose_file(file_type, future_helper, MenuEvent::file_import);
                ui.close_menu();
            }

            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                // Store previous widths to layout

                // IDs
                let delete_width_id = Id::new(format!("menu.{name}.width#delete"));
                let full_width_id = Id::new(format!("menu.{name}.width#full"));
                let hovered_index_id = Id::new(format!("menu.{name}.width#hovered"));

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

                for (idx, (id, name, removable)) in items.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let show_delete_button = *removable && hovered == Some(idx);

                        // Width for the main button
                        let button_width = if show_delete_button {
                            full_width - delete_width
                        } else {
                            full_width
                        };

                        // Render select button
                        let select_button = ui
                            .with_layout(
                                Layout::left_to_right(Align::Center).with_main_align(Align::Min),
                                |ui| {
                                    ui.add(
                                        Button::new(name)
                                            .selected(selected.as_ref() == Some(id))
                                            .min_size([button_width, 0.].into()),
                                    )
                                },
                            )
                            .inner;
                        if select_button.clicked() {
                            let _ = menu_bar.event_channel.0.send(set_event(id.clone()));
                            ui.close_menu();
                        }

                        // Render delete button
                        if show_delete_button {
                            let delete_button =
                                ui.centered_and_justified(|ui| ui.button("\u{1F5D1}")).inner;
                            if delete_button.clicked() {
                                let _ = menu_bar.event_channel.0.send(remove_event(id.clone()));
                            }
                            delete_width_ =
                                Some(delete_button.rect.right() - select_button.rect.right());
                        }

                        // Check if current entry is hovered
                        if ui.ui_contains_pointer() {
                            ui.data_mut(|data| data.insert_temp(hovered_index_id, idx));
                        }
                    });
                }

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

    use std::{
        path::PathBuf,
        sync::mpsc::{channel, Sender},
    };

    use egui::{Button, Context};

    use crate::{
        export_dialog::{ExportProgresses, ExportSettings},
        future_helper::FutureHelper,
    };

    use super::{MenuEvent, SendReceivePair};

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
        pub fn process_events(&mut self, menu_event_sender: &mut Sender<MenuEvent>) {
            if let Ok((target, resolution, fps)) = self.export_channel.1.try_recv() {
                let _ = menu_event_sender.send(MenuEvent::ExportVideo {
                    target,
                    resolution,
                    fps,
                    progress: self.export_progresses.add(),
                });
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

    use std::sync::mpsc::Sender;

    use egui::Context;

    use crate::future_helper::FutureHelper;

    use super::MenuEvent;

    pub struct ExportMenu {}

    impl ExportMenu {
        pub fn new() -> Self {
            Self {}
        }

        pub fn process_events(&mut self, _menu_event_sender: &mut Sender<MenuEvent>) {}

        pub fn draw_button(&mut self, _enabled: bool, _ui: &mut egui::Ui) {}

        pub fn draw_windows(&mut self, _future_helper: &FutureHelper, _ctx: &Context) {}
    }
}
