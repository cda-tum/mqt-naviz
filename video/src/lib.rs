use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, ExitStatus, Stdio},
    sync::mpsc::{channel, Sender},
    thread,
};

use naviz_animator::animator::Animator;
use naviz_renderer::renderer::Renderer;
use wgpu::{
    Buffer, BufferView, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d,
    Features, Instance, InstanceDescriptor, Limits, LoadOp, MapMode, MemoryHints, Operations,
    Queue, RenderPassColorAttachment, RenderPassDescriptor, StoreOp, TexelCopyBufferInfo,
    TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};

/// Struct to export a video from an [Animator]
pub struct VideoExport {
    animator: Animator,
    renderer: Renderer,
    texture: Texture,
    device: Device,
    queue: Queue,
    output_buffer: Buffer,
    fps: u32,
    screen_resolution: (u32, u32),
}

/// Video progress update event
pub enum VideoProgress {
    /// Render-update (`current time`, `duration`)
    Render(f32, f32),
    /// Encode-update (`current time`, `duration`)
    Encode(f32, f32),
    /// Finished export (with `ffmpeg` exit status; may not always be successful)
    Done(ExitStatus),
}

/// Creates a headless rendering [Device] and [Queue]
async fn create_device() -> (Device, Queue) {
    let instance = Instance::new(&InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("No adapter");
    adapter
        .request_device(&DeviceDescriptor {
            label: Some("naviz video renderer"),
            required_features: Features::default(),
            required_limits: Limits::default(),
            memory_hints: MemoryHints::default(),
            trace: wgpu::Trace::default(),
        })
        .await
        .expect("Failed to create device")
}

/// Creates an output-buffer on the passed [Device] for the specified `screen_resolution` and `pixel_size`.
fn create_output_buffer(device: &Device, screen_resolution: (u32, u32), pixel_size: u32) -> Buffer {
    let output_buffer_size =
        (pixel_size * screen_resolution.0 * screen_resolution.1) as wgpu::BufferAddress;
    let output_buffer_desc = wgpu::BufferDescriptor {
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: Some("naviz output buffer"),
        mapped_at_creation: false,
    };
    device.create_buffer(&output_buffer_desc)
}

impl VideoExport {
    /// Creates a new [VideoExport] from the passed [Animator]
    /// and with the passed `screen_resolution` and `fps`
    pub async fn new(animator: Animator, screen_resolution: (u32, u32), fps: u32) -> Self {
        let (device, queue) = create_device().await;
        let texture_format = TextureFormat::Rgba8Unorm;
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("naviz render target"),
            size: Extent3d {
                width: screen_resolution.0,
                height: screen_resolution.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: texture_format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let renderer = Renderer::new(
            &device,
            &queue,
            texture_format,
            &animator.config(),
            &animator.state((0.).into()),
            screen_resolution,
        );

        let output_buffer = create_output_buffer(
            &device,
            screen_resolution,
            texture_format.components() as u32,
        );

        Self {
            animator,
            renderer,
            texture,
            device,
            queue,
            output_buffer,
            fps,
            screen_resolution,
        }
    }

    /// Gets the frame times for the duration of the [Animator] and the set `fps`.
    fn get_frame_times(&self) -> impl Iterator<Item = f32> {
        let frame_count: u64 = (self.animator.duration() * self.fps)
            .ceil()
            .try_into()
            .unwrap();
        let fps = self.fps;
        (0..=frame_count).map(move |i| i as f32 * (1. / fps as f32))
    }

    /// Exports a video the the specified `target`-path using system-installed `ffmpeg`
    pub fn export_video(&mut self, target: &Path, progress: Sender<VideoProgress>) {
        let mut ffmpeg = Command::new("ffmpeg")
            .args([
                "-progress",
                "-",
                "-f",
                "rawvideo",
                "-video_size",
                format!("{}x{}", self.screen_resolution.0, self.screen_resolution.1).as_str(),
                "-pixel_format",
                "rgba",
                "-framerate",
                format!("{}/1", self.fps).as_str(),
                "-i",
                "-",
                // Set chroma subsampling for some video players
                // See https://trac.ffmpeg.org/wiki/Encode/H.264#Encodingfordumbplayers
                // Should be ignored on formats that don't support it
                "-vf",
                "format=yuv420p",
                "-y",
            ])
            .arg(target)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to run ffmpeg");
        let mut ffmpeg_input = ffmpeg.stdin.take().unwrap();
        let ffmpeg_progress = ffmpeg.stdout.take().unwrap();
        let ffmpeg_progress = BufReader::new(ffmpeg_progress)
            .lines()
            .map_while(Result::ok)
            .filter_map(|l| l.strip_prefix("out_time_us=").map(|s| s.parse::<u64>()))
            .filter_map(Result::ok)
            .map(|ms| ms as f32 / 1_000_000.);

        let duration = self.animator.duration().try_into().unwrap();

        let encode_progress = progress.clone();
        thread::spawn(move || {
            ffmpeg_progress.for_each(|t| {
                let _ = encode_progress.send(VideoProgress::Encode(t, duration));
            });
        });

        self.get_frame_times().for_each(|time| {
            self.set_time(time);
            let frame = self.get_frame();
            ffmpeg_input
                .write_all(&frame)
                .expect("Failed to send frame to ffmpeg");
            drop(frame);
            self.output_buffer.unmap();
            let _ = progress.send(VideoProgress::Render(time, duration));
        });
        ffmpeg_input
            .flush()
            .expect("Failed to flush frames to ffmpeg");

        drop(ffmpeg_input);

        if let Ok(code) = ffmpeg.wait() {
            let _ = progress.send(VideoProgress::Done(code));
        }
    }

    /// Updates the [Renderer] to have the state of the [Animator] at the passed `time`
    fn set_time(&mut self, time: f32) {
        self.renderer.update(
            &mut (&self.device, &self.queue),
            &self.device,
            &self.queue,
            &self.animator.config(),
            &self.animator.state(time.into()),
        );
    }

    /// Renders the current frame and gets the resulting data as a [BufferView].
    /// [Self::output_buffer] will need to be [unmapped][Buffer::unmap] after the [BufferView] was used.
    fn get_frame(&self) -> BufferView<'_> {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let [r, g, b, a] = self.animator.background();
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.texture.create_view(&Default::default()),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: r as f64 / u8::MAX as f64,
                            g: g as f64 / u8::MAX as f64,
                            b: b as f64 / u8::MAX as f64,
                            a: a as f64 / u8::MAX as f64,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.renderer.draw(&mut render_pass);
        }

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                aspect: TextureAspect::All,
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            TexelCopyBufferInfo {
                buffer: &self.output_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        self.screen_resolution.0 * self.texture.format().components() as u32,
                    ),
                    rows_per_image: Some(self.screen_resolution.1),
                },
            },
            self.texture.size(),
        );

        self.queue.submit([encoder.finish()]);

        let buffer_slice = self.output_buffer.slice(..);
        let (tx, rx) = channel();
        buffer_slice.map_async(MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        rx.recv().unwrap().unwrap();

        buffer_slice.get_mapped_range()
    }
}
