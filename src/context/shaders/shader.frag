#version 140

in vec3 v_normal;
in vec2 v_texture;
in vec3 o_normal;

out vec4 colour;
uniform vec3 light_direction;
uniform vec3 ambient_colour;
uniform sampler2D tex;
uniform int mode;

const int NORMAL = 1;
const int SHADELESS = 2;
const int WHITE = 3;
const int VERTEX_COLOURED = 4;

const float AMBIENT_STRENGTH = 1.0;
const vec3 LIGHT_COLOUR = vec3(1.0, 1.0, 1.0);
const float MIN_DIFFUSE = 0.1;

void main() {
    vec4 texture_colour = texture(tex, v_texture);

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