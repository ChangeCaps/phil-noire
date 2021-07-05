struct VertexInput {
	[[location(0)]] position: vec3<f32>;
};

struct VertexOutput {
	[[builtin(position)]] position: vec4<f32>;
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

	out.position = camera.view_proj * transform.model * out.w_position;

	return out;
}
