// Module that contains the WarpSet struct and its implementation.
//
// A WarpSet is a structure that goes from a set of four points in the offset image-space to a set
// of four points in baselinespace.

use log::{debug, info, warn, error, log_enabled, Level};
use na::{Vector2,Vector3,Matrix3,Matrix4,Matrix3x4,Matrix2x4, OMatrix, Dyn, MatrixView, Matrix};
use macroquad::math::{Mat4,Vec4,Vec2};
use macroquad::texture::Image;
use macroquad::prelude::Color;
use ::rand::prelude::*;
use std::ops::{Index};
use std::fs::File;
use std::io::Write;

use std::sync::{Arc, Mutex};
use std::thread; // For thread sleeping
use std::time::Duration;

type UPoint = (usize, usize);

#[derive(Debug,Clone)]
pub struct WarpSet/*<T>*/ // TODO: Genericize this
{
    // Contains an entire N-subregion warp set to represent one space (offset space) in a different
    // space (baseline space). This is done by storing a list of subregions in baseline space that
    // refere to an equally-distributed square of subregions in offset space.
    warped_points: OMatrix<Vector2<f32>, Dyn, Dyn>, // Points in baseline-space that correspond to a grid of uniform points in offset space
    mesh_size: usize, // Stores the size of the mesh of points that makes up the region grid
    grid_size: usize, // Stores the size of the grid of regions (note: this will be 1 less than the mesh_size)
    grid_size_vector_f: Vector2<f32>, // Stores the grid size as a vector for easy use
}

// TODO:
//  - new should probably return a Result instead of an Option?
//  - Probably overthinking this. It's not a library, it's just a program to do the thing.
//  - Genericize the WarpSet struct to work either with f32s or f64s

impl WarpSet
{
    pub fn print_warped_points(&self)
    {
        println!("");
        for y in 0..self.mesh_size
        {
            let mut line = "  ".to_string();
            for x in 0..self.mesh_size
            {
                let point = self.warped_points[(y,x)];
                line.push_str(&format!("[{:.2} , {:.2}]{}", point.x, point.y, if (x == self.mesh_size-1){""}else{" -- "}));
            }
            println!("{}",line);
            if(y != self.mesh_size-1)
            {
                let mut line = String::new();
                for _ in 0..self.mesh_size
                {
                    line.push_str("        |        ");
                }
                println!("{}",line);
            }
        }
        println!("");
    }

    // Constructor that just takes the mesh of points, generates auxillary variables
    #[inline(always)]
    fn from_OMatrix(warped_points: OMatrix<Vector2<f32>, Dyn, Dyn>) -> WarpSet
    {
        let mesh_size = warped_points.shape().0;
        let grid_size = mesh_size-1;
        WarpSet{
            mesh_size: mesh_size,
            grid_size: grid_size,
            grid_size_vector_f: Vector2::new(grid_size as f32, grid_size as f32),
            warped_points: warped_points,
        }
    }
    
    // Creates a new warp set from a set of warp points given in row-major order
    // After this, you can call score() to calculate the score of this set of warp points.
    pub fn new(warped_points: &Vec<Vector2<f32>>) -> Option<WarpSet>
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
        let baseline_space_points_column_major: Vec<Vector2<f32>> = 
            (0..mesh_size).map(|i| (0..mesh_size)
            .map(|o| warped_points[i + o * mesh_size])
            .collect::<Vec<Vector2<f32>>>()).flatten().collect();

        Some(
            Self::from_OMatrix(
                OMatrix::<Vector2<f32>, Dyn, Dyn>::from_vec(mesh_size, mesh_size, baseline_space_points_column_major)
                )
            )
    }

    pub fn get_grid_size(&self) -> usize
    {
        self.grid_size
    }

    pub fn get_subdivided(&mut self) -> WarpSet
    {
        let new_mesh_size = self.mesh_size * 2 - 1;
        let mut new_baseline_space_points = OMatrix::<Vector2<f32>, Dyn, Dyn>::zeros(new_mesh_size, new_mesh_size);

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
                new_baseline_space_points[(y*2+1, x*2+1)] = (self.warped_points[(y+1,x)] + self.warped_points[(y,x+1)]) / 2.0;
                new_baseline_space_points[((y+1)*2, x*2+1)] = (self.warped_points[(y+1,x)] + self.warped_points[(y+1,x+1)]) / 2.0;
                new_baseline_space_points[(y*2+1, (x+1)*2)] = (self.warped_points[(y,x+1)] + self.warped_points[(y+1,x+1)]) / 2.0;
            }
        }

        Self::from_OMatrix(new_baseline_space_points)

        //self.warped_points = new_baseline_space_points;
        //self.mesh_size = new_mesh_size;
    }

    // I'd love to have used Mat4x2s here, but glam (which macroquad uses) doesn't support them.
    // I suppose adding them could be a good task for another day.
    // For now, I'll generate the local offset-space points right here.
    //pub fn get_vector_of_mat4s(&self) -> Vec<(Mat4, Vec2)>
    pub fn get_vector_of_mat4s(&self) -> Vec<Mat4>
    {
        let mut out = Vec::new();
        let bsp = &self.warped_points; // Shorthand for the baseline points
        let region_size = 1.0 / ((self.grid_size) as f32); // Size of each region
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
                out.push(
                    Mat4::from_cols_slice(&[
                    bsp[(y,x)].x,     bsp[(y,x)].y,     region_points[0].x, region_points[0].y,
                    bsp[(y,x+1)].x,   bsp[(y,x+1)].y,   region_points[1].x, region_points[1].y,
                    bsp[(y+1,x)].x,   bsp[(y+1,x)].y,   region_points[2].x, region_points[2].y,
                    bsp[(y+1,x+1)].x, bsp[(y+1,x+1)].y, region_points[3].x, region_points[3].y
                    ])
                );
            }
        }
        out
    }

    pub fn get_region_size(&self) -> f32
    {
        1.0 / ((self.grid_size) as f32)
    }

    pub fn mutate(&mut self, mut_rate :f32, mut_scale :f32) -> ()
    {
        let mut rng = thread_rng();

        // Create a randomised list of indice pairs within the set to jump between
        let mut indices: Vec<usize> = (0..(self.mesh_size*self.mesh_size)).collect::<Vec<usize>>();
        indices.shuffle(&mut rng);
        let indices: Vec<UPoint> = indices.iter().map(|n| (n/self.mesh_size, n%self.mesh_size)).collect();

        for index in indices
        {
            
            // Only mutate the point by mut_rate chance
            if rng.gen::<f32>() >= mut_rate
            {
                continue;
            }
            
            // Take a backup of the original point in case the mutation forms a concave quad
            //let backup = self.warped_points[index].clone();

            // TODO: Maybe we should draw a polygon around the point, then calculate the
            // largest inscribed circle from the center by taking the distance to each line in
            // the polygon. That would make more sense.

            // Form a polygon around the point
            let upper_bound:usize = self.mesh_size-1;
            const lower_bound:usize = 0;
            let (x,y) = index;
            let mut edges: Vec<(UPoint, UPoint)> = Vec::new();
            if(x<upper_bound)
            {
                // Right-hand edges
                let rightmost = (x+1,y);
                if(y>lower_bound)
                {
                    // Top to right
                    edges.push(((x,y-1), rightmost));
                }
                if(y<upper_bound)
                {
                    // Right to bottom
                    edges.push((rightmost, (x,y+1)));
                }
            }
            if(x>lower_bound)
            {
                // Left-hand edges
                let leftmost = (x-1,y);
                if(y>lower_bound)
                {
                    // Left to top
                    edges.push((leftmost, (x,y-1)));
                }
                if(y<upper_bound)
                {
                    // Bottom to left
                    edges.push(((x,y+1), leftmost));
                }
            }

            // Calculate the minimum squared distance to each line
            let point = self.warped_points[index];
            let mut shortest_distance_squared = f32::MAX;
            for edge in edges
            {
                let edge_point = self.warped_points[edge.0];
                let edge_direction = (self.warped_points[edge.1] - edge_point).normalize();
                let point_to_edge_point = edge_point - point;
                shortest_distance_squared = shortest_distance_squared.min((point_to_edge_point - point_to_edge_point.dot(&edge_direction)*edge_direction).magnitude_squared());
            }

            // Generate a new point within the maximum circle from the old point (scaled by
            // mut_scale). Note that the new position will never form concave subregions.
            let radius = rng.gen::<f32>() * shortest_distance_squared.sqrt() * mut_scale;
            let angle = rng.gen::<f32>() * std::f32::consts::TAU;
            self.warped_points[index] = (point + Vector2::<f32>::new(angle.cos() * radius, angle.sin() * radius))
                                        .inf(&Vector2::<f32>::new(1.0, 1.0))
                                        .sup(&Vector2::<f32>::new(0.0, 0.0));

            // Clamp to between 0 and 1 so that the points are within the baseline space
            // (This'll never make it go outside of the bounds that we set with the distance
            // calcs)
            //self.warped_points[(y, x)].x = self.warped_points[(y, x)].x.min(1.0).max(0.0);
            //self.warped_points[(y, x)].y = self.warped_points[(y, x)].y.min(1.0).max(0.0);
        }
        
    }

    fn is_concave_region(&self, p1 : &Vector2<f32>, p2 : &Vector2<f32>, p3 : &Vector2<f32>, p4 : &Vector2<f32>) -> bool
    {
        fn cross2d(a :&Vector2<f32>, b :&Vector2<f32>) -> f32
        {
            a.x * b.y - a.y * b.x
        }

        let edge1 = p2 - p1;
        let edge2 = p3 - p2;
        let edge3 = p4 - p3;
        let edge4 = p1 - p4;

        return cross2d(&edge1, &edge2) < 0.0
            || cross2d(&edge2, &edge3) < 0.0
            || cross2d(&edge3, &edge4) < 0.0
            || cross2d(&edge4, &edge1) < 0.0;
    }

    // Takes baseline and offset segmentation images and scores this warp set against their
    // alignment. Each pixel of the offset image is warped to it's corresponding pixel in the
    // baseline image and the total difference between the two is calculated.
    // Note that the offset regions will always be within the baseline image space's bounds,
    // allowing this to work.
    // Pixel-pixel difference (length) is the euclidian length between the two rgba values, meaning
    // that colour images can be used if 4 values are required
    // TODO: Make this optional using generics for 4d vs 1d and also score resolution (f32 vs f64)
    // sqrt(sum( length(baseline[x,y] - offset[x,y])^2 ) / num_pixels)
    // In reality just the summed differences are used, as they are just used to rank relatively
    pub fn score(&self, offset_img: &Image, baseline_img: &Image) -> f32
    {
        let mut score = 0.0;
        let o_img_size_i: Vector2<u32> = Vector2::new(offset_img.width() as u32, offset_img.height() as u32);
        let o_img_size_f: Vector2<f32> = o_img_size_i.map(|n| n as f32);
        let b_img_size_i: Vector2<u32> = Vector2::new(baseline_img.width() as u32, baseline_img.height() as u32);
        let b_img_limit_i: Vector2<u32> = b_img_size_i - Vector2::new(1, 1);
        let b_img_size_f: Vector2<f32> = b_img_size_i.map(|n| n as f32); //Vector2::new(baseline_img.width() as f32, baseline_img.height() as f32);
        let half_pixel: Vector2<f32> = Vector2::new(0.5, 0.5);
        // TODO: Make this threaded
        for y in 0..o_img_size_i.y
        {
            for x in 0..o_img_size_i.x
            {
                // Get the offset-space pixel colour
                let offset_pixel = offset_img.get_pixel(x,y);

                // Skip the pixel if it's transparent.
                if(offset_pixel.a < 0.5)
                {
                    continue;
                }

                // Calculate the point at the center of this pixel
                let p: Vector2<f32> = (Vector2::new(x as f32, y as f32) + half_pixel).component_div(&o_img_size_f);

                // Convert to baselinespace and then back to a pixel coordinate
                let baseline_space_p = self.get_offset_point_in_baseline_space(&p);
                //let mut debug_vector = Vector3::new(baseline_space_p.x, baseline_space_p.y, 0.0);
                let baseline_space_p: Vector2<u32> = baseline_space_p.component_mul(&b_img_size_f).map(|n| n as u32);
                                                     //.inf(&b_img_limit_i); // If in the future
                                                     //there's an issue with the mutation
                                                     //generation, this'll definitely help to do.
                                                     //(along with  -z m to see the bugs)
                
                // Get the baselinespace pixel colour
                let baseline_pixel = baseline_img.get_pixel(baseline_space_p.x, baseline_space_p.y);

                // Compute the squared difference and add it to the score
                score += Self::pixel_diff_1d(&baseline_pixel, &offset_pixel);
            }
        }
        score
        //(score/(o_img_size_i.x * o_img_size_i.y) as f32)//.sqrt()
    }

    //fn pixel_diff<4D> etc, etc...
    fn pixel_diff_4d(p1: &Color, p2: &Color) -> f32
    {
        (p1.to_vec() - p2.to_vec()).length_squared()
    }
    fn pixel_diff_1d(p1: &Color, p2: &Color) -> f32
    {
        // Grey-scale only version:
        (p1.r - p2.r).powf(2.0)
    }

    // Takes in a Vector2 in offset image space (between (0,0) and (1,1)) and returns the corresponding
    // Vector2 in baseline image space (usually between (0,0) and (1,1)), utilising the warped_points
    #[inline(always)]
    pub fn get_offset_point_in_baseline_space(&self, p : &Vector2<f32>) -> Vector2<f32>
    {
        //// First work out the subregion the point is in
        //let subregion_id:Vector2<usize> = Vector2::new(
        //    (p.x * (self.grid_size) as f32) as usize,
        //    (p.y * (self.grid_size) as f32) as usize
        //);

        //// Then work out the offset within that subregion
        //// (region space)
        //let uv = Vector2::new(
        //    (p.x * (self.grid_size) as f32) - subregion_id.x as f32,
        //    (p.y * (self.grid_size) as f32) - subregion_id.y as f32
        //);
        
        // The above, but using in-line commands. Assuming this might be more efficient because it
        // leans on nalgebera's logic, but maybe it isn't?

        // NOTE: I'm not actually sure using grid_size_vector_f here is faster (I definitely need
        // to test over larger images, where in theory it should be faster).
        let subregion: Vector2<f32> = p.component_mul(&self.grid_size_vector_f);// Normalise the point to the subregion space
        let subregion_id: Vector2<usize> = subregion.map(|n| n as usize); // Discard the decimals
        let uv: Vector2<f32> = subregion - subregion_id.map(|n| n as f32); // Just the decimals

        // Then work out the baseline space points for the subregion
        //debug!("\np: {}\nSubregion ID: {}\nbaseline_space_points: {}", p, subregion_id, self.warped_points);
        let p1 = self.warped_points[(subregion_id.y, subregion_id.x)];
        let p2 = self.warped_points[(subregion_id.y, subregion_id.x+1)];
        let p3 = self.warped_points[(subregion_id.y+1, subregion_id.x)];
        let p4 = self.warped_points[(subregion_id.y+1, subregion_id.x+1)];

        // Then bilinearly interpolate the point between the four projection points
        p1 + (p2-p1)*uv.x + (p3-p1)*uv.y + (p1-p2+p4-p3)*uv.x*uv.y
    }

    pub fn write_to_file(&self, path: &str) -> ()
    {
        info!("Writing warpmesh to file: {}...", path);
        let mut file = File::create(path).unwrap();
        let mut mesh_string = String::new();
        for p in self.warped_points.transpose().iter()
        {
            mesh_string.push_str(&format!("({},{}),", p.x, p.y));
        }
        mesh_string.pop(); // Remove the last comma
        file.write_all(mesh_string.as_bytes()).unwrap();
        info!("Warpmesh file written.");
    }
}
