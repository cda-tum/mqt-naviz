#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[cfg(target_arch = "wasm32")]
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use eframe::egui_wgpu::{WgpuSetup, WgpuSetupCreateNew};
use naviz_gui::App;
#[cfg(target_arch = "wasm32")]
use wgpu::{Adapter, DeviceDescriptor};

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NAViz")
            .with_app_id("naviz")
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../rsc/icon.png"))
                    // The icon we set is statically included in the binary
                    // and should therefore always be a valid png
                    .expect("Failed to load icon; This build is probably corrupt."),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "naviz",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(App::new_with_init_and_persistence(
                cc,
                Default::default(),
            )))
        }),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let mut web_options = eframe::WebOptions::default();
    let mut wgpu_setup = match web_options.wgpu_options.wgpu_setup {
        WgpuSetup::CreateNew(setup) => setup,
        _ => WgpuSetupCreateNew::default(),
    };
    let default_device_descriptor = wgpu_setup.device_descriptor;
    wgpu_setup.device_descriptor =
        limit_texture_to_screen_if_required(default_device_descriptor, || {
            let screen_resolution = web_sys::window()
                .and_then(|w| w.screen().ok())
                .and_then(|s| s.width().ok().zip(s.height().ok()));
            let device_pixel_ratio = web_sys::window().map(|w| w.device_pixel_ratio());
            screen_resolution
                .zip(device_pixel_ratio)
                .map(|((w, h), r)| (((w as f64) * r) as u32, ((h as f64) * r) as u32))
                .expect("Failed to get screen resolution")
        });
    web_options.wgpu_options.wgpu_setup = WgpuSetup::CreateNew(wgpu_setup);

    wasm_bindgen_futures::spawn_local(async {
        use eframe::wasm_bindgen::JsCast;

        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("naviz"))
            .and_then(|e| e.dyn_into().ok())
            .expect("Failed to get canvas");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    Ok(Box::new(App::new_with_init_and_persistence(
                        cc,
                        Default::default(),
                    )))
                }),
            )
            .await;

        // Remove the loading text and spinner:
        let loading_text = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading_text"));
        if let Some(loading_text) = loading_text {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

/// [egui requests the maximum 2d texture size to be `8192` to support 4k+ displays][egui_max_texture_size].
/// However, some devices ([~10%; especially older mobile devices][w3d_survey] *) may not support such large textures.
/// As these devices usually don't have as large screens,
/// we can get away with requesting a lower limit for texture size.
///
/// This function checks if the egui requested texture size is larger then supported by the device's GPU.
/// If that is the case and the device's GPU supports a texture size large enough to fit the devices screen,
/// it will reduce the requirement to the maximum capability of the GPU, and print a warning.
/// If the GPU does not support a texture large enough to fit the screen, it will just print an error.
/// If the GPU supports the texture size requested by egui, it will do nothing.
///
/// *) Almost all devices that support the newer WebGPU-spec support such a large texture (even mobile devices),
/// though currently [only few devices support this spec](https://web3dsurvey.com/webgpu).
///
/// [egui_max_texture_size]: https://github.com/emilk/egui/blob/f4ed394a85fdce6a141fab20002554442c8b69aa/crates/egui-wgpu/src/lib.rs#L305-307
/// [w3d_survey]: https://web3dsurvey.com/webgl2/parameters/MAX_TEXTURE_SIZE
#[cfg(target_arch = "wasm32")]
pub fn limit_texture_to_screen_if_required(
    device_descriptor: Arc<dyn Fn(&Adapter) -> DeviceDescriptor<'static> + Send + Sync>,
    screen_resolution: impl FnOnce() -> (u32, u32) + 'static + Copy + Send + Sync,
) -> Arc<dyn Fn(&Adapter) -> DeviceDescriptor<'static> + Send + Sync> {
    use log::{error, warn};

    Arc::new(move |adapter| {
        let mut device_descriptor = device_descriptor(adapter);
        let max_texture_size = adapter.limits().max_texture_dimension_2d;
        let egui_max_texture_size = device_descriptor.required_limits.max_texture_dimension_2d;
        if max_texture_size < egui_max_texture_size {
            let screen_resolution = screen_resolution();
            if max_texture_size < screen_resolution.0 || max_texture_size < screen_resolution.1 {
                error!("GPU does not support textures of size {egui_max_texture_size}, which is requested by egui.");
                error!("GPU does not support textures large enough to contain full screen surface; not patching requested size.");
            } else {
                warn!("GPU does not support textures of size {egui_max_texture_size}, which is requested by egui.");
                warn!("Downsizing to supported size of {max_texture_size} which is smaller than the current screen resolution.");
                warn!("Resizing the window to larger sizes may break egui.");
                device_descriptor.required_limits.max_texture_dimension_2d = max_texture_size;
            }
        }
        device_descriptor
    })
}
