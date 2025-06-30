#import util::to_color;
#import viewport::viewport_projection;

struct CircleSpec {
	@location(0) center: vec2<f32>,
	@location(1) radius: f32,
	@location(2) radius_inner: f32,
	@location(3) color: u32,
}

struct VOutput {
	// center in input space
	@location(0) center: vec2<f32>,
	@location(1) radius: f32,
	@location(2) radius_inner: f32,
	@location(3) color: u32,
	// current position in input space
	@location(4) pos: vec2<f32>,
	@builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32, spec: CircleSpec) -> VOutput {
	// Generate vertices for the bounding square to draw the circle in

	// Positions for bounding square
	var pos = array<vec2<f32>,4>(
		spec.center + vec2<f32>(-spec.radius,  spec.radius),
		spec.center + vec2<f32>(-spec.radius, -spec.radius),
		spec.center + vec2<f32>( spec.radius, -spec.radius),
		spec.center + vec2<f32>( spec.radius,  spec.radius),
	);
	// Indices for triangles of bounding square
	var idx = array<u32, 6>(0, 1, 2, 2, 3, 0);

	var out: VOutput;
	out.center = spec.center;
	out.radius = spec.radius;
	out.radius_inner = spec.radius_inner;
	out.color = spec.color;
	out.pos = pos[idx[in_vertex_index]];
	out.position = viewport_projection * vec4<f32>(pos[idx[in_vertex_index]], 0.0, 1.0);
	return out;
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
	// distance to center
	var dist = distance(in.center, in.pos);

	if dist > in.radius || dist < in.radius_inner { // not in draw region
		return vec4<f32>(1.0, 1.0, 1.0, 0.0);
	} else { // in draw region
		return to_color(in.color);
	}
}
