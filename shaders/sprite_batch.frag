#version 450
#extension GL_EXT_nonuniform_qualifier : enable

layout(location = 0) in vec4 v_color;
layout(location = 1) in vec2 v_uv;
layout(location = 2) flat in uint v_texture_id;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D u_textures[32];

void main() {
    f_color = v_color * texture(u_textures[nonuniformEXT(v_texture_id)], v_uv);
}