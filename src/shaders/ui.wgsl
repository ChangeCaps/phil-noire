struct VertexInput {
	[[location(0)]] position: vec2<f32>;
	[[location(1)]] uv: vec2<f32>;
	[[location(2)]] color: vec4<f32>;
};

struct VertexOutput {
	[[builtin(position)]] position: vec4<f32>;
	[[location(0)]] uv: vec2<f32>;
	[[location(1)]] color: vec4<f32>;
};

[[block]]
struct ScreenSize {
	size: vec2<f32>;
};

[[group(0), binding(0)]]
var<uniform> screen_size: ScreenSize;

[[stage(vertex)]]
fn main(in: VertexInput) -> VertexOutput {
	var out: VertexOutput;

	out.position = vec4<f32>(
		2.0 * in.position.x / screen_size.size.x - 1.0, 
		1.0 - 2.0 * in.position.y / screen_size.size.y,
		0.0,
		1.0
	);
	out.uv = in.uv;
	out.color = in.color;

	return out;
}

[[group(0), binding(1)]]
var texture: texture_2d<f32>;

[[group(1), binding(0)]]
var sampler: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let color = textureSample(texture, sampler, in.uv) * in.color;

	return color;
}
