use wgpu::RenderPass;

/// A trait for something which can be drawn/rendered.
pub trait Drawable {
    /// Draws this [Drawable], optionally calling `rebind` if enabled by setting `REBIND`-parameter to `true`.
    ///
    /// Note: `rebind` is intended to get the bindings into their initial state.
    /// Implementations that do not change any bindings may therefore choose not to call `rebind`,
    /// even when `REBIND` is set to `true`.
    /// If you want to always modify the bindings, do so after the call instead.
    fn draw<const REBIND: bool>(
        &self,
        render_pass: &mut RenderPass<'_>,
        rebind: impl Fn(&mut RenderPass),
    );
}
