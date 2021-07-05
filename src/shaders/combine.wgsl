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

[[group(0), binding(0)]]
var t_depth: texture_depth_2d;

[[group(0), binding(1)]]
var t_position: texture_2d<f32>;

[[group(0), binding(2)]]
var t_normal: texture_2d<f32>;

[[group(0), binding(3)]]
var t_albedo: texture_2d<f32>;

[[group(0), binding(4)]]
var t_emission: texture_2d<f32>;

[[group(0), binding(5)]]
var t_light: texture_2d<f32>;

[[group(1), binding(0)]]
var sampler: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let depth = textureSample(t_depth, sampler, in.uv);
	let position = textureSample(t_position, sampler, in.uv).xyz;
	let normal = textureSample(t_normal, sampler, in.uv).xyz;
	let albedo = textureSample(t_albedo, sampler, in.uv).rgb;
	let emission = textureSample(t_emission, sampler, in.uv).rgb;
	let light = textureSample(t_light, sampler, in.uv).rgb;

	var color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

	if (depth < 1.0) {
		color = albedo * light;
	}

	color = color + emission;

	return vec4<f32>(color, 1.0);
}
