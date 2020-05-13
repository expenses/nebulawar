#version 150

in vec3 position;
in vec3 normal;
in vec2 texture;
in mat4 instance_pos;
in vec2 uv_dimensions;
in vec2 uv_offset;

out vec3 v_normal;
out vec3 o_normal;
out vec2 v_texture;

uniform mat4 perspective;
uniform mat4 view;

void main() {
    v_texture = uv_offset + texture * uv_dimensions;
    o_normal = normal;
    
    mat4 modelview = view * instance_pos;
    v_normal = transpose(inverse(mat3(instance_pos))) * normal;

    gl_Position = perspective * modelview * vec4(position, 1.0);
}
