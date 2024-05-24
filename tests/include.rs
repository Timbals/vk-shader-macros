use vk_shader_macros::{include_glsl, ShaderData};

#[allow(dead_code)]
static TEST: ShaderData =
    include_glsl!("example.vert", version: 450, optimize: size, target: vulkan1_1);
