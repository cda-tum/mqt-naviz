use glam::{Mat4, Vec3};
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer,
};
use log::warn;
use wgpu::{Device, MultisampleState, Queue, RenderPass, TextureFormat};

use crate::viewport::ViewportProjection;

#[derive(Clone, Copy, Default, Debug)]
pub enum HAlignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Copy, Default, Debug)]
pub enum VAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Alignment(pub HAlignment, pub VAlignment);

#[derive(Clone, Copy, Debug)]
pub struct TextSpec<'a, TextIterator: IntoIterator<Item = (&'a str, (f32, f32), Alignment)>> {
    /// The viewport projection to render in.
    /// Does not use viewport (or globals) directly,
    /// but renders using [glyphon].
    pub viewport_projection: ViewportProjection,
    /// The size of the font
    pub font_size: f32,
    /// The font to use
    pub font_family: &'a str,
    /// The texts to render: (`text`, `position`, `alignment`)
    pub texts: TextIterator,
    /// The color of the texts to render
    pub color: [u8; 4],
}

/// The cache containing the pre-baked data to bake.
///
/// Create using [BakeCache::create],
/// fully bake using [Text::bake].
struct BakeCache {
    /// The text-buffers
    text_buffers: Vec<(Buffer, (f32, f32), Alignment)>,
    /// The text color
    color: Color,
    /// The viewport projection to render in
    viewport_projection: ViewportProjection,
    /// The last screen resolution
    screen_resolution: (u32, u32),
}

/// A component that renders text
pub struct Text {
    atlas: TextAtlas,
    glyphon_viewport: glyphon::Viewport,
    text_renderer: TextRenderer,
    font_system: FontSystem,
    swash_cache: SwashCache,
    bake_cache: BakeCache,
}

impl Text {
    pub fn new<'a, TextIterator: IntoIterator<Item = (&'a str, (f32, f32), Alignment)>>(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        spec: TextSpec<'a, TextIterator>,
        screen_resolution: (u32, u32),
    ) -> Self {
        let mut font_system = FontSystem::new();
        // Load a default font
        // Used when system-fonts cannot be loaded (e.g., on web)
        font_system
            .db_mut()
            .load_font_data(include_bytes!(env!("DEFAULT_FONT_PATH")).to_vec());

        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let glyphon_viewport = glyphon::Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let text_renderer =
            TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);

        let mut text = Self {
            atlas,
            glyphon_viewport,
            text_renderer,

            bake_cache: BakeCache::create(spec, screen_resolution, &mut font_system),

            font_system,
            swash_cache,
        };
        text.bake(device, queue);
        text
    }

    /// Updates this [Text] with the new [TextSpec].
    pub fn update<'a, TextIterator: IntoIterator<Item = (&'a str, (f32, f32), Alignment)>>(
        &mut self,
        (device, queue): (&Device, &Queue),
        spec: TextSpec<'a, TextIterator>,
    ) {
        self.bake_cache = BakeCache::create(
            spec,
            self.bake_cache.screen_resolution,
            &mut self.font_system,
        );
        self.bake(device, queue);
    }

    /// Updates the viewport resolution of this [Text]
    pub fn update_viewport(
        &mut self,
        (device, queue): (&Device, &Queue),
        screen_resolution: (u32, u32),
    ) {
        self.bake_cache.screen_resolution = screen_resolution;
        self.bake(device, queue);
    }

    /// Bakes the [BakeCache] of this [Text] to the [Text::text_renderer]
    fn bake(&mut self, device: &Device, queue: &Queue) {
        let BakeCache {
            text_buffers,
            color,
            viewport_projection,
            screen_resolution,
        } = &self.bake_cache;

        // update the viewport to the set resolution
        self.glyphon_viewport.update(
            queue,
            Resolution {
                width: screen_resolution.0,
                height: screen_resolution.1,
            },
        );

        // create the TextAreas
        let text_areas = text_buffers.iter().map(|(buf, pos, alignment)| {
            to_text_area(
                buf,
                *pos,
                *alignment,
                *color,
                *viewport_projection,
                *screen_resolution,
            )
        });

        // bake the text to display
        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.glyphon_viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();
    }

    /// Draws this [Text].
    ///
    /// Will overwrite bind groups.
    /// If `REBIND` is `true`, will call the passed `rebind`-function to rebind groups.
    pub fn draw<const REBIND: bool>(
        &self,
        render_pass: &mut RenderPass<'_>,
        rebind: impl FnOnce(&mut RenderPass),
    ) {
        self.text_renderer
            .render(&self.atlas, &self.glyphon_viewport, render_pass)
            .unwrap();

        if REBIND {
            rebind(render_pass);
        }
    }
}

impl BakeCache {
    /// Creates a new [BakeCache] from the passed [TextSpec]
    fn create<'a, TextIterator: IntoIterator<Item = (&'a str, (f32, f32), Alignment)>>(
        TextSpec {
            viewport_projection,
            font_size,
            font_family,
            texts,
            color,
        }: TextSpec<'a, TextIterator>,
        screen_resolution: (u32, u32),
        font_system: &mut FontSystem,
    ) -> Self {
        // create the text buffers
        let text_buffers: Vec<_> = texts
            .into_iter()
            .map(|(text, pos, alignment)| {
                (
                    to_text_buffer(text, font_system, font_size, font_family),
                    pos,
                    alignment,
                )
            })
            .collect();

        Self {
            text_buffers,
            color: Color::rgba(color[0], color[1], color[2], color[3]),
            viewport_projection,
            screen_resolution,
        }
    }
}

/// Creates a [glyphon::Buffer] of the passed `text`.
fn to_text_buffer(
    text: &str,
    font_system: &mut FontSystem,
    font_size: f32,
    font_family: &str,
) -> Buffer {
    let mut text_buffer = Buffer::new(font_system, Metrics::new(font_size, 1.2 * font_size));
    text_buffer.set_size(font_system, None, None);
    text_buffer.set_text(
        font_system,
        text,
        Attrs::new().family(Family::Name(font_family)),
        Shaping::Advanced,
    );
    text_buffer.shape_until_scroll(font_system, false);
    text_buffer
}

/// Creates a [TextArea] of the passed [glyphon::Buffer].
/// Will handle alignment.
fn to_text_area(
    text_buffer: &Buffer,
    (x, y): (f32, f32),
    alignment: Alignment,
    color: Color,
    viewport: ViewportProjection,
    screen_resolution: (u32, u32),
) -> TextArea {
    let bounds = TextBounds {
        left: 0,
        top: 0,
        right: screen_resolution.0 as i32,
        bottom: screen_resolution.1 as i32,
    };

    // Transform the coordinates:
    let mat: Mat4 = viewport.into();
    // Into wgsl view-space
    let (x, y, _) = mat.transform_point3(Vec3::new(x, y, 0.)).into();
    fn map(val: f32, in_start: f32, in_end: f32, out_start: f32, out_end: f32) -> f32 {
        out_start + ((out_end - out_start) / (in_end - in_start)) * (val - in_start)
    }
    // Then map back into glyphon viewport-space
    let x = map(x, -1., 1., 0., screen_resolution.0 as f32);
    let y = map(y, -1., 1., screen_resolution.1 as f32, 0.);

    // Average width and height scale:
    let scale = get_scale(mat, screen_resolution);

    // Align in glyphon viewport-space
    let (x, y) = get_aligned_position(
        alignment,
        (x, y),
        || {
            text_buffer
                .layout_runs()
                .map(|r| r.line_w)
                .fold(0., f32::max)
                * scale
        },
        || {
            text_buffer
                .layout_runs()
                .map(|r| r.line_height)
                .sum::<f32>()
                * scale
        },
    );

    TextArea {
        buffer: text_buffer,
        left: x,
        top: y,
        scale,
        bounds,
        default_color: color,
        custom_glyphs: &[],
    }
}

/// Aligns an element at the passed position.
/// The width and height will be determined lazily if needed.
fn get_aligned_position(
    alignment: Alignment,
    (x, y): (f32, f32),
    w: impl FnOnce() -> f32,
    h: impl FnOnce() -> f32,
) -> (f32, f32) {
    let x = match alignment.0 {
        HAlignment::Left => x,
        HAlignment::Center => x - w() / 2.,
        HAlignment::Right => x - w(),
    };
    let y = match alignment.1 {
        VAlignment::Top => y,
        VAlignment::Center => y - h() / 2.,
        VAlignment::Bottom => y - h(),
    };
    (x, y)
}

/// Gets the scaling-factor to use.
/// Will average scale in x and y direction.
///
/// When debugging, will warn if the two scales are off.
fn get_scale(projection_matrix: Mat4, screen_resolution: (u32, u32)) -> f32 {
    // Get scale:
    // transform a unit from input space into canvas space,
    // then into screen space

    let canvas_unit_x = projection_matrix.transform_vector3(Vec3::new(1., 0., 0.)).x / 2.0;
    let scale_x = canvas_unit_x * screen_resolution.0 as f32;
    let canvas_unit_y = projection_matrix.transform_vector3(Vec3::new(0., 1., 0.)).y / 2.0;
    let scale_y = -canvas_unit_y * screen_resolution.1 as f32;

    const MAX_DIFF: f32 = 0.001;
    if cfg!(debug_assertions) && (scale_x - scale_y).abs() > MAX_DIFF {
        warn!("Different scale: {scale_x} != {scale_y}");
    }

    // Average directions
    (scale_x + scale_y) / 2.
}
