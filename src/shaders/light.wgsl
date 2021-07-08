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

struct DirectionalLight {
	direction: vec3<f32>;
	color: vec3<f32>;
	strength: f32;
};

[[block]]
struct DirectionalLights {
	len: u32;
	lights: array<DirectionalLight, 8>;
};

[[group(0), binding(0)]]
var<uniform> directional_lights: DirectionalLights;

[[block]]
struct Camera {
	pos: vec3<f32>;
};

[[group(0), binding(1)]]
var<uniform> camera: Camera;

[[block]]
struct Uniforms {
	ambient_color: vec3<f32>;
	ambient_strength: f32;
};

[[group(0), binding(2)]]
var<uniform> uniforms: Uniforms;

[[group(1), binding(0)]]
var t_depth: texture_depth_2d;

[[group(1), binding(1)]]
var t_position: texture_2d<f32>;

[[group(1), binding(2)]]
var t_normal: texture_2d<f32>;

[[group(2), binding(0)]]
var sampler: sampler;

struct FragmentOutput {
	[[location(0)]] light: vec4<f32>;	
	[[location(1)]] emission: vec4<f32>;	
};

[[stage(fragment)]]
fn main(in: VertexOutput) -> FragmentOutput {
	var out: FragmentOutput;

	let depth = textureSample(t_depth, sampler, in.uv);
	let p = textureSample(t_position, sampler, in.uv);
	let position = p.xyz;
	let normal = textureSample(t_normal, sampler, in.uv).xyz;

	var light: vec3<f32> = uniforms.ambient_color * uniforms.ambient_strength;

	var i: u32 = 0u;
	loop {
		if (i >= directional_lights.len) { break; }

		let light_dir = -normalize(directional_lights.lights[i].direction);
		let view_dir = normalize(camera.pos - position);
		let half_dir = normalize(view_dir + light_dir);

		let color = directional_lights.lights[i].color;
		let strength = directional_lights.lights[i].strength;

		let diffuse_strength = max(dot(light_dir, normal), 0.0) * strength;
		let diffuse_color = color * diffuse_strength;

		let specular_strength = pow(max(dot(half_dir, normal), 0.0), 32.0) * strength;
		let specular_color = color * specular_strength;

		light = light + diffuse_color + specular_color;

		i = i + 1u;
	}

	out.light = vec4<f32>(light, 0.0);
	out.emission = vec4<f32>(light - 1.0, 0.0) * p.w;

	return out;
}
