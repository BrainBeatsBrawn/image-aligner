#version 330
precision lowp float;

in vec2 uv;
in vec2 center;

layout(location=0) out vec4 fragColor;

//uniform vec2 mousePos;

uniform mat4 warpPoints; // First row: offset-space xs, 2nd: offset ys, 3rd: baseline xs, 4th: baseline ys
uniform float offsetOpacity; // How transparent the offset image is
//uniform vec3 debugColor; // The color to use for debugging
//uniform int drawDebug; // Whether to draw debug info
//
uniform sampler2D offsetImage; // The image

#define M_PI 3.141592653589793

// Returns the 2D cross product of two vectors
float cross2d(in vec2 a, in vec2 b)
{
    return a.x*b.y - a.y*b.x;
}

// Calculates forward bilinear interpolation between the 4 points
vec2 bilinear(in vec2 uv, in vec2 a, in vec2 b, in vec2 d, in vec2 c)
{
    return a + (b-a)*uv.x + (d-a)*uv.y + (a-b+c-d)*uv.x*uv.y;
}

// Calculates the inverse bilinear interpolation between the 4 points, an edited version of the
// function from demonstrated by Inigo Quilez at https://iquilezles.org/articles/ibilinear/
//vec2 invBilinear( in vec2 p, in vec2 a, in vec2 b, in vec2 d, in vec2 c )
//{
//    vec2 res = vec2(-1.0);
//
//    vec2 e = b-a;
//    vec2 f = d-a;
//    vec2 g = a-b+c-d;
//    vec2 h = p-a;
//        
//    float k2 = cross2d( g, f );
//    float k1 = cross2d( e, f ) + cross2d( h, g );
//    float k0 = cross2d( h, e );
//    
//    // if edges are parallel, this is a linear equation
//    if( abs(k2)<0.001 )
//    {
//        res = vec2( (h.x*k1+f.x*k0)/(e.x*k1-g.x*k0), -k0/k1 );
//    }
//    // otherwise, it's a quadratic
//	else
//    {
//        float w = k1*k1 - 4.0*k0*k2;
//        if( w<0.0 ) return vec2(-1.0);
//        w = sqrt( w );
//
//        float ik2 = 0.5/k2;
//        float v = (-k1 - w)*ik2;
//        float u = (h.x - f.x*v)/(e.x + g.x*v);
//        
//        if( u<0.0 || u>1.0 || v<0.0 || v>1.0 )
//        {
//           v = (-k1 + w)*ik2;
//           u = (h.x - f.x*v)/(e.x + g.x*v);
//        }
//        res = vec2( u, v );
//    }
//    
//    return res;
//}
#define EPSILON 0.0000001
vec2 invBilinear( in vec2 p, in vec2 a, in vec2 b, in vec2 d, in vec2 c )
{
    vec2 res = vec2(-1.0);

    vec2 e = b-a;
    vec2 f = d-a;
    vec2 g = a-b+c-d;
    vec2 h = p-a;
        
    float k2 = cross2d( g, f );
    float k1 = cross2d( e, f ) + cross2d( h, g );
    float k0 = cross2d( h, e );

    float k2u = cross2d(e, g);
    float k1u = cross2d(e, f) + cross2d(g, h);
    float k0u = cross2d(h, f);

    float v1, u1, v2, u2;
    
    // if edges are parallel, this is a linear equation
    if(abs(k2)<EPSILON)
    {
	v1 = -k0/k1;

	float denom_x = e.x + g.x * v1;
	float denom_y = e.y + g.y * v1;

	if(abs(denom_x) > abs(denom_y))
	{
	    u1 = (h.x - f.x*v1) / denom_x;
        }else{
	    u1 = (h.y - f.y*v1) / denom_y;
	}
    }
    else if(abs(k2u) < EPSILON)
    {
	u1 = k0u/k1u;

	float denom_x = f.x + g.x*u1;
	float denom_y = f.y + g.y*u1;

	if(abs(denom_x) > abs(denom_y))
	{
	    v1 = (h.x - e.x*u1) / denom_x;
	}else{
	    v2 = (h.y - e.y*u1) / denom_y;
	}
    }
    else
    {
	float w = k1*k1 - 4.0*k0*k2;

	if (w<0.)
	{
	    return res;
	}

	w = sqrt(w);
	v1 = (-k1  - w)/(2.0*k2);
	v2 = (-k1  + w)/(2.0*k2);
	u1 = (-k1u - w)/(2.0*k2u);
	u2 = (-k1u + w)/(2.0*k2u);
    }

    bool b1 = v1>0. && v1<1. && u1>0. && u1<1.;
    bool b2 = v2>0. && v2<1. && u2>0. && u2<1.;

    if (b1 && !b2) res = vec2(u1, v1);
    if (!b1 && b2) res = vec2(u2, v2);
    
    return res;
}


void main()
{
    //// Draw some overlay debug info
    // By default, we're outside of the subregion and/or the mask, so we just draw a transparent texture
    // (Red, because of course it's red)
    fragColor = vec4(1.0, 0.0, 0.0, 0.0); // Colour it red.

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

    // Get the uv in offset space
    vec4 warpedSegmentColor = vec4(0.0, 0.0, 0.0, 0.0);
    vec2 uvInOffsetSegmentSpace = invBilinear(uv, offsetP1, offsetP2,
                                                  offsetP3, offsetP4);

    if(uvInOffsetSegmentSpace.x >= 0.0 && uvInOffsetSegmentSpace.x <= 1.0 &&
       uvInOffsetSegmentSpace.y >= 0.0 && uvInOffsetSegmentSpace.y <= 1.0)
    {
	fragColor = vec4(0.0, 1.0, 0.0, 0.5);  // Green
        vec2 baselineCoords = bilinear(uvInOffsetSegmentSpace, baselineP1, baselineP2,
                                                               baselineP3, baselineP4);

        fragColor = texture2D(offsetImage, baselineCoords);
    	fragColor.a = min(fragColor.a, offsetOpacity);
    }
}
