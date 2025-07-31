use std::ops::{Deref, DerefMut};

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

/// A [Drawable] which wraps a child-[Drawable] and decides whether the child is visible or hidden.
/// If the child is not visible, it will never be drawn.
pub struct Hidable<Child: Drawable> {
    visible: bool,
    child: Child,
}

impl<Child: Drawable> Hidable<Child> {
    /// Creates a new [Hidable] wrapping the passed `child`.
    /// Will be visible by default.
    pub fn new(child: Child) -> Self {
        Self {
            visible: true,
            child,
        }
    }

    /// Update the visibility of this [Hidable]
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Update the visibility of this [Hidable]
    pub fn with_visibility(mut self, visible: bool) -> Self {
        self.set_visible(visible);
        self
    }
}

impl<Child: Drawable> Drawable for Hidable<Child> {
    fn draw<const REBIND: bool>(
        &self,
        render_pass: &mut RenderPass<'_>,
        rebind: impl Fn(&mut RenderPass),
    ) {
        if !self.visible {
            // no bindings are changed, therefore we never need to rebind
            return;
        }
        self.child.draw::<REBIND>(render_pass, rebind);
    }
}

impl<Child: Drawable> Deref for Hidable<Child> {
    type Target = Child;
    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl<Child: Drawable> DerefMut for Hidable<Child> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}
