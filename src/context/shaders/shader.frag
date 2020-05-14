#version 450

layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec3 o_normal;
layout(location = 2) in vec2 v_texture;

layout(location = 0) out vec4 colour;

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 perspective;
    mat4 view;
    vec3 light_direction;
    vec3 ambient_colour;
    int mode;
};
layout(set = 0, binding = 1) uniform texture2D tex;
layout(set = 0, binding = 2) uniform sampler samp;

const int NORMAL = 1;
const int SHADELESS = 2;
const int WHITE = 3;
const int VERTEX_COLOURED = 4;

const float AMBIENT_STRENGTH = 1.0;
const vec3 LIGHT_COLOUR = vec3(1.0, 1.0, 1.0);
const float MIN_DIFFUSE = 0.1;

void main() {
    /*vec3 c;

    if (mode == 0) {
        c = vec3(1.0, 0.0, 0.0);
    } else if (mode == 1) {
        c = vec3(1.0, 1.0, 0.0);
    } else if (mode == 2) {
        c = vec3(0.0, 1.0, 0.0);
    } else if (mode == 3) {
        c = vec3(0.0, 1.0, 1.0);
    } else if (mode == 4) {
        c = vec3(0.0, 0.0, 1.0);
    }

    colour = vec4(c, 1.0);*/

    vec4 texture_colour = texture(sampler2D(tex, samp), v_texture);

    if (mode == VERTEX_COLOURED) {
        colour = vec4(o_normal, 1.0);
    } else if (mode == SHADELESS) {
        colour = texture_colour;
    } else if (mode == WHITE) {
        colour = vec4(vec3(1.0), v_texture.x);
    } else {
        // Ambient
        vec3 ambient = ambient_colour * AMBIENT_STRENGTH; 

        // Diffuse
        vec3 norm = normalize(v_normal);
        vec3 light_dir = normalize(light_direction);  

        float diff = max(dot(norm, light_dir), MIN_DIFFUSE);
        vec3 diffuse = diff * LIGHT_COLOUR;

        vec3 result = (ambient + diffuse) * texture_colour.rgb;
        colour = vec4(result, 1.0);
    }
}
