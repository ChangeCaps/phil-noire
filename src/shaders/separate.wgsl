struct VertexInput {
	[[location(0)]] position: vec3<f32>;
	[[location(1)]] normal: vec3<f32>;
	[[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
	[[builtin(position)]] position: vec4<f32>;
	[[location(0)]] w_position: vec4<f32>;
	[[location(1)]] w_normal: vec4<f32>;
	[[location(2)]] uv: vec2<f32>;
};

[[block]]
struct Camera {
	view_proj: mat4x4<f32>;
};

[[group(0), binding(0)]]
var<uniform> camera: Camera;

[[block]]
struct Transform {
	model: mat4x4<f32>;
};

[[group(0), binding(1)]]
var<uniform> transform: Transform;

[[stage(vertex)]]
fn main(in: VertexInput) -> VertexOutput {
	var out: VertexOutput;

	out.w_position = transform.model * vec4<f32>(in.position, 1.0);
	out.w_normal = transform.model * vec4<f32>(in.normal, 0.0);
	out.position = camera.view_proj * out.w_position;
	out.uv = in.uv;

	return out;
}

[[block]]
struct PbrMaterial {
	albedo: vec3<f32>;
	emission: vec3<f32>;
	specular_bloom: f32;
};

[[group(0), binding(2)]]
var<uniform> material: PbrMaterial;

struct FragmentOutput {
	[[location(0)]] position: vec4<f32>;
	[[location(1)]] normal: vec4<f32>;
	[[location(2)]] albedo: vec4<f32>;
	[[location(3)]] emission: vec4<f32>;
};

[[stage(fragment)]]
fn main(in: VertexOutput) -> FragmentOutput {
	var out: FragmentOutput;

	out.position = vec4<f32>(in.w_position.xyz, material.specular_bloom);
	out.normal = vec4<f32>(in.w_normal.xyz, 1.0);
	out.albedo = vec4<f32>(material.albedo, 0.0);
	out.emission = vec4<f32>(material.emission, 0.0);

	return out;
}
