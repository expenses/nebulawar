#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 texture;
layout(location = 3) in vec2 uv_offset;
layout(location = 4) in vec2 uv_dimensions;
layout(location = 5) in mat4 instance_pos;

layout(location = 0) out vec3 v_normal;
layout(location = 1) out vec3 o_normal;
layout(location = 2) out vec2 v_texture;

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 perspective;
    mat4 view;
    vec4 light_direction;
    vec4 ambient_colour;
    int mode;
    float dpi;
};

const int WHITE = 3;

void main() {
    if (mode == WHITE) {
        gl_PointSize = 2.0 * dpi;
        gl_LineWidth = dpi;
    }

    v_texture = vec2(
        uv_offset.x + texture.x * uv_dimensions.x,
        1.0 - (uv_offset.y + texture.y * uv_dimensions.y)
    );
    o_normal = normal;
    
    mat4 modelview = view * instance_pos;
    v_normal = transpose(inverse(mat3(instance_pos))) * normal;

    gl_Position = perspective * modelview * vec4(position, 1.0);
}
