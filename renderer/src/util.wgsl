#define_import_path util

// https://github.com/gfx-rs/wgpu/issues/4426
fn to_color(c: u32) -> vec4<f32> {
	var r = (c >>  0) &0xFF;
	var g = (c >>  8) &0xFF;
	var b = (c >> 16) &0xFF;
	var a = (c >> 24) &0xFF;
	return vec4<f32>(f32(r) / 255., f32(g) / 255., f32(b) / 255., f32(a) / 255.);
}
