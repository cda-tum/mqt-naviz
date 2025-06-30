use std::sync::Arc;

use egui::Ui;
use naviz_animator::animator::Animator;
use naviz_parser::{
    config::{machine::MachineConfig, visual::VisualConfig},
    input::concrete::Instructions,
};
use naviz_renderer::{buffer_updater::BufferUpdater, renderer::Renderer};
use naviz_state::{config::Config, state::State};
use wgpu::{Device, Queue};

use crate::progress_bar::ProgressBar;

#[derive(Default)]
pub struct AnimatorAdapter {
    update_full: bool,
    progress_bar: ProgressBar,

    animator: Option<Animator>,
    machine: Option<MachineConfig>,
    visual: Option<VisualConfig>,
    instructions: Option<Instructions>,
}

/// The animator state at a current time (as set by [AnimatorAdapter::set_time]),
/// returned from [AnimatorAdapter::get].
#[derive(Clone)]
pub struct AnimatorState {
    /// Is a full update required?
    /// (see [Updatable::update_full][naviz_renderer::component::updatable::Updatable::update_full])
    update_full: bool,
    /// The current state
    state: State,
    /// The current config
    config: Arc<Config>,
    /// The background color
    background: [u8; 4],
}

impl AnimatorState {
    /// Updates the passed [Renderer] to represent the current animator-state
    pub fn update(
        &self,
        renderer: &mut Renderer,
        updater: &mut impl BufferUpdater,
        device: &Device,
        queue: &Queue,
    ) {
        let config = &self.config;
        let state = &self.state;
        if self.update_full {
            renderer.update_full(updater, device, queue, config, state);
        } else {
            renderer.update(updater, device, queue, config, state);
        }
    }

    /// Gets the background-color of this [AnimatorState]
    pub fn background(&self) -> [u8; 4] {
        self.background
    }
}

impl AnimatorAdapter {
    /// Sets the machine config
    pub fn set_machine_config(&mut self, config: MachineConfig) {
        self.machine = Some(config);
        self.recreate_animator(false);
    }

    /// Sets the visual config
    pub fn set_visual_config(&mut self, config: VisualConfig) {
        self.visual = Some(config);
        self.recreate_animator(false);
    }

    /// Sets the instructions
    pub fn set_instructions(&mut self, instructions: Instructions) {
        self.instructions = Some(instructions);
        self.recreate_animator(true);
    }

    /// Gets the instructions
    pub fn get_instructions(&self) -> Option<&Instructions> {
        self.instructions.as_ref()
    }

    /// Recreates the animator.
    /// Call this when new machine, visual, instructions are set.
    ///
    /// Set `reset_time` to `true` when the duration needs to be updated.
    /// This also sets the time back to `0`.
    /// Time will always be reset when creating the animator for the first time.
    fn recreate_animator(&mut self, reset_time: bool) {
        if let (Some(machine), Some(visual), Some(instructions)) =
            (&self.machine, &self.visual, &self.instructions)
        {
            let animator = Animator::new(machine.clone(), visual.clone(), instructions.clone());
            self.update_full = true;
            if reset_time || self.animator.is_none() {
                self.progress_bar = ProgressBar::new(animator.duration().try_into().unwrap());
            }
            self.animator = Some(animator);
        }
    }

    /// Gets an [AnimatorState] from this [AnimatorAdapter],
    /// or [None] if not enough inputs were set.
    ///
    /// Will update internal state
    /// like resetting the `update_full`-state such that full updates only happen if required
    /// (i.e., whenever new instructions/configs are loaded).
    /// When the result of this function is unused,
    /// this may lead to unexpected behavior.
    /// It is also expected that a single [AnimatorAdapter] only serves one [Animator]
    /// (or any other [Animator]s use the same states instead of calling [AnimatorAdapter::get]
    /// for each instance),
    /// as otherwise not all [Animator]s may get updated fully.
    ///
    /// If the [AnimatorState] should be gotten without changing any internal state,
    /// use [AnimatorAdapter::peek] instead.
    #[must_use]
    pub fn get(&mut self) -> Option<AnimatorState> {
        let state = self.peek();
        self.update_full = false;
        state
    }

    /// Gets an [AnimatorState] from this [AnimatorAdapter]
    /// or [None] if not enough inputs were set.
    ///
    /// Will not do any internal state updates.
    /// If piping this output directly into an [Animator],
    /// [AnimatorAdapter::get] should probably used instead
    /// as this updates internal state to prevent unnecessary updates.
    pub fn peek(&self) -> Option<AnimatorState> {
        self.animator.as_ref().map(|animator| AnimatorState {
            update_full: self.update_full,
            config: animator.config(),
            state: animator.state((self.progress_bar.animation_time() as f32).into()),
            background: animator.background(),
        })
    }

    /// Creates an [Animator] from this [AnimatorAdapter],
    /// or [None] if not enough inputs were set.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn animator(&self) -> Option<Animator> {
        if let (Some(machine), Some(visual), Some(instructions)) =
            (&self.machine, &self.visual, &self.instructions)
        {
            Some(Animator::new(
                machine.clone(),
                visual.clone(),
                instructions.clone(),
            ))
        } else {
            None
        }
    }

    /// Checks if all three inputs
    /// ([machine][AnimatorAdapter::set_machine_config],
    /// [visual][AnimatorAdapter::set_visual_config],
    /// [instructions][AnimatorAdapter::set_instructions])
    /// are set
    pub fn all_inputs_set(&self) -> bool {
        self.machine.is_some() && self.visual.is_some() && self.instructions.is_some()
    }

    /// Draws the progress-bar of this [AnimatorAdapter] using [ProgressBar::draw].
    /// Will also update the animation-time.
    pub fn draw_progress_bar(&mut self, ui: &mut Ui) {
        self.progress_bar.draw(ui);
    }
}
