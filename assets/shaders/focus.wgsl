// The time since startup data is in the globals binding which is part of the mesh_view_bindings import
#import bevy_pbr::{
    mesh_view_bindings::globals,
    forward_io::VertexOutput,
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {

    let speed = 10.0;

    let offset = dot(in.world_position, vec4<f32>(10.0, 10.0, 10.0, 0.0));

    if sin(offset + globals.time * speed) < 0.0 {
        // See https://github.com/bevyengine/bevy/pull/15782
        // and https://github.com/gfx-rs/wgpu/issues/4416
        if true {
            discard;
        }
        return vec4<f32>(0.0);
    }

    // The globals binding contains various global values like time
    // which is the time since startup in seconds
    let t = sin(globals.time * speed) * 0.5 + 0.5;

    let red = vec3<f32>(1.0, 0.0, 0.0);
    let green = vec3<f32>(0.0, 1.0, 0.0);

    return vec4<f32>(mix(red, green, t), 1.0);
}