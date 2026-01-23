use log::{//debug,
          info,
          //warn,
          error,
          //log_enabled,
          //Level
         };
use na::{Vector2,
         //Vector3,
         //Matrix3,
         //Matrix4,
         //Matrix3x4,
         //Matrix2x4,
         OMatrix,
         Dyn,
         //MatrixView,
         //Matrix
        };
use num_traits::float::Float;

use macroquad::prelude::*;
use crate::miniquad::{BlendState, Equation, BlendFactor, BlendValue};
use macroquad::prelude::ShaderSource;
use std::fs::{File, read_to_string};
use std::io::Read;
use std::io::Write;

const UI_HEIGHT:f32 = 5.0;
const WARPMESH_STACK_HEIGHT:f32 = 1.0;
const WARPMESH_STACK_WIDTH:f32 = 1.0;


#[derive(Debug, Clone)]
pub struct WarpSet
{
    // Contains an entire N-subregion warp set to represent one space (offset space) in a different
    // space (baseline space). This is done by storing a list of subregions in baseline space that
    // refere to an equally-distributed square of subregions in offset space.
    warped_points: OMatrix<Vector2<f64>, Dyn, Dyn>, // Points in baseline-space that correspond to a grid of uniform points in offset space
    mesh_size: usize, // Stores the size of the mesh of points that makes up the region grid, only touched on creation
    grid_size: usize, // Stores the size of the grid of regions (note: this will be 1 less than the mesh_size), only touched on creation
    subregion_count: f32, // Stores the number of subregions (aka grid_size^2), only touched on creation
    closest_point_index: (usize, usize), // Stores the index of the point currently closest to the cursor
    closest_point_offset: Vector2<f64>,  // stores the offset from the mouse to the closest point
    vector_of_mat_fours: Vec<Mat4>, // Stores/chaches the Mat4s that make up each subregion when
                                    // used in drawing. Updated whenver a point is moved.
    warp_shader: Material,
    drag_happening: bool, // Keeps track of whether a drag is happening

    pub offset_opacity: f32, // How opaque the offset is
}

const FRAGMENT_SHADER: &'static str = include_str!("display_shader.frag");
const VERTEX_SHADER: &'static str = include_str!("display_shader.vert");

impl WarpSet
{

    //// Contsructors and creators

    // Constructor that just takes the mesh of points, generates auxillary variables
    #[inline(always)]
    #[allow(non_snake_case)]
    fn from_OMatrix(warped_points: OMatrix<Vector2<f64>, Dyn, Dyn>, offset_image: & Texture2D) -> WarpSet
    {
        // Create the offset warp display shader
        let offset_display_material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX_SHADER,
                fragment: FRAGMENT_SHADER,
            },
            MaterialParams {
                uniforms: vec![
                               UniformDesc::new("warpPoints", UniformType::Mat4),
                               UniformDesc::new("offsetOpacity", UniformType::Float1),
                              ],
                textures: vec![
                                "offsetImage".to_owned(),
                              ],
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                ..Default::default()
            }
        ).unwrap();
        
        offset_display_material.set_texture("offsetImage", offset_image.weak_clone());

        let mesh_size = warped_points.shape().0;
        let grid_size = mesh_size-1;
        let mut ws = WarpSet{
            mesh_size: mesh_size,
            grid_size: grid_size,
            subregion_count: grid_size as f32 * grid_size as f32,
            warped_points: warped_points,
            closest_point_index: (0,0),
            closest_point_offset: Vector2::new(0., 0.),
            vector_of_mat_fours: Vec::<Mat4>::new(),
            warp_shader: offset_display_material,
            drag_happening: false,

            offset_opacity: 1.0,
        };
        ws.update_mat_fours(); // Calculate mat4s
        ws
    }
    
    // Creates a new warp set from a set of warp points given in row-major order
    pub fn new(warped_points: &Vec<Vector2<f64>>, offset_image: & Texture2D) -> Option<WarpSet>
    {
        // Make sure that the given point list is the correct length (aka a square number)
        let point_count = warped_points.len();
        if (point_count as f32).sqrt().fract() != 0.0
        {
            error!("The given point lists are not a square number");
            return None;
        }

        let mesh_size = (point_count as f32).sqrt() as usize;
        
        // Create the list of points in column major order
        let baseline_space_points_column_major: Vec<Vector2<f64>> = 
            (0..mesh_size).map(|i| (0..mesh_size)
            .map(|o| warped_points[i + o * mesh_size])
            .collect::<Vec<Vector2<f64>>>()).flatten().collect();

        Some(
            Self::from_OMatrix(
                OMatrix::<Vector2<f64>, Dyn, Dyn>::from_vec(mesh_size, mesh_size, baseline_space_points_column_major),
                offset_image
                )
            )
    }

    // Creates a new warpset from a file
    pub fn from_file(filepath: &str, offset_image: & Texture2D) -> Option<WarpSet>
    {
        println!("Reading file \"{}\"...", filepath);
        let file_contents = read_to_string(filepath).expect("Couldn't load warp file file from given filepath.");
        let vector = get_start_points_from_string(file_contents).expect("Start points malformed.");
        Self::new(&vector, offset_image)
    }

    // Subdivides this mesh
    pub fn subdivide(&mut self) -> ()
    {
        let new_mesh_size = self.mesh_size * 2 - 1;
        let mut new_baseline_space_points = OMatrix::<Vector2<f64>, Dyn, Dyn>::zeros(new_mesh_size, new_mesh_size);

        // Populate the current points into the new matrix
        for y in 0..self.mesh_size
        {
            for x in 0..self.mesh_size
            {
                new_baseline_space_points[(y*2, x*2)] = self.warped_points[(y,x)];
            }
        }

        // Interpolate the new points
        for y in 0..self.grid_size
        {
            for x in 0..self.grid_size
            {
                new_baseline_space_points[(y*2+1, x*2)] = (self.warped_points[(y,x)] + self.warped_points[(y+1,x)]) / 2.0;
                new_baseline_space_points[(y*2, x*2+1)] = (self.warped_points[(y,x)] + self.warped_points[(y,x+1)]) / 2.0;
                new_baseline_space_points[((y+1)*2, x*2+1)] = (self.warped_points[(y+1,x)] + self.warped_points[(y+1,x+1)]) / 2.0;
                new_baseline_space_points[(y*2+1, (x+1)*2)] = (self.warped_points[(y,x+1)] + self.warped_points[(y+1,x+1)]) / 2.0;
                // The midpoint needs to be at the center of the new points
                new_baseline_space_points[(y*2+1, x*2+1)] = 
                    (
                    (new_baseline_space_points[(y*2+1, x*2)] + new_baseline_space_points[(y*2, x*2+1)])/2. + 
                    (new_baseline_space_points[((y+1)*2, x*2+1)] + new_baseline_space_points[(y*2+1, (x+1)*2)])/2.
                    )/2.;
            }
        }

        // Update internal variables
        self.warped_points = new_baseline_space_points;
        self.mesh_size = new_mesh_size;
        self.grid_size = self.mesh_size-1;
        self.subregion_count = self.grid_size as f32*2.;
        self.update_mat_fours();
    }

    //// General getters/setters
    /*
    pub fn get_grid_size(&self) -> usize
    {
        self.grid_size
    }
    */


    //// Points UI
    // Takes in the mouse pointer, in real-space,
    // Marks the point closest to it.
    pub fn update_closest_point(&mut self, mouse_pos:&Vec2, drag_down:bool) -> ()
    {
        self.drag_happening = drag_down; // Just keep track of this for use in rendering

        let mouse_pos = Vector2::<f64>::new(mouse_pos.x.into(), mouse_pos.y.into());
        
        // We only care about finding the closest point if we're not dragging
        if drag_down
        {
            // Move the point if it's being dragged and the new point doesn't form a concave region
            self.set_and_check_new_point_pos(self.closest_point_index, mouse_pos + self.closest_point_offset);

        }else{

            // And if we aren't dragging, then we can find it
            let mut closest_distance = f64::MAX;
            for x in 0..self.mesh_size
            {
                for y in 0..self.mesh_size
                {
                    let p = self.warped_points[(x,y)];
                    let diff = p - mouse_pos;
                    let dist = diff.x*diff.x+diff.y*diff.y;
                    if dist < closest_distance
                    {
                        closest_distance = dist;
                        self.closest_point_index = (x, y);
                        self.closest_point_offset = diff;
                    }
                }
            }
        }
    }

    pub fn has_point_been_warped(&self, v: Vector2::<f64>) -> bool
    {
        if !self.drag_happening
        {
            return false;
        }

        let gridsize_vector_f = Vector2::<f64>::new(self.grid_size as f64, self.grid_size as f64);
        let subregion: Vector2<f64> = v.component_mul(&gridsize_vector_f);// Normalise the point to the subregion space
        let subregion_id: Vector2<usize> = subregion.map(|n| n as usize); // Discard the decimals

        // If any of the points in the region that effects this projection point are the closest
        // point (that has been moved), then this projection point has been warped
        (subregion_id.y, subregion_id.x) == self.closest_point_index ||
        (subregion_id.y, subregion_id.x+1) == self.closest_point_index ||
        (subregion_id.y+1, subregion_id.x) == self.closest_point_index ||
        (subregion_id.y+1, subregion_id.x+1) == self.closest_point_index
    }

    // Takes a point and attempts to move it, ensures convexity
    fn set_and_check_new_point_pos(&mut self, index:(usize, usize), new_pos:Vector2::<f64>) -> bool
    {
        // Quit out if we're trying to drag the points outside of the baseline region.
        //if new_pos.x < 0. || new_pos.x > 1. || new_pos.y < 0. || new_pos.y > 1.
        //{
        //    return false;
        //}
        //// Actually, we want to be able to place the points outside...

        let (x, y) = index;
        let x = x as i32;
        let y = y as i32;

        // Temporarily set the warped points to include the new point
        let previous_point = self.warped_points[self.closest_point_index];
        self.warped_points[self.closest_point_index] = new_pos;

        let is_convex = self.is_convex((x,y),      // bottom-right
                                       (x+1, y  ),
                                       (x+1, y+1),
                                       (x,   y+1),
                                      ) &&
                        self.is_convex((x-1, y),   // bottom-left
                                       (x,y),
                                       (x,   y+1),
                                       (x-1, y+1),
                                      ) &&
                        self.is_convex((x-1, y-1), // Top-left
                                       (x,   y-1),
                                       (x,y),
                                       (x-1, y),
                                      ) &&
                        self.is_convex((x,   y-1), // Top-right
                                       (x+1, y-1),
                                       (x+1, y),
                                       (x,y),
                                      );
        
        if is_convex
        {
            // If it's still convex, then all is well with the world and carry on
            // (after updating the mat4s)
            self.update_mat_fours();
            return true
        }else{
            // If it's not still convex, then revert the change
            self.warped_points[self.closest_point_index] = previous_point;
            return false
        }
    }

    // Checks the convexity of a quad
    fn is_convex(&mut self, p1:(i32,i32), p2:(i32,i32), p3:(i32,i32), p4:(i32,i32)) -> bool
    {

        let point_indices: [(i32, i32); 4] = [p1, p2, p3, p4];
        let mut points: [Vector2::<f64>; 4] = [Default::default(); 4];
        
        // First check if any point is out of bounds.
        // If it is, then this quad is actually just a point or a line and therefor convex by default.
        for i in 0..4
        {
            if point_indices[i].0 < 0 || point_indices[i].0 as usize > self.grid_size
            || point_indices[i].1 < 0 || point_indices[i].1 as usize > self.grid_size
            {
                return true
            }else {
                let point_index = point_indices[i];
                points[i] = self.warped_points[(point_index.0 as usize, point_index.1 as usize)];
            }
        }

        //// Now compute the convexity
        for i in 0..4
        {
            // get points
            let p0 = points[i];
            let p1 = points[(i+1)%4];
            let p2 = points[(i+2)%4];

            // get edges
            let edge0 = p0 - p1;
            let edge1 = p2 - p1;

            // calculate cross product edge0 x edge1
            let cross = edge0.x * edge1.y - edge0.y * edge1.x;
            if cross <= 0.
            {
                return false
            }
        }

        // Return true if everything's convex
        true
    }

    fn update_mat_fours(&mut self) -> ()
    {
        self.vector_of_mat_fours = Vec::new();
        let bsp = &self.warped_points; // Shorthand for the baseline points
        let region_size = 1.0 / ((self.grid_size) as f32); // Size of each region in baseline space
        for y in 0..self.grid_size
        {
            for x in 0..self.grid_size
            {
                // Calculate the four points in the offset space that form the current region
                let region_points = [
                    Vector2::new(x as f32 * region_size, y as f32 * region_size),
                    Vector2::new((x+1) as f32 * region_size, y as f32 * region_size),
                    Vector2::new(x as f32 * region_size, (y+1) as f32 * region_size),
                    Vector2::new((x+1) as f32 * region_size, (y+1) as f32 * region_size),
                ];
                self.vector_of_mat_fours.push(
                    Mat4::from_cols_slice(&[
                    bsp[(y,x)].x as f32,     bsp[(y,x)].y as f32,     region_points[0].x as f32, region_points[0].y as f32,
                    bsp[(y,x+1)].x as f32,   bsp[(y,x+1)].y as f32,   region_points[1].x as f32, region_points[1].y as f32,
                    bsp[(y+1,x)].x as f32,   bsp[(y+1,x)].y as f32,   region_points[2].x as f32, region_points[2].y as f32,
                    bsp[(y+1,x+1)].x as f32, bsp[(y+1,x+1)].y as f32, region_points[3].x as f32, region_points[3].y as f32
                    ])
                );
            }
        }
    }
    
    //// Projection
    pub fn map_point_from_offset_to_baseline<F:Float + std::fmt::Debug>(&self, p: &Vector2<F>) -> Vector2<f64>
    {
        // TODO: Genericise the use of the input vector type
        //let gridsize_vector_f = Vector2::<F>::new(&self.grid_size as F, &self.grid_size as F);
        
        let p = Vector2::<f64>::new(p[0].to_f64().expect("failed to cast to f64"),
                                    p[1].to_f64().expect("failed to cast to f64")
                                   );

        let gridsize_vector_f = Vector2::<f64>::new(self.grid_size as f64, self.grid_size as f64);
        let subregion: Vector2<f64> = p.component_mul(&gridsize_vector_f);// Normalise the point to the subregion space
        let subregion_id: Vector2<usize> = subregion.map(|n| n as usize); // Discard the decimals
        let uv: Vector2<f64> = subregion - subregion_id.map(|n| n as f64); // Just the decimals
    
        // Then work out the baseline space points for the subregion
        //debug!("\np: {}\nSubregion ID: {}\nbaseline_space_points: {}", p, subregion_id, self.warped_points);
        let p1 = self.warped_points[(subregion_id.y, subregion_id.x)];
        let p2 = self.warped_points[(subregion_id.y, subregion_id.x+1)];
        let p3 = self.warped_points[(subregion_id.y+1, subregion_id.x)];
        let p4 = self.warped_points[(subregion_id.y+1, subregion_id.x+1)];

        // Then bilinearly interpolate the point between the four projection points
        p1 + (p2-p1)*uv.x + (p3-p1)*uv.y + (p1-p2+p4-p3)*uv.x*uv.y
    }
    // Takes a point in baseline-space, converts it to offset space
    // (aka it performs those horrible inverse-transforms)
    // Shouldn't be trusted to return highly accurate results
    pub fn map_point_from_baseline_to_offset(&self, p: &Vector2<f64>) -> Option<Vector2<f64>>
    {
        // Check each subregion to see if p is within it
        for x in 0..self.grid_size
        {
            for y in 0..self.grid_size
            {
                // Get the 4 relevant points
                let warp_p1 = self.warped_points[(x,y)];
                let warp_p2 = self.warped_points[(x+1,y)];
                let warp_p3 = self.warped_points[(x,y+1)];
                let warp_p4 = self.warped_points[(x+1,y+1)];
                
                // Compute the uv in this subregion
                let uv = Self::inv_bilinear(p, warp_p1, warp_p3,
                                              warp_p2, warp_p4);
                if uv.is_none() { continue; }
                let uv = uv.unwrap();

                if uv.x < 0. || uv.y < 0.
                {
                    continue;
                }

                // If it's within range then compute it's proper location within offsetspace by
                // adding the space taken up by the other patches.
                let offset = Vector2::<f64>::new(y as f64, x as f64);
                return Some((offset+uv)/self.grid_size as f64);
            }
        }
        None
    }
    
    // Calculates the inverse bilinear interpolation between the 4 points, an edited version of the
    // function from demonstrated by Inigo Quilez at https://iquilezles.org/articles/ibilinear/
    // HackerHam Version:
    // https://gist.github.com/ivanpopelyshev/2a75479075286deb8ee5dc1fb2e07f09
    pub fn inv_bilinear(p: &Vector2<f64>, a: Vector2<f64>, b: Vector2<f64>, d: Vector2<f64>, c: Vector2<f64>) -> Option<Vector2<f64>>
    {
        fn cross2d(a: &Vector2<f64>, b: &Vector2<f64>) -> f64
        {
            a.x*b.y - a.y*b.x
        }
    
        let e = b-a;
        let f = d-a;
        let g = a-b+c-d;
        let h = p-a;
    
        // Get a measure of the size of the region that we
        // can scale everything by to avoid rounding errors
            
        let k2 = cross2d( &g, &f );
        let k1 = cross2d( &e, &f ) + cross2d( &h, &g );
        let k0 = cross2d( &h, &e );

        let k2u = cross2d(&e, &g);
        let k1u = cross2d(&e, &f) + cross2d(&g, &h);
        let k0u = cross2d(&h, &f);

        let mut v1 = 0.0;
        let u1;
        let mut v2 = 0.0;
        let mut u2 = 0.0;
        
        // if edges are parallel, this is a linear equation
        const EPSILON: f64 = 0.00001;
        if k2.abs() < EPSILON
        {
            v1 = -k0/k1;

            let denom_x = e.x + g.x * v1;
            let denom_y = e.y + g.y * v1;

            if denom_x.abs() > denom_y.abs()
            {
                u1 = (h.x - f.x*v1) / denom_x;
            }else{
                u1 = (h.y - f.y*v1) / denom_y;
            }
        }
        else if k2u.abs() < EPSILON
        {
            u1 = k0u/k1u;

            let denom_x = f.x + g.x*u1;
            let denom_y = f.y + g.y*u1;

            if denom_x.abs() > denom_y.abs()
            {
                v1 = (h.x - e.x*u1) / denom_x;
            }else{
                v2 = (h.y - e.y*u1) / denom_y;
            }
        }
        // otherwise, it's a quadratic
    	else
        {
            let w = k1*k1 - 4.0*k0*k2;

            if w < 0.0 {
                return None;
            }

            let w = w.sqrt();
            v1 = (-k1  - w)/(2.0*k2);
            v2 = (-k1  + w)/(2.0*k2);
            u1 = (-k1u - w)/(2.0*k2u);
            u2 = (-k1u + w)/(2.0*k2u);
        }

        let b1 = v1>0.0 && v1<1.0 && u1>0.0 && u1<1.0;
        let b2 = v2>0.0 && v2<1.0 && u2>0.0 && u2<1.0;

        let mut out = None;
        if  b1 && !b2 { out = Some(Vector2::<f64>::new(u1, v1)); }
        if !b1 &&  b2 { out = Some(Vector2::<f64>::new(u2, v2)); }

        out
    }

    //// Saving
    pub fn write_to_file(&self, path: &str) -> ()
    {
        println!("Writing warpmesh to file: {}...", path);
        let mut file = File::create(path).unwrap();
        let mut mesh_string = String::new();
        for p in self.warped_points.transpose().iter()
        {
            mesh_string.push_str(&format!("({},{}),", p.x, p.y));
        }
        mesh_string.pop(); // Remove the last comma
        file.write_all(mesh_string.as_bytes()).unwrap();
        println!("Warpmesh file written!");
    }

    //// Drawing
    pub fn draw_ui(&self, baseline_size: &Vec2) -> ()
    {
        // It's assumed that the baseline is always 1.0 high.
        let point_xy_scale = vec3(baseline_size.x, baseline_size.y, 1.);
        let point_size_scale = 0.05/self.mesh_size as f32;

        fn two_d_to_three_d(v: Vector2<f64>) -> Vec3
        {
            vec3(v.x as f32, UI_HEIGHT, v.y as f32)
        }
        
        // Draw a cross at the closest node, or a triangle if it's moving
        {
            let p = self.warped_points[self.closest_point_index];
            let p = two_d_to_three_d(p) * point_xy_scale;
            let cross_point_righthanded = vec3(point_size_scale, 0., point_size_scale);
            let cross_point_lefthanded = vec3(-point_size_scale, 0., point_size_scale);
            if self.drag_happening
            {
                // Draw a circle of N sides
                let n = 32;
                let mut prev = vec3(point_size_scale, 0., 0.);
                for i in 1..(n+1)
                {
                    let angle = (i as f32/n as f32) * std::f32::consts::PI*2.;
                    let offset = vec3(angle.cos()*point_size_scale, 0., angle.sin()*point_size_scale);
                    draw_line_3d(p + prev, p + offset, ORANGE);
                    prev = offset;
                }
            }else{
                // Draw a cross
                draw_line_3d(p - cross_point_righthanded, p + cross_point_righthanded, ORANGE);
                draw_line_3d(p - cross_point_lefthanded, p + cross_point_lefthanded, ORANGE);
            }
        }
        
        // Draw the lines between each circle
        for x in 0..self.grid_size
        {
            for y in 0..self.grid_size
            {
                let p0 = two_d_to_three_d(self.warped_points[(x,y)]);
                let p1 = two_d_to_three_d(self.warped_points[(x+1,y)]);
                let p2 = two_d_to_three_d(self.warped_points[(x,y+1)]);
                draw_line_3d(p0 * point_xy_scale,p1 * point_xy_scale,RED);
                draw_line_3d(p0 * point_xy_scale,p2 * point_xy_scale,RED);
            }
        }
        // Draw final lines
        for x in 0..self.grid_size
        {
            let p0 = two_d_to_three_d(self.warped_points[(x,self.grid_size)]);
            let p1 = two_d_to_three_d(self.warped_points[(x+1,self.grid_size)]);
            draw_line_3d(p0 * point_xy_scale,p1 * point_xy_scale,RED);
        }
        for y in 0..self.grid_size
        {
            let p0 = two_d_to_three_d(self.warped_points[(self.grid_size,y)]);
            let p1 = two_d_to_three_d(self.warped_points[(self.grid_size,y+1)]);
            draw_line_3d(p0 * point_xy_scale,p1 * point_xy_scale,RED);
        }
    }

    pub fn draw_warped_offset_image(&self, baseline_plane_size: &Vec2) -> ()
    {
        // The reference to this texture should probably be set somewhere else just once so it's
        // not duplicating it every time, but that's a lot of effort atm.
        self.warp_shader.set_uniform("offsetOpacity", self.offset_opacity);
        
        //// Draw each sub-region by iterating over the warpset
        gl_use_material(&self.warp_shader);
        for (i, subregion_points) in self.vector_of_mat_fours.iter().enumerate()
        {
            // Set the uniforms
            self.warp_shader.set_uniform("warpPoints", *subregion_points);
            // (subregion_points needs to be dereferenced to get the whole matrix of points)


            // Draw the subregion as a slice in vertical space - only the subregion is opaque
            draw_plane(vec3(baseline_plane_size.x,
                            WARPMESH_STACK_HEIGHT + (i as f32/self.subregion_count)*WARPMESH_STACK_WIDTH,
                            0.5),
                       *baseline_plane_size, // The subregion is drawn in respect to the baseline plane size
                       None,
                       BLUE
                      );
        }
        // Go back to the default material
        gl_use_default_material();
    }
}

pub fn get_start_points_from_string(mut start_points_input: String) -> Option<Vec::<Vector2<f64>>>
{
    let mut out = Vec::<Vector2<f64>>::new();

    if start_points_input == ""
    {
        println!("No points provided, using default values (unit square)...");
        out = vec![
            Vector2::new(0.0,0.0),
            Vector2::new(1.0,0.0),
            Vector2::new(0.0,1.0),
            Vector2::new(1.0,1.0)
        ];
    }else{
        println!("Custom start points provided, parsing...");
        if start_points_input.ends_with(".wrp")
        {
            println!("\tWarp mesh file '{}' provided, loading...", start_points_input);
            let mut f = File::open(start_points_input.as_str()).unwrap();
            let mut data = String::new();
            match f.read_to_string(&mut data)
            {
                Ok(_) => {
                    start_points_input = data;
                },
                Err(e) => {
                    error!("Error reading file: {}", e);
                }
            }
        }
        let point_strings = start_points_input.split("),(").collect::<Vec<&str>>();
        for point in point_strings
        {
            let point_str = point.trim().trim_matches(|c| c == '(' || c == ')');
            let vals = point_str.split(",").collect::<Vec<&str>>();
            let x = vals[0].parse::<f64>();
            let y = vals[1].parse::<f64>();
            if vals.len() != 2 || x.is_err() || y.is_err()
            {
                error!("Invalid point \"{}\", found in supplied start points argument \"{}\"",point_str, start_points_input);
                return None;
            }
            out.push(Vector2::new(x.unwrap(),y.unwrap()));
        }
        info!("\t...done.");
    }
    Some(out)
}
