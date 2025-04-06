struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct InstanceInput {
    @builtin(instance_index) id: u32,
    @location(1) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) vertex_color: vec3<f32>,
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
fn vs_main(in: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let vpos = instance.position + in.position;
    out.clip_position = camera.projection * camera.view * vec4(vpos, 1.0);
    
    let x_id = instance.id % 1024;
    let y_id = (instance.id / 1024) % 1024;
    let z_id = (instance.id / (1024 * 1024)) % 1024;

    let col_offset = 0.5 * normalize(vec3<f32>(f32(x_id) / 1024.0, f32(z_id) / 1024.0, f32(y_id) / 1024.0));

    out.vertex_color = vec3(0.5, 0.5, 0.5) + col_offset;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> Attachments {
    var result: Attachments;
    result.color = vec4(in.vertex_color, 1.0);
    return result;
}
