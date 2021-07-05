struct VertexOutput {
	[[builtin(position)]] position: vec4<f32>;
	[[location(0)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn main([[builtin(vertex_index)]] index: u32) -> VertexOutput {
	var out: VertexOutput;

	let x = -1.0 + f32((index & 1u) << 2u);
	let y = -1.0 + f32((index & 2u) << 1u);
	out.position = vec4<f32>(x, y, 0.0, 1.0);
	out.uv = (vec2<f32>(x, y) + 1.0) / 2.0;
	out.uv.y = 1.0 - out.uv.y;
	
	return out;
}

[[block]]
struct Uniforms {
	horizontal: bool;
	iterations: u32;
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

[[group(1), binding(0)]]
var t_bloom: texture_2d<f32>;

[[group(2), binding(0)]]
var sampler: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	var out: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);

	let texel_size = 1.0 / vec2<f32>(textureDimensions(t_bloom));

	var i: u32 = 0u;
	loop {
		if (i >= uniforms.iterations) { break; }

		let mod = pow(1.0 - f32(i) / f32(uniforms.iterations), 7.0) * 0.227;

		if (uniforms.horizontal) {
			out = out + textureSample(t_bloom, sampler, in.uv + vec2<f32>(texel_size.x * f32(i), 0.0)) * mod;
			out = out + textureSample(t_bloom, sampler, in.uv - vec2<f32>(texel_size.x * f32(i), 0.0)) * mod;
		} else {
			out = out + textureSample(t_bloom, sampler, in.uv + vec2<f32>(0.0, texel_size.y * f32(i))) * mod;
			out = out + textureSample(t_bloom, sampler, in.uv - vec2<f32>(0.0, texel_size.y * f32(i))) * mod;
		}

		i = i + 1u;
	}

	return out;
}
