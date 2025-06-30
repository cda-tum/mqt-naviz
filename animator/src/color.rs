use std::ops::{Add, Deref, DerefMut, Mul};

/// A color, consisting of an `r`, `g`, `b`, and `a` component
#[derive(Clone, Copy, Default)]
pub struct Color(pub [u8; 4]);

impl Deref for Color {
    type Target = [u8; 4];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Color {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Color {
    /// Mixes this color over the `base`-color
    pub fn over(&self, base: &Self) -> Self {
        const RED: usize = 0;
        const GREEN: usize = 1;
        const BLUE: usize = 2;
        const ALPHA: usize = 3;

        let self_red: u32 = self[RED] as u32;
        let self_green: u32 = self[GREEN] as u32;
        let self_blue: u32 = self[BLUE] as u32;
        let self_alpha: u32 = self[ALPHA] as u32;
        let base_red: u32 = base[RED] as u32;
        let base_green: u32 = base[GREEN] as u32;
        let base_blue: u32 = base[BLUE] as u32;
        let base_alpha: u32 = base[ALPHA] as u32;

        let a = self_alpha + (base_alpha * (255 - self_alpha) / 255);
        if a == 0 {
            return Self([0, 0, 0, 0]);
        }
        let r = (self_red * self_alpha + base_red * base_alpha * (255 - self_alpha) / 255) / a;
        let g = (self_green * self_alpha + base_green * base_alpha * (255 - self_alpha) / 255) / a;
        let b = (self_blue * self_alpha + base_blue * base_alpha * (255 - self_alpha) / 255) / a;

        Self([r as u8, g as u8, b as u8, a as u8])
    }
}

impl From<naviz_parser::common::color::Color> for Color {
    fn from(value: naviz_parser::common::color::Color) -> Self {
        Self(value.rgba())
    }
}

impl Mul<f32> for Color {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        let scale = |v: u8| ((v as f32) * rhs) as u8;
        Self(self.0.map(scale))
    }
}

impl Add<Color> for Color {
    type Output = Self;
    fn add(self, rhs: Color) -> Self::Output {
        Self([
            self.0[0].saturating_add(rhs.0[0]),
            self.0[1].saturating_add(rhs.0[1]),
            self.0[2].saturating_add(rhs.0[2]),
            self.0[3].saturating_add(rhs.0[3]),
        ])
    }
}
