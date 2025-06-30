#import util::to_color;
#import viewport::viewport_projection;

struct LineSpec {
	@location(0) start: vec2<f32>,
	@location(1) end: vec2<f32>,
	@location(2) color: u32,
	@location(3) width: f32,
	@location(4) segment_length: f32,
	@location(5) duty: f32,
}

struct VOutput {
	// start of line in input-space
	@location(0) start: vec2<f32>,
	// end of line in input-space
	@location(1) end: vec2<f32>,
	// current position in input space
	@location(2) pos: vec2<f32>,
	@location(3) segment_length: f32,
	@location(4) duty: f32,
	@location(5) color: u32,
	@builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32, spec: LineSpec) -> VOutput {
	// Generate vertices for the rectangle to draw the line as

	// Construct perpendicular vector of length width / 2
	var perp = normalize(spec.end - spec.start);
	perp = vec2<f32>(perp.y, -perp.x);
	perp = perp * spec.width / 2;

	// Positions for bounding rect of line
	var pos = array<vec2<f32>,4>(
		spec.start - perp,
		spec.end   - perp,
		spec.end   + perp,
		spec.start + perp,
	);
	// Indexes for triangles of bounding rect
	var idx = array<u32, 6>(0, 1, 2, 2, 3, 0);

	// Construct vertex output
	var out: VOutput;
	out.start = spec.start;
	out.end = spec.end;
	out.pos = pos[idx[in_vertex_index]];
	out.segment_length = spec.segment_length;
	out.duty = spec.duty;
	out.color = spec.color;
	out.position = viewport_projection * vec4<f32>(pos[idx[in_vertex_index]].x, pos[idx[in_vertex_index]].y, 0.0, 1.0);
	return out;
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
	// Not dashed (no segments or fully drawn segments)
	if in.segment_length <= 0 || in.duty >= 1 {
		return to_color(in.color);
	}

	// Dashed
	var dist = dot(normalize(in.end - in.start), in.pos - in.start); // Current distance
	dist = dist + ((in.duty * in.segment_length) / 2); // offset by half a drawn segment

	var dist_local = fract(dist / in.segment_length); // Distance in local segment

	if dist_local <= in.duty { // Draw dash
		return to_color(in.color);
	} else { // Draw empty
		return vec4<f32>(1.0, 1.0, 1.0, 0.0);
	}
}
