@group(0) @binding(0)
var<storage, read_write> positions: array<vec3<f32>>;
@group(0) @binding(1)
var<storage, read_write> velocities: array<vec3<f32>>;

struct WorldInfo {
    time: f32,
    delta: f32,
};

struct PushConstants {
    world_info: WorldInfo,
}

var<push_constant> push_constants: PushConstants;

fn force(p: vec3<f32>) -> vec3<f32> {
    let l = length(p);
    let d = -p / l;

    return d * (10000.0 / l);
}

@compute
@workgroup_size(8, 8, 1) fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x + id.y * 1024 + id.z * 1024 * 1024;

    velocities[i] += force(positions[i]) * push_constants.world_info.delta;
    positions[i] += velocities[i] * push_constants.world_info.delta;
}
