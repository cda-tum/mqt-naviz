//! Import definitions/handling for the naviz-gui

use egui::{TextEdit, Ui};
use naviz_import::{ImportFormat, ImportOptions};

use crate::{drawable::Drawable, file_type::FileFilter};

impl FileFilter for ImportFormat {
    fn name(&self) -> &'static str {
        self.name()
    }

    fn extensions(&self) -> &'static [&'static str] {
        self.file_extensions()
    }
}

impl Drawable for &mut ImportOptions {
    /// Draws a settings-ui for these [ImportOptions].
    /// Edits from the ui will be reflected inside `self`.
    fn draw(self, ui: &mut Ui) {
        match self {
            ImportOptions::MqtNa(options) => {
                ui.horizontal(|ui| {
                    ui.label("Atom prefix");
                    ui.add(
                        TextEdit::singleline(&mut options.atom_prefix).desired_width(f32::INFINITY),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("CZ global zone");
                    ui.add(
                        TextEdit::singleline(&mut options.global_zones.cz)
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("RY global zone");
                    ui.add(
                        TextEdit::singleline(&mut options.global_zones.ry)
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("RZ global zone");
                    ui.add(
                        TextEdit::singleline(&mut options.global_zones.rz)
                            .desired_width(f32::INFINITY),
                    );
                });
            }
        }
    }
}
