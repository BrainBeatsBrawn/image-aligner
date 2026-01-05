#version 330
precision lowp float;

in vec2 uv;
in vec2 center;

layout(location=0) out vec4 fragColor;

//uniform vec2 mousePos;

uniform mat4 warpPoints; // First row: offset-space xs, 2nd: offset ys, 3rd: baseline xs, 4th: baseline ys
uniform float offsetOpacity; // How transparent the offset image is
uniform vec3 debugColor; // The color to use for debugging
uniform int drawDebug; // Whether to draw debug info

uniform sampler2D offsetImage;

#define M_PI 3.141592653589793

#define DRAW_DEBUG_OVERRIDE

#define GRID_WIDTH 0.05
#define HALF_GRID_WIDTH (GRID_WIDTH/2.0)

float cross2d(in vec2 a, in vec2 b)
{
    return a.x*b.y - a.y*b.x;
}

// Returns the distance of p from the line segment between p1 and p2
float lineSDF(in vec2 p, in vec2 p1, in vec2 p2, in float thickness)
{
    float length = distance(p1,p2);
    vec2 dir = normalize(p2-p1);
    vec2 p1ToP = p-p1;

    float axialProj = dot(dir, p1ToP); // how far along the axis
    float coaxialProj = dot(vec2(-dir.y,dir.x), p1ToP);// How far along the co-axis

    float coaxialSDF = abs(coaxialProj) - thickness/2;
    float axialSDF = abs(axialProj-length/2) - length/2;

    return max(coaxialSDF, axialSDF);
}

float quadrilateralSDF(in vec2 p, in vec2 p1, in vec2 p2, in vec2 p3, in vec2 p4, in float thickness)
{
    return min(
            min( lineSDF(p, p1, p2, thickness), lineSDF(p, p2, p4, thickness)),
            min( lineSDF(p, p4, p3, thickness), lineSDF(p, p3, p1, thickness))
           );
}

float pointSDF(in vec2 p, in vec2 c, in float radius)
{
    return length(c-p) - radius;
}

#define POINTER_ANGLE 0.6981 // 40 degrees
float angledPointSDF(in vec2 p, in vec2 c, in float radius, in float angle)
{
    float regDist = pointSDF(p, c, radius);
    float renderingAngle = atan(p.y-c.y, p.x-c.x);
    return max(regDist + (POINTER_ANGLE/(POINTER_ANGLE - max(abs(renderingAngle - angle), POINTER_ANGLE)) * radius *0.25), regDist);
}

float quadPointsSDF(in vec2 p, in vec2 p1, in vec2 p2, in vec2 p3, in vec2 p4, in float r)
{
    return min(
            min( angledPointSDF(p, p1, r, -M_PI*0.75), angledPointSDF(p, p2, r, -M_PI*0.25) ),
            min( angledPointSDF(p, p3, r,  M_PI*0.75), angledPointSDF(p, p4, r,  M_PI*0.25) )
           );
}

// Calculates forward bilinear interpolation between the 4 points
vec2 bilinear(in vec2 uv, in vec2 a, in vec2 b, in vec2 d, in vec2 c)
{
    return a + (b-a)*uv.x + (d-a)*uv.y + (a-b+c-d)*uv.x*uv.y;
}

// Calculates the inverse bilinear interpolation between the 4 points, an edited version of the
// function from demonstrated by Inigo Quilez at https://iquilezles.org/articles/ibilinear/
vec2 invBilinear( in vec2 p, in vec2 a, in vec2 b, in vec2 d, in vec2 c )
{
    vec2 res = vec2(-1.0);

    vec2 e = b-a;
    vec2 f = d-a;
    vec2 g = a-b+c-d;
    vec2 h = p-a;

    // Get a measure of the size of the region that we
    // can scale everything by to avoid rounding errors
    float avgLen = (distance(a,c) + distance(b,d))/2.0;
    e /= avgLen;
    f /= avgLen;
    g /= avgLen;
    h /= avgLen;
    p /= avgLen;
        
    float k2 = cross2d( g, f );
    float k1 = cross2d( e, f ) + cross2d( h, g );
    float k0 = cross2d( h, e );
    
    // if edges are parallel, this is a linear equation
    if( abs(k2)<0.001 )
    {
        res = vec2( (h.x*k1+f.x*k0)/(e.x*k1-g.x*k0), -k0/k1 );
    }
    // otherwise, it's a quadratic
	else
    {
        float w = k1*k1 - 4.0*k0*k2;
        if( w<0.0 ) return vec2(-1.0);
        w = sqrt( w );

        float ik2 = 0.5/k2;
        float v = (-k1 - w)*ik2;
        float u = (h.x - f.x*v)/(e.x + g.x*v);
        
        if( u<0.0 || u>1.0 || v<0.0 || v>1.0 )
        {
           v = (-k1 + w)*ik2;
           u = (h.x - f.x*v)/(e.x + g.x*v);
        }
        res = vec2( u, v );
    }
    
    return res;
}


void main()
{
    //// Draw some overlay debug info
    // By default, we're outside of the subregion and/or the mask, so we just draw a transparent texture
    // (Red, because of course it's red)
    fragColor = vec4(1.0, 0.0, 0.0, 0.5);

    // TODO: For efficiency's sake, we could do a bounding box around the subregion and only draw
    // this stuff if within it. Or, we could scale the physical quad drawing this down to the
    // subregion and thten just draw in there (this'd involve changing the UV coordinates, I'm sure
    // macroquad has a way to do this - equally, if we didn't scale them but instead scaled the
    // warp points by the respective x/y scale to match the subregion-space, it'd probably mean we
    // could remove the decimal instability fix in the invBilinear function)

    // Note that because of the weirdness in Macroquad's Mat4 vs Nalgebra's Mat4, the warpPoints matrix is transposed
    // Outlines the four points that make up a warped region in base
    vec2 offsetP1 = vec2(warpPoints[0][0], warpPoints[0][1]);
    vec2 offsetP2 = vec2(warpPoints[1][0], warpPoints[1][1]);
    vec2 offsetP3 = vec2(warpPoints[2][0], warpPoints[2][1]);
    vec2 offsetP4 = vec2(warpPoints[3][0], warpPoints[3][1]);

    // Outlines the warped region's original points
    vec2 baselineP1 = vec2(warpPoints[0][2], warpPoints[0][3]);
    vec2 baselineP2 = vec2(warpPoints[1][2], warpPoints[1][3]);
    vec2 baselineP3 = vec2(warpPoints[2][2], warpPoints[2][3]);
    vec2 baselineP4 = vec2(warpPoints[3][2], warpPoints[3][3]);

    vec4 overlayColor = vec4(0.0, 0.0, 0.0, 0.0);

    float relativeHalfGridWidth = distance(offsetP1, offsetP4) * HALF_GRID_WIDTH;

    #ifdef DRAW_DEBUG_OVERRIDE
    if(drawDebug > 0)
    {
      //float quadDist = 100.0;// get rid of it quadrilateralSDF(uv, offsetP1, offsetP2, offsetP3, offsetP4, relativeHalfGridWidth);

      float lineMult = 0.3;
      float correspondenceDist = 
             min(
              min( lineSDF(uv, offsetP1, baselineP1, relativeHalfGridWidth*lineMult), lineSDF(uv, offsetP2, baselineP2, relativeHalfGridWidth*lineMult)),
              min( lineSDF(uv, offsetP3, baselineP3, relativeHalfGridWidth*lineMult), lineSDF(uv, offsetP4, baselineP4, relativeHalfGridWidth*lineMult))
             );
      //correspondenceDist = 100.0;// TEMP REMOVE

      float pointSize = relativeHalfGridWidth * 0.3;
      vec2 DOWN = vec2(0.0, pointSize*1.5);
      vec2 RIGHT = vec2(pointSize*1.5, 0.0);
      vec2 iOffsetP1 = offsetP1 + DOWN + RIGHT;
      vec2 iOffsetP2 = offsetP2 + DOWN - RIGHT;
      vec2 iOffsetP3 = offsetP3 - DOWN + RIGHT;
      vec2 iOffsetP4 = offsetP4 - DOWN - RIGHT;
      float pointDist =
      min(
        quadPointsSDF(uv, offsetP1, offsetP2, offsetP3, offsetP4, pointSize*0.5), // Actual points
        quadPointsSDF(uv, iOffsetP1, iOffsetP2, iOffsetP3, iOffsetP4, pointSize) // Inset points
      );

      overlayColor = vec4(debugColor, 1.0-step(0.0, min(correspondenceDist,pointDist)));
    }
    #endif

    //// Now draw the actual warped subsegment
    vec4 warpedSegmentColor = vec4(0.0, 0.0, 0.0, 0.0);

    vec2 uvInOffsetSegmentSpace = invBilinear(uv, offsetP1, offsetP2,
                                                  offsetP3, offsetP4);

    if(uvInOffsetSegmentSpace.x >= 0.0 && uvInOffsetSegmentSpace.x <= 1.0 &&
       uvInOffsetSegmentSpace.y >= 0.0 && uvInOffsetSegmentSpace.y <= 1.0)
    {
        //// Convert offset segment space to baseline space
        vec2 baselineCoords = bilinear(uvInOffsetSegmentSpace, baselineP1, baselineP2,
                                                               baselineP3, baselineP4);

        warpedSegmentColor = texture2D(offsetImage, baselineCoords);
        warpedSegmentColor.a = offsetOpacity;

        // Debug
        //warpedSegmentColor = vec4(uvInOffsetSegmentSpace, 0.0f, offsetOpacity);

        // Draw the segment outline
        if(uvInOffsetSegmentSpace.x < relativeHalfGridWidth || uvInOffsetSegmentSpace.x > 1.0-relativeHalfGridWidth ||
           uvInOffsetSegmentSpace.y < relativeHalfGridWidth || uvInOffsetSegmentSpace.y > 1.0-relativeHalfGridWidth)
        {
            warpedSegmentColor = vec4(1.0, 0.0, 0.0, 1.0);
        }
    }

    //// Combine overlay and warped images
    fragColor = mix(warpedSegmentColor, overlayColor, overlayColor.a);
}
