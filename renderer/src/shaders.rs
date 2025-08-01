//! Some helper functions for linking and compiling shaders using a [Composer].

use std::{borrow::Cow, collections::HashMap};

use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, ComposerError, NagaModuleDescriptor, ShaderDefValue,
};
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};

/// Helper to link a shader `source` using a [Composer]
/// and then compile it on the [Device].
/// It will be given the passed `path` as a name / file-path.
/// Allows passing defines to the shader (or just [Default::default]).
pub fn compile_shader(
    device: &Device,
    composer: &mut Composer,
    source: &'static str,
    path: &'static str,
    defines: HashMap<String, ShaderDefValue>,
) -> Result<ShaderModule, Box<ComposerError>> {
    let module = composer.make_naga_module(NagaModuleDescriptor {
        source,
        file_path: path,
        shader_defs: defines,
        ..Default::default()
    })?;

    let shader = device.create_shader_module(ShaderModuleDescriptor {
        source: ShaderSource::Naga(Cow::Owned(module)),
        label: Some(path),
    });

    Ok(shader)
}

/// Creates a [Composer].
///
/// Will be validating iff compiling with debug_assertions.
pub fn create_composer() -> Composer {
    if cfg!(debug_assertions) {
        Composer::default()
    } else {
        Composer::non_validating()
    }
}

/// Loads the default shaders to the passed [Composer].
pub fn load_default_shaders(mut composer: Composer) -> Result<Composer, Box<ComposerError>> {
    composer.add_composable_module(ComposableModuleDescriptor {
        source: include_str!("./util.wgsl"),
        file_path: "util.wgsl",
        ..Default::default()
    })?;
    composer.add_composable_module(ComposableModuleDescriptor {
        source: include_str!("./globals.wgsl"),
        file_path: "globals.wgsl",
        ..Default::default()
    })?;
    composer.add_composable_module(ComposableModuleDescriptor {
        source: include_str!("./viewport.wgsl"),
        file_path: "viewport.wgsl",
        ..Default::default()
    })?;

    Ok(composer)
}
