use naviz_state::{
    config::{
        Config, GridConfig, HPosition, LineConfig, MachineConfig, TrapConfig, VPosition, ZoneConfig,
    },
    state::State,
};
use wgpu::{Device, Queue, RenderPass};

use crate::{
    buffer_updater::BufferUpdater,
    component::drawable::Drawable,
    viewport::{Viewport, ViewportProjection, ViewportSource},
};

use super::{
    primitive::{
        circles::{CircleSpec, Circles},
        lines::{LineSpec, Lines},
        rectangles::{RectangleSpec, Rectangles},
        text::{Alignment, HAlignment, Text, TextSpec, VAlignment},
    },
    updatable::Updatable,
    ComponentInit,
};

/// A component to draw the machine background:
/// - Background grid and coordinate legend
/// - Static traps
/// - Zones
pub struct Machine {
    viewport: Viewport,
    background_grid: Lines,
    static_traps: Circles,
    coordinate_legend: Text,
    zones: Rectangles,
}

/// Padding between the grid and the legend (numbers and labels)
const LABEL_PADDING: f32 = 12.;

impl Machine {
    pub fn new(
        ComponentInit {
            device,
            queue,
            format,
            globals,
            shader_composer,
            config,
            state: _,
            viewport_projection,
            screen_resolution,
        }: ComponentInit,
    ) -> Self {
        let mut text_buffer = Vec::new();
        let MachineSpec {
            lines,
            traps,
            labels,
            zones,
        } = get_specs(config, viewport_projection, &mut text_buffer);
        let viewport = Viewport::new(viewport_projection, device);

        Self {
            background_grid: Lines::new(
                device,
                format,
                globals,
                &viewport,
                shader_composer,
                &lines,
            ),
            static_traps: Circles::new(device, format, globals, &viewport, shader_composer, &traps),
            coordinate_legend: Text::new(device, queue, format, labels, screen_resolution),
            zones: Rectangles::new(device, format, globals, &viewport, shader_composer, zones),
            viewport,
        }
    }

    /// Updates the viewport resolution of this [Machine]
    pub fn update_viewport(
        &mut self,
        device: &Device,
        queue: &Queue,
        screen_resolution: (u32, u32),
    ) {
        self.coordinate_legend
            .update_viewport((device, queue), screen_resolution);
    }
}

impl Drawable for Machine {
    /// Draws this [Machine].
    ///
    /// May overwrite bind groups.
    /// If `REBIND` is `true`, will call the passed `rebind`-function to rebind groups.
    fn draw<const REBIND: bool>(
        &self,
        render_pass: &mut RenderPass<'_>,
        rebind: impl Fn(&mut RenderPass),
    ) {
        self.viewport.bind(render_pass);
        self.background_grid.draw(render_pass);
        self.static_traps.draw(render_pass);
        self.zones.draw(render_pass);
        self.coordinate_legend.draw::<REBIND>(render_pass, rebind);
    }
}

impl Updatable for Machine {
    fn update(
        &mut self,
        _updater: &mut impl BufferUpdater,
        _device: &Device,
        _queue: &Queue,
        _config: &Config,
        _state: &State,
    ) {
        // Nothing depends on state
    }

    fn update_full(
        &mut self,
        updater: &mut impl BufferUpdater,
        device: &Device,
        queue: &Queue,
        config: &Config,
        _state: &State,
        viewport_projection: ViewportProjection,
    ) {
        self.viewport.update(updater, viewport_projection);
        let mut text_buffer = Vec::new();
        let MachineSpec {
            lines,
            traps,
            labels,
            zones,
        } = get_specs(config, viewport_projection, &mut text_buffer);
        self.background_grid.update(updater, &lines);
        self.static_traps.update(updater, &traps);
        self.coordinate_legend.update((device, queue), labels);
        self.zones.update(updater, zones);
    }
}

/// Creates an iterator that yields [f32]s from `start` to `end` (both included) in steps of `step`
fn range_f32(start: f32, end: f32, step: f32) -> impl Iterator<Item = f32> {
    let len = end - start;
    let steps = (len / step) as u64;
    (0..=steps).map(move |i| start + (i as f32 * step))
}

#[derive(Clone, Debug)]
struct MachineSpec<'a, TextIterator: IntoIterator<Item = (&'a str, (f32, f32), Alignment)>> {
    /// The coordinate grid
    lines: Vec<LineSpec>,
    /// Circles to draw to represent the traps
    traps: Vec<CircleSpec>,
    /// Axis labels (including the numbers)
    labels: TextSpec<'a, TextIterator>,
    /// Rectangles to draw for the zones
    zones: Vec<RectangleSpec>,
}

/// Gets the specs for [Machine] from the passed [State] and [Config].
fn get_specs<'a>(
    config: &'a Config,
    viewport_projection: ViewportProjection,
    text_buffer: &'a mut Vec<(String, (f32, f32), Alignment)>,
) -> MachineSpec<'a, impl IntoIterator<Item = (&'a str, (f32, f32), Alignment)>> {
    let MachineConfig { grid, traps, zones } = &config.machine;
    let viewport_source = viewport_projection.source;

    let lines = get_grid_lines_specs(grid, viewport_source);

    let traps = get_trap_specs(traps);

    build_number_labels(grid, text_buffer, viewport_source);
    let texts = add_grid_legend(
        grid,
        viewport_source,
        text_buffer.iter().map(|(t, p, a)| (t.as_str(), *p, *a)),
    );

    let zones = get_zone_specs(zones);

    MachineSpec {
        lines,
        traps,
        labels: TextSpec {
            viewport_projection,
            font_size: grid.legend.font.size,
            font_family: &grid.legend.font.family,
            texts,
            color: grid.legend.font.color,
        },
        zones,
    }
}

/// Create the [LineSpec]s for the grid fit to the [ViewportSource].
#[inline]
fn get_grid_lines_specs(grid: &GridConfig, vp: ViewportSource) -> Vec<LineSpec> {
    if !grid.display_ticks {
        // Don't display any ticks
        return Vec::new();
    }

    // The viewport-edges clamped to the grid/legend steps
    let vp_left_grid = clamp_to(vp.left(), grid.step.0);
    let vp_top_grid = clamp_to(vp.top(), grid.step.1);

    // create LineSpecs; first x, then y
    range_f32(vp_left_grid, vp.right(), grid.step.0)
        .map(|x| LineSpec {
            start: [x, vp.top()],
            end: [x, vp.bottom()],
            color: grid.line.color,
            width: grid.line.width,
            segment_length: grid.line.segment_length,
            duty: grid.line.duty,
        })
        .chain(
            range_f32(vp_top_grid, vp.bottom(), grid.step.1).map(|y| LineSpec {
                start: [vp.left(), y],
                end: [vp.right(), y],
                color: grid.line.color,
                width: grid.line.width,
                segment_length: grid.line.segment_length,
                duty: grid.line.duty,
            }),
        )
        .collect()
}

/// Create the [CircleSpec]s for the static traps
fn get_trap_specs(traps: &TrapConfig) -> Vec<CircleSpec> {
    traps
        .positions
        .iter()
        .map(|(x, y)| CircleSpec {
            center: [*x, *y],
            radius: traps.radius,
            radius_inner: traps.radius - traps.line_width,
            color: traps.color,
        })
        .collect()
}

/// Fill the `text_buffer` with the strings for the legend numbers in x- and y-direction.
fn build_number_labels(
    grid: &GridConfig,
    text_buffer: &mut Vec<(String, (f32, f32), Alignment)>,
    vp: ViewportSource,
) {
    if !grid.legend.display_numbers {
        // Don't display number labels
        *text_buffer = Vec::new();
        return;
    }

    // The viewport-edges clamped to the grid/legend steps
    let vp_left_legend = clamp_to(vp.left(), grid.legend.step.0);
    let vp_top_legend = clamp_to(vp.top(), grid.legend.step.1);

    *text_buffer = range_f32(vp_left_legend, vp.right(), grid.legend.step.0)
        .map(|x| {
            (
                format!("{x}"),
                (
                    x,
                    grid.legend
                        .position
                        .0
                        .get(vp.top() - LABEL_PADDING, vp.bottom() + LABEL_PADDING),
                ),
                Alignment(HAlignment::Center, get_v_alignment(grid.legend.position.0)),
            )
        })
        .chain(
            range_f32(vp_top_legend, vp.bottom(), grid.legend.step.1).map(|y| {
                (
                    format!("{y}"),
                    (
                        grid.legend
                            .position
                            .1
                            .get(vp.left() - LABEL_PADDING, vp.right() + LABEL_PADDING),
                        y,
                    ),
                    Alignment(get_h_alignment(grid.legend.position.1), VAlignment::Center),
                )
            }),
        )
        .collect();
}

/// Add the grid legends to the `texts`
#[inline]
fn add_grid_legend<'a>(
    grid: &'a GridConfig,
    vp: ViewportSource,
    texts: impl IntoIterator<Item = (&'a str, (f32, f32), Alignment)>,
) -> impl Iterator<Item = (&'a str, (f32, f32), Alignment)> {
    let texts = texts.into_iter();

    if !grid.legend.display_labels {
        // Don't display any labels
        // Still need to chain (empty) vector to produce same output type
        return texts.chain(Vec::new());
    }

    // Add axis labels
    texts.chain(vec![
        (
            grid.legend.labels.0.as_str(),
            (
                grid.legend
                    .position
                    .1
                    .inverse()
                    .get(vp.left() - LABEL_PADDING, vp.right() + LABEL_PADDING),
                grid.legend.position.0.get(vp.top(), vp.bottom()),
            ),
            Alignment(
                get_h_alignment(grid.legend.position.1.inverse()),
                VAlignment::Center,
            ),
        ),
        (
            grid.legend.labels.1.as_str(),
            (
                grid.legend.position.1.get(vp.left(), vp.right()),
                grid.legend
                    .position
                    .0
                    .inverse()
                    .get(vp.top() - LABEL_PADDING, vp.bottom() + LABEL_PADDING),
            ),
            Alignment(
                HAlignment::Center,
                get_v_alignment(grid.legend.position.0.inverse()),
            ),
        ),
    ])
}

/// Build the [RectangleSpec]s for the zones
fn get_zone_specs(zones: &[ZoneConfig]) -> Vec<RectangleSpec> {
    zones
        .iter()
        .copied()
        .map(
            |ZoneConfig {
                 start,
                 size,
                 line:
                     LineConfig {
                         width,
                         segment_length,
                         duty,
                         color,
                     },
             }| RectangleSpec {
                start: start.into(),
                size: size.into(),
                color,
                width,
                duty,
                segment_length,
            },
        )
        .collect()
}

/// Gets the [VAlignment] based on the passed [VPosition]
#[inline]
fn get_v_alignment(p: VPosition) -> VAlignment {
    p.get(VAlignment::Bottom, VAlignment::Top)
}

/// Gets the [HAlignment] based on the passed [HPosition]
#[inline]
fn get_h_alignment(p: HPosition) -> HAlignment {
    p.get(HAlignment::Right, HAlignment::Left)
}

/// Clamps `num` to be in steps of `step`.
/// Will always round down.
#[inline]
fn clamp_to(num: f32, step: f32) -> f32 {
    num - num % step
}

#[cfg(test)]
mod test {
    use crate::viewport::ViewportTarget;

    use super::*;

    /// Identity-Viewport, which can be used for testing
    fn viewport_identity() -> ViewportProjection {
        ViewportProjection {
            source: ViewportSource {
                x: 0.,
                y: 0.,
                width: 1.,
                height: 1.,
            },
            target: ViewportTarget {
                x: 0.,
                y: 0.,
                width: 1.,
                height: 1.,
            },
        }
    }

    /// Sanity-Check: example config should produce plausible results.
    /// Note: This does not check the exact output of the specs, only plausibility of output counts.
    #[test]
    fn example_specs() {
        let config = Config::example();
        let viewport_projection = viewport_identity();
        let mut text_buffer = Vec::new();
        let specs = get_specs(&config, viewport_projection, &mut text_buffer);

        assert!(!specs.lines.is_empty(), "Did not produce any lines");
        assert_eq!(
            specs.traps.len(),
            config.machine.traps.positions.len(),
            "Did not produce same number of traps as input"
        );
        assert_eq!(
            specs.zones.len(),
            config.machine.zones.len(),
            "Did not produce same number of zones as input"
        );
        assert!(
            specs.labels.texts.into_iter().next().is_some(),
            "Did not produce any text specs"
        );
    }

    /// Sanity-Check: example config with all `display`-options set to `false` should produce plausible results.
    /// Note: This does not check the exact output of the specs, only plausibility of output counts.
    #[test]
    fn example_no_display_specs() {
        let mut config = Config::example();
        config.machine.grid.display_ticks = false;
        config.machine.grid.legend.display_labels = false;
        config.machine.grid.legend.display_numbers = false;
        config.time.display = false;

        let viewport_projection = viewport_identity();
        let mut text_buffer = Vec::new();
        let specs = get_specs(&config, viewport_projection, &mut text_buffer);

        assert!(specs.lines.is_empty(), "Should not produce any lines");
        assert_eq!(
            specs.traps.len(),
            config.machine.traps.positions.len(),
            "Did not produce same number of traps as input"
        );
        assert_eq!(
            specs.zones.len(),
            config.machine.zones.len(),
            "Did not produce same number of zones as input"
        );
        assert!(
            specs.labels.texts.into_iter().next().is_none(),
            "Should not produce any text specs"
        );
    }
}
