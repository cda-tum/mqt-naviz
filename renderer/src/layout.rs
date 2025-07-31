use crate::viewport::{ViewportProjection, ViewportSource, ViewportTarget};

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub content: ViewportProjection,
    pub legend: Option<ViewportProjection>,
    pub time: Option<ViewportProjection>,
}

impl Layout {
    /// Padding around elements
    const PADDING: f32 = 0.01;
    /// Padding between content and legend
    const PADDING_BETWEEN: f32 = 0.1;
    /// Target area for the content
    const CONTENT_TARGET: ViewportTarget = ViewportTarget {
        x: -1. + Self::PADDING,
        y: -1. + Self::PADDING,
        width: 0.8 * 2. - 2. * Self::PADDING,
        height: 2.0 - 2. * Self::PADDING,
    };
    /// Target area for the legend
    const LEGEND_TARGET: ViewportTarget = ViewportTarget {
        x: 0.8 * 2. - 1. + Self::PADDING + Self::PADDING_BETWEEN,
        y: -0.9 + Self::PADDING,
        width: 0.2 * 2. - 2. * Self::PADDING,
        height: 1.9 - 2. * Self::PADDING,
    };
    const TIME_TARGET: ViewportTarget = ViewportTarget {
        x: 0.8 * 2. - 1. + Self::PADDING + Self::PADDING_BETWEEN,
        y: -1. + Self::PADDING,
        width: 0.2 * 2. - 2. * Self::PADDING,
        height: 0.1 - 2. * Self::PADDING,
    };

    /// Creates a new [Layout] for `content`, `legend`, and `time`
    /// based on the passed `screen_size`, `content` size, and `legend_height`.
    /// `content` will be padded by `content_padding_y` in y-direction in its viewport-space
    /// (but its [ViewportSource] will stay the same).
    /// Will put the content on the left part of the screen (taking roughly 80%)
    /// and the legend on the right (taking roughly 20%).
    pub fn new_full(
        screen_size: (u32, u32),
        content: ViewportSource,
        content_padding_y: f32,
        legend_height: f32,
        time_height: f32,
    ) -> Self {
        // layout content
        let content = fit_and_center(
            content,
            Self::CONTENT_TARGET,
            screen_size,
            content_padding_y,
        );

        // grow legend target by gobbling free space on the left
        let legend_target =
            gobble_space_left_until(Self::LEGEND_TARGET, content.target, 0. + Self::PADDING, 0.4);

        // calculate appropriate legend width
        let legend_width = calculate_width(
            legend_height,
            screen_size,
            (legend_target.width, legend_target.height),
        );

        // layout legend
        let legend = ViewportProjection {
            source: ViewportSource {
                x: 0.,
                y: 0.,
                width: legend_width,
                height: legend_height,
            },
            target: legend_target,
        };

        // grow time target by gobbling free space on the left
        let time_target =
            gobble_space_left_until(Self::TIME_TARGET, content.target, 0. + Self::PADDING, 0.4);

        // calculate appropriate time width
        let time_width = calculate_width(
            time_height,
            screen_size,
            (time_target.width, time_target.height),
        );

        // layout time
        let time = ViewportProjection {
            source: ViewportSource {
                x: 0.,
                y: 0.,
                width: time_width,
                height: time_height,
            },
            target: time_target,
        };

        Self {
            content,
            legend: Some(legend),
            time: Some(time),
        }
    }

    /// Creates a new [Layout] which only contains the `content`-viewport.
    pub fn new_content_only(
        screen_size: (u32, u32),
        content: ViewportSource,
        content_padding_y: f32,
    ) -> Self {
        Self {
            content: fit_and_center(content, Default::default(), screen_size, content_padding_y),
            legend: None,
            time: None,
        }
    }
}

/// Gets a [ViewportProjection] that fits and centers the [ViewportSource] into the [ViewportTarget].
/// Will also shrink the target by `padding_y` in source coordinates.
fn fit_and_center(
    source: ViewportSource,
    target: ViewportTarget,
    screen_size: (u32, u32),
    padding_y: f32,
) -> ViewportProjection {
    let size = get_size_keep_aspect(
        screen_size,
        (target.width, target.height),
        (source.width, source.height),
    );
    let projection = ViewportProjection {
        source,
        target: center_in(target, size),
    };
    shrink_target_by_source_padding(projection, padding_y)
}

/// Gets the size of a possible viewport which:
/// - Has the correct aspect ratio to not distort the content
/// - Fills `max_size` as much as possible
/// - Does not extend beyond `max_size`
fn get_size_keep_aspect(
    screen_size: (u32, u32),
    max_size: (f32, f32),
    viewport_size: (f32, f32),
) -> (f32, f32) {
    // Get the size (in wgpu screen units) to render in original size
    let x = viewport_size.0 / screen_size.0 as f32 * 2.;
    let y = viewport_size.1 / screen_size.1 as f32 * 2.;

    // Scale to fit width
    let scale_width_fit = max_size.0 / x;
    let x_width_fit = x * scale_width_fit;
    let y_width_fit = y * scale_width_fit;

    if y_width_fit <= max_size.1 {
        // Fits inside
        (x_width_fit, y_width_fit)
    } else {
        // Scale to fit height instead
        let scale_height_fit = max_size.1 / y;
        let x_height_fit = x * scale_height_fit;
        let y_height_fit = y * scale_height_fit;
        (x_height_fit, y_height_fit)
    }
}

/// Creates a new [ViewportTarget] of size `size` and centers it in the passed `bounds`.
fn center_in(bounds: ViewportTarget, size: (f32, f32)) -> ViewportTarget {
    let ml = (bounds.width - size.0) / 2.; // Margin left
    let mb = (bounds.height - size.1) / 2.; // Margin bottom
    ViewportTarget {
        x: bounds.x + ml,
        y: bounds.y + mb,
        width: size.0,
        height: size.1,
    }
}

/// Shrinks the target by the specified `padding_y` in source-space.
/// Will keep aspect ratio.
fn shrink_target_by_source_padding(
    ViewportProjection { source, target }: ViewportProjection,
    padding_y: f32,
) -> ViewportProjection {
    let px = padding_y / source.width * target.width;
    let py = px / target.width * target.height;

    ViewportProjection {
        source,
        target: ViewportTarget {
            x: target.x + px / 2.,
            y: target.y + py / 2.,
            width: target.width - px,
            height: target.height - py,
        },
    }
}

/// Gobbles space left of `target` until the bounds of `left` or `min_x` is reached.
/// Will only expand `ratio` of the gobbled space.
fn gobble_space_left_until(
    target: ViewportTarget,
    left: ViewportTarget,
    min_x: f32,
    ratio: f32,
) -> ViewportTarget {
    let left_x = left.x + left.width;
    let possible_x = ratio * left_x + (1. - ratio) * target.x;
    let new_x = possible_x.max(min_x);
    let delta_x = target.x - new_x;

    if new_x < target.x {
        // Only gobble to the left (don't move to the right instead)
        ViewportTarget {
            x: new_x,
            y: target.y,
            width: target.width + delta_x,
            height: target.height,
        }
    } else {
        target
    }
}

/// Calculates the source width based on the passed source `height`, `screen_size` and viewport `target` size
fn calculate_width(
    height: f32,
    screen_size: (u32, u32),
    (target_width, target_height): (f32, f32),
) -> f32 {
    height / screen_size.1 as f32 / target_height * target_width * screen_size.0 as f32
}
