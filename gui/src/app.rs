use core::str;
use std::ops::{Deref, DerefMut};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
};

use eframe::egui_wgpu::CallbackTrait;
use log::error;
use naviz_import::{ImportError, ImportOptions};
use naviz_parser::config::{machine::MachineConfig, visual::VisualConfig};
use naviz_renderer::renderer::Renderer;
use naviz_repository::Repository;
use naviz_state::{config::Config, state::State};
#[cfg(not(target_arch = "wasm32"))]
use naviz_video::{VideoExport, VideoProgress};

use crate::{
    animator_adapter::{AnimatorAdapter, AnimatorState},
    aspect_panel::AspectPanel,
    canvas::{CanvasContent, EmptyCanvas, WgpuCanvas},
    current_machine::CurrentMachine,
    error::{
        ConfigError, ConfigFormat, Error, InputError, InputType, RepositoryError,
        RepositoryLoadSource, Result,
    },
    errors::{ErrorEmitter, Errors},
    file_type::FileType,
    future_helper::FutureHelper,
    init::{IdOrManual, InitOptions, Persistence},
    menu::MenuBar,
    util::WEB,
};

/// The main App to draw using [egui]/[eframe].
///
/// Can be dereferenced to [AppState] to interface with and update the state.
pub struct App {
    state: AppState,
    ui: AppUI,
    future_helper: FutureHelper,
}

/// The state of the app.
/// Contains all internal state and the interface to update the app's state.
/// Is contained in the [App].
pub struct AppState {
    animator_adapter: AnimatorAdapter,
    machine_repository: Repository,
    style_repository: Repository,
    current_machine: CurrentMachine,
    current_style_id: Option<String>,
    persistence: Persistence,
    cache: AppCache,
}

/// Caches some states of the app for operations such as sorting.
#[derive(Default)]
struct AppCache {
    /// sorted machine list
    machines: Vec<(String, String, bool)>,
    /// sorted style list
    styles: Vec<(String, String, bool)>,
}

impl AppCache {
    /// Materializes the [list][Repository::list] of the passed [Repository].
    fn materialize(repo: &Repository) -> Vec<(String, String, bool)> {
        repo.list()
            .map(|(id, name, removable)| (id.to_string(), name.to_string(), removable))
            .collect()
    }

    /// Iterates over the passed list
    fn iter(vec: &[(String, String, bool)]) -> impl Iterator<Item = (&str, &str, bool)> {
        vec.iter()
            .map(|(id, name, removable)| (id.as_str(), name.as_str(), *removable))
    }

    /// Updates the cached list of machines from the passed machine-[Repository] and the passed list of compatible machine-IDs
    pub fn update_machines(&mut self, machine_repository: &Repository, compatible: &[String]) {
        self.machines = Self::materialize(machine_repository);
        self.machines
            .sort_by(|(a_id, a_name, _), (b_id, b_name, _)| {
                compatible
                    .contains(a_id)
                    .cmp(&compatible.contains(b_id))
                    .reverse()
                    .then_with(|| a_name.cmp(b_name))
            });
    }

    /// Updates the cached list of styles from the passed style-[Repository]
    pub fn update_styles(&mut self, style_repository: &Repository) {
        self.styles = Self::materialize(style_repository);
        self.styles
            .sort_by(|(_, a_name, _), (_, b_name, _)| a_name.cmp(b_name));
    }

    /// Gets the (cached and sorted) list of machines
    pub fn machines(&self) -> impl Iterator<Item = (&str, &str, bool)> {
        Self::iter(&self.machines)
    }

    /// Gets the (cached and sorted) list of styles
    pub fn styles(&self) -> impl Iterator<Item = (&str, &str, bool)> {
        Self::iter(&self.styles)
    }
}

/// The UI-elements of the [App].
struct AppUI {
    menu_bar: MenuBar,
    errors: Errors,
}

impl Deref for App {
    type Target = AppState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl App {
    /// Creates a new [App] by constructing an [AppState].
    /// Also [sets up][RendererAdapter::setup] a [RendererAdapter].
    ///
    /// Can be used to delegate to one of the [AppState]-constructors.
    fn new_from_state(
        cc: &eframe::CreationContext<'_>,
        constructor: impl FnOnce(&mut Errors) -> AppState,
    ) -> Self {
        let mut errors = Errors::default();

        RendererAdapter::setup(cc);

        Self {
            state: constructor(&mut errors),
            ui: AppUI {
                menu_bar: MenuBar::new(),
                errors,
            },
            future_helper: FutureHelper::new().expect("Failed to create FutureHelper"), // This is unrecoverable
        }
    }

    /// Create a new instance of the [App]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_from_state(cc, AppState::new)
    }

    /// Create a new instance of the [App] with the specified [InitOptions]
    pub fn new_with_init(cc: &eframe::CreationContext<'_>, init_options: InitOptions<'_>) -> Self {
        Self::new_from_state(cc, |errors| AppState::new_with_init(init_options, errors))
    }

    /// Create a new instance of the [AppState] with the specified [InitOptions]
    /// and loading the last persisted state.
    /// The passed [InitOptions] will overwrite any persisted options.
    pub fn new_with_init_and_persistence(
        cc: &eframe::CreationContext<'_>,
        init_options: InitOptions<'_>,
    ) -> Self {
        Self::new_from_state(cc, |errors| {
            AppState::new_with_init_and_persistence(Persistence::load(cc), init_options, errors)
        })
    }

    /// Gets the [Errors]-instance of this [App],
    /// which can be used to pipe [Error]s to.
    pub fn errors(&mut self) -> &mut Errors {
        &mut self.ui.errors
    }
}

impl AppState {
    /// Create a new instance of the [AppState].
    /// Errors will be piped to the passed [Errors].
    fn new(errors: &mut Errors) -> Self {
        let mut machine_repository = Repository::empty()
            .bundled_machines()
            .map_err(|e| {
                Error::Repository(
                    RepositoryError::Load(RepositoryLoadSource::Bundled, e),
                    ConfigFormat::Machine,
                )
            })
            .pipe(errors, Repository::empty);
        let mut style_repository = Repository::empty()
            .bundled_styles()
            .map_err(|e| {
                Error::Repository(
                    RepositoryError::Load(RepositoryLoadSource::Bundled, e),
                    ConfigFormat::Style,
                )
            })
            .pipe(errors, Repository::empty);

        // Load user-dirs only on non-web builds as there is no filesystem on web
        if !WEB {
            machine_repository = machine_repository
                .user_dir_machines()
                .map_err(|e| {
                    Error::Repository(
                        RepositoryError::Load(RepositoryLoadSource::UserDir, e),
                        ConfigFormat::Machine,
                    )
                })
                .pipe(errors, Repository::empty);
            style_repository = style_repository
                .user_dir_styles()
                .map_err(|e| {
                    Error::Repository(
                        RepositoryError::Load(RepositoryLoadSource::UserDir, e),
                        ConfigFormat::Machine,
                    )
                })
                .pipe(errors, Repository::empty);
        }

        let mut app = Self {
            animator_adapter: AnimatorAdapter::default(),
            machine_repository,
            style_repository,
            current_machine: Default::default(),
            current_style_id: None,
            persistence: Default::default(),
            cache: Default::default(),
        };

        app.update_machines();
        app.update_styles();

        // Load any machine as default (if any machine is available)
        if let Some((id, machine)) = app.machine_repository.try_get_any() {
            app.set_loaded_machine(Some(id.to_string()), machine);
        }
        // Load any style as default (if any style is available)
        if let Some((id, style)) = app.style_repository.try_get_any() {
            app.set_loaded_style(Some(id.to_string()), style);
        }

        app
    }

    /// Create a new instance of the [AppState] with the specified [InitOptions].
    /// Errors will be piped to the passed [Errors].
    fn new_with_init(init_options: InitOptions<'_>, errors: &mut Errors) -> Self {
        let mut app = Self::new(errors);

        if let Some((import_options, data)) = init_options.input {
            match import_options {
                Some(import_options) => app.import(import_options, data).map_err(Error::Import),
                None => app.open(data),
            }
            .pipe_void(errors)
        }

        if let Some(machine) = init_options.machine {
            match machine {
                IdOrManual::Id(machine_id) => app.set_machine(machine_id),
                IdOrManual::Manual(data) => app.set_machine_manually(data),
            }
            .pipe_void(errors)
        }
        if let Some(style) = init_options.style {
            match style {
                IdOrManual::Id(style_id) => app.set_style(style_id),
                IdOrManual::Manual(data) => app.set_style_manually(data),
            }
            .pipe_void(errors)
        }

        app
    }

    /// Create a new instance of the [AppState] with the specified [InitOptions] and [Persistence].
    /// The passed [InitOptions] will overwrite options in [Persistence].
    /// Errors will be piped to the passed [Errors].
    fn new_with_init_and_persistence(
        persistence: Option<Persistence>,
        init_options: InitOptions<'_>,
        errors: &mut Errors,
    ) -> Self {
        if let Some(persistence) = persistence {
            let persisted: InitOptions<'_> = (&persistence).into();
            Self::new_with_init(persisted.merge(init_options), errors)
        } else {
            // Nothing previously persisted
            Self::new_with_init(init_options, errors)
        }
    }

    /// Import the instructions from `data` using the specified [ImportOptions]
    pub fn import(
        &mut self,
        import_options: ImportOptions,
        data: &[u8],
    ) -> Result<(), ImportError> {
        let instructions = import_options.import(data)?;
        self.animator_adapter.set_instructions(instructions);
        self.update_machines(); // update compatible machines
        Ok(())
    }

    /// Open the naviz-instructions from `data`
    pub fn open(&mut self, data: &[u8]) -> Result<()> {
        let input = naviz_parser::input::lexer::lex(
            str::from_utf8(data)
                .map_err(|e| Error::FileOpen(InputType::Instruction(InputError::UTF8(e))))?,
        )
        .map_err(|e| Error::FileOpen(InputType::Instruction(InputError::Lex(e.into_inner()))))?;
        let input = naviz_parser::input::parser::parse(&input).map_err(|e| {
            Error::FileOpen(InputType::Instruction(InputError::Parse(e.into_inner())))
        })?;
        let input = naviz_parser::input::concrete::Instructions::new(input)
            .map_err(|e| Error::FileOpen(InputType::Instruction(InputError::Convert(e))))?;
        self.animator_adapter.set_instructions(input);
        self.update_machines(); // update compatible machines
        self.select_compatible_machine()?;
        Ok(())
    }

    /// Opens a file by [FileType].
    pub fn open_by_type(&mut self, file_type: FileType, data: &[u8]) -> Result<()> {
        match file_type {
            FileType::Instructions => self.open(data),
            FileType::Machine => self.set_machine_manually(data),
            FileType::Style => self.set_style_manually(data),
        }
    }

    /// Selects any compatible machine for the currently opened machine.
    /// Returns `true` if a compatible machine could be found and was loaded,
    /// or `false` otherwise.
    pub fn select_compatible_machine(&mut self) -> Result<bool> {
        if let Some(instructions) = self.animator_adapter.get_instructions() {
            if instructions.directives.targets.is_empty() {
                // No targets specified => no machine is compatible => cannot load any
                return Ok(false);
            }

            if self
                .current_machine
                .compatible_with(&instructions.directives.targets)
            {
                // compatible machine already loaded
                return Ok(true);
            }

            // Machine is not compatible or not set => load compatible machine

            // Find some compatible machine
            let compatible_machine = instructions
                .directives
                .targets
                .iter()
                .find(|id| self.machine_repository.has(id));
            if let Some(id) = compatible_machine {
                // compatible machine exists => load machine
                self.set_machine(id.clone().as_str())?;
                return Ok(true);
            }

            // failed to find a compatible machine
            return Ok(false);
        }
        // No instructions loaded =>cannot set any compatible machine
        Ok(false)
    }

    /// Sets the machine to the one with the specified `id`
    pub fn set_machine(&mut self, id: impl Into<String>) -> Result<()> {
        let id = id.into();
        let machine = self
            .machine_repository
            .get(&id)
            .ok_or(Error::Repository(
                RepositoryError::Search,
                ConfigFormat::Machine,
            ))?
            .map_err(|e| Error::Repository(RepositoryError::Open(e), ConfigFormat::Machine))?;
        self.set_loaded_machine(Some(id.clone()), machine);
        // keep machine in persistence
        self.persistence.machine = Some(IdOrManual::Id(id));
        Ok(())
    }

    /// Sets the current machine to `machine` with the optional `id`.
    /// If `id` is [None], the machine is assumed to be set manually.
    fn set_loaded_machine(&mut self, id: Option<impl Into<String>>, machine: MachineConfig) {
        let id = id.map(Into::into);
        self.current_machine = id
            .clone()
            .map(CurrentMachine::Id)
            .unwrap_or(CurrentMachine::Manual);
        self.animator_adapter.set_machine_config(machine);
    }

    /// Set the current machine to the one specified in `data`.
    pub fn set_machine_manually(&mut self, data: &[u8]) -> Result<()> {
        let machine = naviz_parser::config::lexer::lex(str::from_utf8(data).map_err(|e| {
            Error::FileOpen(InputType::Config(
                ConfigFormat::Machine,
                ConfigError::UTF8(e),
            ))
        })?)
        .map_err(|e| {
            Error::FileOpen(InputType::Config(
                ConfigFormat::Machine,
                ConfigError::Lex(e.into_inner()),
            ))
        })?;
        let machine = naviz_parser::config::parser::parse(&machine).map_err(|e| {
            Error::FileOpen(InputType::Config(
                ConfigFormat::Machine,
                ConfigError::Parse(e.into_inner()),
            ))
        })?;
        let machine: naviz_parser::config::generic::Config = machine.into();
        let machine: MachineConfig = machine.try_into().map_err(|e| {
            Error::FileOpen(InputType::Config(
                ConfigFormat::Machine,
                ConfigError::Convert(e),
            ))
        })?;
        self.set_loaded_machine(None::<String>, machine);
        // keep machine in persistence
        self.persistence.machine = Some(IdOrManual::Manual(data.into()));
        Ok(())
    }

    /// Sets the style to the one with the specified `id`
    pub fn set_style(&mut self, id: impl Into<String>) -> Result<()> {
        let id = id.into();
        let style = self
            .style_repository
            .get(&id)
            .ok_or(Error::Repository(
                RepositoryError::Search,
                ConfigFormat::Style,
            ))?
            .map_err(|e| Error::Repository(RepositoryError::Open(e), ConfigFormat::Style))?;
        self.set_loaded_style(Some(id.clone()), style);
        // keep style in persistence
        self.persistence.style = Some(IdOrManual::Id(id));
        Ok(())
    }

    /// Sets the current style to `style` with the optional `id`.
    /// If `id` is [None], the style is assumed to be set manually.
    fn set_loaded_style(&mut self, id: Option<impl Into<String>>, style: VisualConfig) {
        self.current_style_id = id.map(Into::into);
        self.animator_adapter.set_visual_config(style);
    }

    /// Set the current style to the one specified in `data`.
    pub fn set_style_manually(&mut self, data: &[u8]) -> Result<()> {
        let visual = naviz_parser::config::lexer::lex(str::from_utf8(data).map_err(|e| {
            Error::FileOpen(InputType::Config(ConfigFormat::Style, ConfigError::UTF8(e)))
        })?)
        .map_err(|e| {
            Error::FileOpen(InputType::Config(
                ConfigFormat::Style,
                ConfigError::Lex(e.into_inner()),
            ))
        })?;
        let visual = naviz_parser::config::parser::parse(&visual).map_err(|e| {
            Error::FileOpen(InputType::Config(
                ConfigFormat::Style,
                ConfigError::Parse(e.into_inner()),
            ))
        })?;
        let visual: naviz_parser::config::generic::Config = visual.into();
        let visual: VisualConfig = visual.try_into().map_err(|e| {
            Error::FileOpen(InputType::Config(
                ConfigFormat::Style,
                ConfigError::Convert(e),
            ))
        })?;
        self.set_loaded_style(None::<String>, visual);
        // keep style in persistence
        self.persistence.style = Some(IdOrManual::Manual(data.into()));
        Ok(())
    }

    /// Gets the ID of the currently selected machine
    /// or [None] if the current machine does not have an ID
    /// (i.e., is loaded from memory).
    pub fn get_current_machine_id(&self) -> Option<&str> {
        self.current_machine.id()
    }

    /// Gets the ID of the currently selected style
    /// or [None] if the current style does not have an ID
    /// (i.e., is loaded from memory).
    pub fn get_current_style_id(&self) -> Option<&str> {
        self.current_style_id.as_deref()
    }

    /// Gets the list of loaded machines.
    /// Will be sorted by name and compatible machines will be at the top.
    pub fn get_machines(&self) -> impl Iterator<Item = (&str, &str, bool)> {
        self.cache.machines()
    }

    /// Gets the list of loaded styles sorted by name.
    pub fn get_styles(&self) -> impl Iterator<Item = (&str, &str, bool)> {
        self.cache.styles()
    }

    /// Imports a machine to the repository by the specified `path`.
    /// The id will be set to the file name.
    /// See [Repository::import_machine_to_user_dir].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn import_machine(&mut self, path: &Path) -> Result<()> {
        self.machine_repository
            .import_machine_to_user_dir(path)
            .map_err(|e| Error::Repository(RepositoryError::Import(e), ConfigFormat::Machine))?;
        self.update_machines();
        Ok(())
    }

    /// Imports a style to the repository by the specified `path`.
    /// The id will be set to the file name.
    /// See [Repository::import_style_to_user_dir].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn import_style(&mut self, path: &Path) -> Result<()> {
        self.style_repository
            .import_style_to_user_dir(path)
            .map_err(|e| Error::Repository(RepositoryError::Import(e), ConfigFormat::Style))?;
        self.update_styles();
        Ok(())
    }

    /// Removes an imported machine from the repository.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn remove_machine(&mut self, id: &str) -> Result<()> {
        self.machine_repository
            .remove_from_user_dir(id)
            .map_err(|e| Error::Repository(RepositoryError::Remove(e), ConfigFormat::Machine))?;
        self.update_machines();
        Ok(())
    }

    /// Removes an imported style from the repository.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn remove_style(&mut self, id: &str) -> Result<()> {
        self.style_repository
            .remove_from_user_dir(id)
            .map_err(|e| Error::Repository(RepositoryError::Remove(e), ConfigFormat::Style))?;
        self.update_styles();
        Ok(())
    }

    /// Starts an export of the visualization to the specified `target`-path
    /// with the specified `resolution` and `fps`.
    /// Updates will be sent over the `progress`-channel.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export(
        &self,
        target: PathBuf,
        resolution: (u32, u32),
        fps: u32,
        progress: Sender<VideoProgress>,
    ) {
        if let Some(animator) = self.animator_adapter.animator() {
            let video = VideoExport::new(animator, resolution, fps);
            thread::spawn(move || {
                let mut video = futures::executor::block_on(video);
                video.export_video(&target, progress);
            });
        }
    }

    /// Updates the cached list of machines
    fn update_machines(&mut self) {
        self.cache.update_machines(
            &self.machine_repository,
            self.animator_adapter
                .get_instructions()
                .map(|i| i.directives.targets.as_slice())
                .unwrap_or(&[]),
        );
    }

    /// Updates the cached list of styles
    fn update_styles(&mut self) {
        self.cache.update_styles(&self.style_repository);
    }

    /// Returns whether a visualization is currently loaded:
    /// Some machine and style is selected and some instructions are loaded.
    pub fn visualization_loaded(&self) -> bool {
        self.animator_adapter.all_inputs_set()
    }

    /// Whether to force the zen-mode.
    /// See [Renderer::set_force_zen].
    pub fn set_force_zen(&mut self, force_zen: bool) {
        self.animator_adapter.set_force_zen(force_zen);
    }

    /// Gets whether the zen-mode is currently forced as set by [AppState::set_force_zen]
    pub fn get_force_zen(&mut self) -> bool {
        self.animator_adapter.get_force_zen()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Menu
        egui::TopBottomPanel::top("app_menu").show(ctx, |ui| {
            self.ui.menu_bar.draw(
                &mut self.state,
                &mut self.ui.errors,
                &self.future_helper,
                ctx,
                ui,
            )
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            let padding = ui.style().spacing.item_spacing.y;
            let (_, space) = ui.allocate_space(ui.available_size());
            let panel = AspectPanel {
                space,
                aspect_ratio: 16. / 9.,
                top: 0.,
                bottom: 20. + padding,
                left: 0.,
                right: 0.,
            };
            let animator_state = self.animator_adapter.get();
            panel.draw(
                ui,
                |ui| {
                    if let Some(animator_state) = animator_state {
                        WgpuCanvas::new(RendererAdapter::new(animator_state)).draw(ctx, ui);
                    } else {
                        // Animator is not ready (something missing) => empty canvas
                        WgpuCanvas::new(EmptyCanvas::new()).draw(ctx, ui);
                    }
                },
                |_| {},
                |_| {},
                |ui| {
                    ui.add_space(padding);
                    self.animator_adapter.draw_progress_bar(ui);
                },
                |_| {},
            );
        });

        self.ui.errors.draw(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.persistence);
    }
}

/// An adapter from [naviz_renderer] to [CallbackTrait].
///
/// Setup the renderer using [RendererAdapter::setup]
/// before drawing the renderer using the callback implementation.
#[derive(Clone)]
struct RendererAdapter {
    size: (f32, f32),
    /// The animator_state to render
    animator_state: AnimatorState,
}

impl RendererAdapter {
    /// Creates a [Renderer] and stores it in the egui [RenderState][eframe::egui_wgpu::RenderState].
    /// This created renderer will later be rendered from [RendererAdapter::paint].
    ///
    /// The renderer is stored in the renderer state
    /// in order for the graphics pipeline to have the same lifetime as the egui render pass.
    /// See [this section from the egui demo][https://github.com/emilk/egui/blob/0.28.1/crates/egui_demo_app/src/apps/custom3d_wgpu.rs#L83-L85]
    pub fn setup(cc: &eframe::CreationContext<'_>) {
        let wgpu_render_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("No wgpu render state found"); // Should not happen when `wgpu` is enabled

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(Renderer::new(
                &wgpu_render_state.device,
                &wgpu_render_state.queue,
                wgpu_render_state.target_format,
                &Config::example(),
                &State::example(),
                (1920, 1080), // Use some default resolution to create renderer, as the canvas-resolution is not yet known
            ));
    }

    /// Creates a new [RendererAdapter] from the passed [AnimatorState]
    pub fn new(animator_state: AnimatorState) -> Self {
        Self {
            animator_state,
            size: Default::default(),
        }
    }
}

impl CallbackTrait for RendererAdapter {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &eframe::egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(r) = callback_resources.get_mut::<Renderer>() {
            r.update_viewport(
                device,
                queue,
                (
                    (self.size.0 * screen_descriptor.pixels_per_point) as u32,
                    (self.size.1 * screen_descriptor.pixels_per_point) as u32,
                ),
            );
            self.animator_state
                .update(r, &mut (device, queue), device, queue);
        } else {
            error!("Failed to get renderer");
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &eframe::egui_wgpu::CallbackResources,
    ) {
        if let Some(r) = callback_resources.get::<Renderer>() {
            r.draw(render_pass);
        } else {
            error!("Failed to get renderer");
        }
    }
}

impl CanvasContent for RendererAdapter {
    fn background_color(&self) -> egui::Color32 {
        let [r, g, b, a] = self.animator_state.background();
        egui::Color32::from_rgba_unmultiplied(r, g, b, a)
    }

    fn target_size(&mut self, size: (f32, f32)) {
        self.size = size;
    }
}
