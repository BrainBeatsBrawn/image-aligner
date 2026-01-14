#version 330
attribute vec3 position;
attribute vec2 texcoord;

varying lowp vec2 center;
varying lowp vec2 uv;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    vec4 res = Projection * Model * vec4(position, 1);
    uv = texcoord;
    gl_Position = res;
}
