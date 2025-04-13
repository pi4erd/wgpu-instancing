@group(0) @binding(0)
var<storage, read_write> positions: array<vec4<f32>>;
@group(0) @binding(1)
var<storage, read_write> velocities: array<vec4<f32>>;

struct WorldInfo {
    time: f32,
    delta: f32,
};

struct PushConstants {
    dimensions: vec4<u32>,
    world_info: WorldInfo,
}

var<push_constant> push_constants: PushConstants;

fn force(p: vec3<f32>) -> vec3<f32> {
    let l = length(p);
    let d = -p / l;

    return 1.0e9 * d / (l * l);
}

@compute
@workgroup_size(8, 8, 4) fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x + id.y * push_constants.dimensions.x + id.z * push_constants.dimensions.x * push_constants.dimensions.y;

    velocities[i] = vec4(velocities[i].xyz + force(positions[i].xyz) * push_constants.world_info.delta, 1.0);
    positions[i] = vec4(positions[i].xyz + velocities[i].xyz * push_constants.world_info.delta, 1.0);
}
