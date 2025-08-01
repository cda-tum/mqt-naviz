use egui::{style::HandleShape, Button, DragValue, Slider, Ui};
use egui_extras::{Size, StripBuilder};

/// Height of the progress bar
const BAR_HEIGHT: f32 = 20.;
/// Width of the play/pause button
const PLAY_PAUSE_WIDTH: f32 = BAR_HEIGHT;
/// Width of the speed field
const SPEED_WIDTH: f32 = BAR_HEIGHT * 2.;

/// Play icon (unicode)
const PLAY_ICON: &str = "\u{25B6}";
/// Pause icon (unicode)
const PAUSE_ICON: &str = "\u{23F8}";
/// Replay icon (unicode)
const REPLAY_ICON: &str = "\u{27F2}";

/// Maximum speed
const MAX_SPEED: f64 = 5.;

/// Prefix for speed (`x <speed>`):
/// Thin Space, Multiplication Sign
const TIMES_PREFIX: &str = "\u{00D7}\u{2009}";

/// The progress bar that displays the animation progress,
/// allows scrubbing though the animation,
/// pausing, and changing the playback speed.
///
/// This handles all the time-updates.
/// Consumers can get the current animation-time using [ProgressBar::animation_time].
pub struct ProgressBar {
    /// The current time in the animation
    animation_time: f64,
    // /The duration of the animation
    duration: f64,
    /// The current playback-speed
    /// (i.e., how many animation-time-units will be advanced per real-time-unit)
    speed: f64,
    /// Whether playback is currently paused
    paused: bool,
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::new(0.)
    }
}

impl ProgressBar {
    /// Creates a new progress-bar with the specified `duration`
    pub fn new(duration: f64) -> Self {
        Self::new_with_speed(duration, 1.)
    }

    /// Creates a new progress-bar with the specified `duration` and `speed`
    pub fn new_with_speed(duration: f64, speed: f64) -> Self {
        Self {
            animation_time: 0.,
            speed,
            duration,
            paused: false,
        }
    }

    /// Gets the currently set speed
    pub fn get_speed(&self) -> f64 {
        self.speed
    }

    /// Updates the `animation_time` respecting `paused` and `speed`.
    fn update_time(&mut self, delta: f32) {
        if !self.paused {
            self.animation_time += self.speed * delta as f64;

            if self.is_end() {
                // pause on end
                self.paused = true;
            }
        }
    }

    /// Returns `true` when the playback has reached the end
    fn is_end(&self) -> bool {
        self.animation_time >= self.duration
    }

    /// Draws the play/pause button
    fn draw_pause(&mut self, ui: &mut Ui) {
        let icon = match (self.paused, self.is_end()) {
            (true, false) => PLAY_ICON,
            (true, true) => REPLAY_ICON,
            (false, _) => PAUSE_ICON,
        };
        if ui
            .add_sized([PLAY_PAUSE_WIDTH, BAR_HEIGHT], Button::new(icon))
            .clicked()
        {
            self.paused = !self.paused;

            if self.is_end() {
                // replay when pressing play on end
                self.animation_time = 0.;
            }
        }
    }

    /// Draws the progress-bar that allows scrubbing
    fn draw_progress(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // manually set slider width: https://github.com/emilk/egui/issues/462
            ui.spacing_mut().slider_width = ui.available_width();
            ui.add_sized(
                [ui.available_width(), BAR_HEIGHT],
                Slider::new(&mut self.animation_time, 0. ..=self.duration)
                    .handle_shape(HandleShape::Rect { aspect_ratio: 0.5 })
                    .show_value(false),
            );
        });
    }

    /// Draws the current playback-speed as a changeable field
    fn draw_speed(&mut self, ui: &mut Ui) {
        ui.add_sized(
            [SPEED_WIDTH, BAR_HEIGHT],
            DragValue::new(&mut self.speed)
                .range(0. ..=MAX_SPEED)
                .prefix(TIMES_PREFIX)
                .speed(0.01),
        );
    }

    /// Gets the current time in the animation.
    pub fn animation_time(&self) -> f64 {
        self.animation_time
    }

    /// Draws this full [ProgressBar] and updates the times.
    pub fn draw(&mut self, ui: &mut Ui) {
        self.update_time(ui.input(|i| i.unstable_dt));

        StripBuilder::new(ui)
            .size(Size::exact(PLAY_PAUSE_WIDTH))
            .size(Size::remainder())
            .size(Size::initial(SPEED_WIDTH))
            .horizontal(|mut strip| {
                strip.cell(|ui| self.draw_pause(ui));
                strip.cell(|ui| self.draw_progress(ui));
                strip.cell(|ui| self.draw_speed(ui));
            });
    }
}
