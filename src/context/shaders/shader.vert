#version 150

in vec3 position;
in vec3 normal;
in vec2 texture;

out vec3 v_normal;
out vec2 v_texture;

uniform mat4 perspective;
uniform mat4 view;
uniform mat4 model;

void main() {
    v_texture = texture;
    
    mat4 modelview = view * model;
    v_normal = transpose(inverse(mat3(model))) * normal;

    gl_Position = perspective * modelview * vec4(position, 1.0);
}