use naviz_state::{
    config::{Config, HPosition, LineConfig, MachineConfig, VPosition, ZoneConfig},
    state::State,
};
use wgpu::{Device, Queue, RenderPass};

use crate::{
    buffer_updater::BufferUpdater,
    viewport::{Viewport, ViewportProjection},
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

    /// Draws this [Machine].
    ///
    /// May overwrite bind groups.
    /// If `REBIND` is `true`, will call the passed `rebind`-function to rebind groups.
    pub fn draw<const REBIND: bool>(
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

    // The viewport edges
    let vp_left = viewport_projection.source.left();
    let vp_right = viewport_projection.source.right();
    let vp_top = viewport_projection.source.top();
    let vp_bottom = viewport_projection.source.bottom();
    // The viewport-edges clamped to the grid/legend steps
    let vp_left_grid = vp_left - vp_left % grid.step.0;
    let vp_top_grid = vp_top - vp_top % grid.step.1;
    let vp_left_legend = vp_left - vp_left % grid.legend.step.0;
    let vp_top_legend = vp_top - vp_top % grid.legend.step.1;

    // Create the LineSpecs for the grid; first x, then y
    let lines: Vec<_> = range_f32(vp_left_grid, vp_right, grid.step.0)
        .map(|x| LineSpec {
            start: [x, vp_top],
            end: [x, vp_bottom],
            color: grid.line.color,
            width: grid.line.width,
            segment_length: grid.line.segment_length,
            duty: grid.line.duty,
        })
        .chain(
            range_f32(vp_top_grid, vp_bottom, grid.step.1).map(|y| LineSpec {
                start: [vp_left, y],
                end: [vp_right, y],
                color: grid.line.color,
                width: grid.line.width,
                segment_length: grid.line.segment_length,
                duty: grid.line.duty,
            }),
        )
        .collect();

    // Create the CircleSpecs for the static traps
    let traps: Vec<_> = traps
        .positions
        .iter()
        .map(|(x, y)| CircleSpec {
            center: [*x, *y],
            radius: traps.radius,
            radius_inner: traps.radius - traps.line_width,
            color: traps.color,
        })
        .collect();

    // Create the text specs for the numbers numbers (x then y)
    // First create strings, then convert to text spec
    *text_buffer = range_f32(vp_left_legend, vp_right, grid.legend.step.0)
        .map(|x| {
            (
                format!("{x}"),
                (
                    x,
                    grid.legend
                        .position
                        .0
                        .get(vp_top - LABEL_PADDING, vp_bottom + LABEL_PADDING),
                ),
                Alignment(HAlignment::Center, get_v_alignment(grid.legend.position.0)),
            )
        })
        .chain(
            range_f32(vp_top_legend, vp_bottom, grid.legend.step.1).map(|y| {
                (
                    format!("{y}"),
                    (
                        grid.legend
                            .position
                            .1
                            .get(vp_left - LABEL_PADDING, vp_right + LABEL_PADDING),
                        y,
                    ),
                    Alignment(get_h_alignment(grid.legend.position.1), VAlignment::Center),
                )
            }),
        )
        .collect();
    let texts = text_buffer.iter().map(|(t, p, a)| (t.as_str(), *p, *a));
    // Add axis labels
    let texts = texts.chain([
        (
            grid.legend.labels.0.as_str(),
            (
                grid.legend
                    .position
                    .1
                    .inverse()
                    .get(vp_left - LABEL_PADDING, vp_right + LABEL_PADDING),
                grid.legend.position.0.get(vp_top, vp_bottom),
            ),
            Alignment(
                get_h_alignment(grid.legend.position.1.inverse()),
                VAlignment::Center,
            ),
        ),
        (
            grid.legend.labels.1.as_str(),
            (
                grid.legend.position.1.get(vp_left, vp_right),
                grid.legend
                    .position
                    .0
                    .inverse()
                    .get(vp_top - LABEL_PADDING, vp_bottom + LABEL_PADDING),
            ),
            Alignment(
                HAlignment::Center,
                get_v_alignment(grid.legend.position.0.inverse()),
            ),
        ),
    ]);

    let zones: Vec<_> = zones
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
        .collect();

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
