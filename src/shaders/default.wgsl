struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct InstanceInput {
    @builtin(instance_index) id: u32,
    @location(1) position: vec4<f32>,
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

struct WorldInfo {
    time: f32,
    delta: f32,
};

struct PushConstants {
    dimensions: vec3<u32>,
    world_info: WorldInfo,
}

var<push_constant> push_constants: PushConstants;

@group(0) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(in: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let vpos = instance.position.xyz + in.position;
    out.clip_position = camera.projection * camera.view * vec4(vpos, 1.0);
    
    let x_id = instance.id % push_constants.dimensions.x;
    let y_id = (instance.id / push_constants.dimensions.x) % push_constants.dimensions.y;
    let z_id = (instance.id / (push_constants.dimensions.x * push_constants.dimensions.y)) % push_constants.dimensions.z;

    let col_offset = 0.5 * normalize(vec3<f32>(
        f32(x_id) / f32(push_constants.dimensions.x),
        f32(z_id) / f32(push_constants.dimensions.z),
        f32(y_id) / f32(push_constants.dimensions.y)
    ));

    out.vertex_color = vec3(0.3, 0.1, 0.3) + col_offset;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> Attachments {
    var result: Attachments;
    result.color = vec4(in.vertex_color, 1.0);
    return result;
}
