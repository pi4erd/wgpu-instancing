struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

struct Attachments {
    @location(0) color: vec4<f32>,
}

struct Camera {
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.projection * camera.view * vec4(in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> Attachments {
    var result: Attachments;
    result.color = vec4(1.0, 1.0, 1.0, 1.0);
    return result;
}
