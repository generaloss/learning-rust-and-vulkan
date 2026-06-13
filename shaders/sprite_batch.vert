#version 450

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec4 a_color;
layout(location = 2) in vec2 a_uv;
layout(location = 3) in uint a_texture_id;

layout(location = 0) out vec4 v_color;
layout(location = 1) out vec2 v_uv;
layout(location = 2) flat out uint v_texture_id;

layout(push_constant) uniform PushConstants {
    mat4 u_combined;
};

void main() {
    v_color = a_color;
    v_uv = a_uv;
    v_texture_id = a_texture_id;
    gl_Position = u_combined * vec4(a_pos, 0.0, 1.0);
}