use std::path::Path;

use naviz_animator::animator::Animator;
use naviz_import::{ImportFormat, ImportOptions};
use naviz_parser::config::{machine::MachineConfig, visual::VisualConfig};
use naviz_video::{VideoExport, VideoProgress};
use pyo3::{
    create_exception,
    exceptions::{PyException, PyIOError, PyValueError},
    prelude::*,
    types::{PyDict, PyString},
};
use serde_pyobject::{from_pyobject, to_pyobject};

create_exception!(naviz, ParseError, PyException);
create_exception!(naviz, VideoExportError, PyException);

#[pyclass]
struct Repository(naviz_repository::Repository);

#[pymethods]
impl Repository {
    /// Get the styles-repository
    #[staticmethod]
    pub fn styles() -> PyResult<Self> {
        Ok(Self(
            naviz_repository::Repository::empty()
                .bundled_styles()
                .map_err(|_| ParseError::new_err("Failed to read bundled styles"))?
                .user_dir_styles()
                .map_err(|_| ParseError::new_err("Failed to read styles from user-directory"))?,
        ))
    }

    /// Get the machines-repository
    #[staticmethod]
    pub fn machines() -> PyResult<Self> {
        Ok(Self(
            naviz_repository::Repository::empty()
                .bundled_machines()
                .map_err(|_| ParseError::new_err("Failed to read bundled machines"))?
                .user_dir_machines()
                .map_err(|_| ParseError::new_err("Failed to read machines from user-directory"))?,
        ))
    }

    /// Get a config entry by `id` from this repository
    pub fn get(&self, id: &str) -> PyResult<Option<String>> {
        if let Some(content) = self.0.get_raw(id) {
            let content =
                content.map_err(|_| PyIOError::new_err("Failed to get entry from repository"))?;
            return String::from_utf8(content.into_owned())
                .map(Some)
                .map_err(|_| PyIOError::new_err("Invalid UTF-8"));
        }
        Ok(None)
    }
}

/// Export a video from the `input` to the `output`-location
/// at the specified `resolution`
/// with the specified framerate (`fps`)
/// using the `machine` and `style` configs
/// (use the [Repository] to get configs by id).
/// When `import_options` are specified,
/// the `input` is imported from the specified format.
#[pyfunction]
#[pyo3(signature = (input, output, resolution, fps, machine, style, import_options=None))]
fn export_video(
    input: &str,
    output: &str,
    resolution: (u32, u32),
    fps: u32,
    machine: &str,
    style: &str,
    import_options: Option<Bound<PyDict>>,
) -> PyResult<()> {
    let input = if let Some(import_options) = import_options {
        // Import the input
        let import_options: ImportOptions = from_pyobject(import_options)
            .map_err(|e| PyValueError::new_err(format!("Invalid input options: {e}")))?;
        import_options
            .import(input.as_bytes())
            .map_err(|e| ParseError::new_err(format!("Failed to import input: {e:?}")))?
    } else {
        // Load the input
        let input = naviz_parser::input::lexer::lex(input)
            .map_err(|_| ParseError::new_err("Failed to lex input"))?;
        let input = naviz_parser::input::parser::parse(&input)
            .map_err(|_| ParseError::new_err("Failed to parse input"))?;
        naviz_parser::input::concrete::Instructions::new(input)
            .map_err(|_| ParseError::new_err("Failed to convert input to instructions"))?
    };

    // Machine config
    let machine = naviz_parser::config::lexer::lex(machine)
        .map_err(|_| ParseError::new_err("Failed to lex machine"))?;
    let machine = naviz_parser::config::parser::parse(&machine)
        .map_err(|_| ParseError::new_err("Failed to parse machine"))?;
    let machine: naviz_parser::config::generic::Config = machine.into();
    let machine: MachineConfig = machine
        .try_into()
        .map_err(|_| ParseError::new_err("Failed to convert machine to config"))?;

    // Visual config
    let style = naviz_parser::config::lexer::lex(style)
        .map_err(|_| ParseError::new_err("Failed to lex style"))?;
    let style = naviz_parser::config::parser::parse(&style)
        .map_err(|_| ParseError::new_err("Failed to parse style"))?;
    let style: naviz_parser::config::generic::Config = style.into();
    let style: VisualConfig = style
        .try_into()
        .map_err(|_| ParseError::new_err("Failed to convert machine to config"))?;

    // Create animator
    let animator = Animator::new(machine, style, input);

    // Setup video export and start exporting
    let mut video = futures::executor::block_on(VideoExport::new(animator, resolution, fps));
    let (tx, rx) = std::sync::mpsc::channel();
    video.export_video(Path::new(output), tx);

    // Wait until export is done
    for m in rx {
        if let VideoProgress::Done(status) = m {
            if status.success() {
                // Successful export
                return Ok(());
            }
            // Unsuccessful export
            if let Some(code) = status.code() {
                return Err(VideoExportError::new_err(format!(
                    "ffmpeg returned error code {code}",
                )));
            } else {
                return Err(VideoExportError::new_err("ffmpeg exited abnormally"));
            }
        }
    }

    // No `VideoProgress::Done` was received,
    // but the `tx`-channel was dropped.
    // Video export must have crashed.
    Err(VideoExportError::new_err(
        "video export crashed during export",
    ))
}

/// Get the default import-settings for the specified import-`format`
#[pyfunction]
pub fn default_import_settings<'py>(
    py: Python<'py>,
    format: Bound<PyString>,
) -> PyResult<Bound<'py, PyAny>> {
    let format: ImportFormat = from_pyobject(format)?;
    let default_options: ImportOptions = format.into();
    let default_options = to_pyobject(py, &default_options)?;
    Ok(default_options)
}

/// A Python module implemented in Rust.
#[pymodule]
fn naviz(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(export_video, m)?)?;
    m.add_function(wrap_pyfunction!(default_import_settings, m)?)?;
    m.add_class::<Repository>()?;
    Ok(())
}
