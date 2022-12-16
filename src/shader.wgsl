struct CameraUniform {
    pos: vec3<f32>,
    to_view: mat4x4<f32>,
    to_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct InstanceInput {
    @location(1) pos: vec3<f32>,
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) quad_position: vec2<f32>,
    @location(1) color: vec3<f32>,
}

let PI = 3.1415926535;

let DISTANCE_NEA = 50.0;
let DISTANCE_MID = 100.0;
let DISTANCE_FAR = 150.0;

let COLOR_NEA = vec3<f32>(1.0, 0.0, 0.0);
let COLOR_MID = vec3<f32>(0.0, 1.0, 0.0);
let COLOR_FAR = vec3<f32>(0.0, 0.2, 1.0);

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var model_to_view: mat4x4<f32> = camera.to_view * model_matrix;
    model_to_view[0][0] = 1.0;
    model_to_view[0][1] = 0.0;
    model_to_view[0][2] = 0.0;
    model_to_view[1][0] = 0.0;
    model_to_view[1][1] = 1.0;
    model_to_view[1][2] = 0.0;
    model_to_view[2][0] = 0.0;
    model_to_view[2][1] = 0.0;
    model_to_view[2][2] = 1.0;

    let dist: f32 = distance(instance.pos, camera.pos);

    var out: VertexOutput;
    out.clip_position = camera.to_proj * model_to_view * vec4<f32>(model.position, 0.0, 1.0);
    out.quad_position = model.position;
    out.color = COLOR_NEA;
    if dist > DISTANCE_NEA { out.color = mix(COLOR_NEA, COLOR_MID, smoothstep(DISTANCE_NEA, DISTANCE_MID, dist)); }
    if dist > DISTANCE_MID { out.color = mix(COLOR_MID, COLOR_FAR, smoothstep(DISTANCE_MID, DISTANCE_FAR, dist)); }
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let quad_dist: f32 = sqrt(in.quad_position.x * in.quad_position.x + in.quad_position.y * in.quad_position.y) * 2.0;
    let alpha: f32 = cos((quad_dist * PI) / 2.0);
    return vec4<f32>(in.color, clamp(alpha, 0.0, 1.0));
}
