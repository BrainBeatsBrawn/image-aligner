use na::Vector2;
use crate::WarpSet;

use macroquad::prelude::*;

use std::f32::consts::PI;

const UI_HEIGHT:f32 = 5.0;
const ARBITRARY_SMALL:f32 = 0.05; // How close are two ends of a line for a green ~~circle~~ square.

//#[derive(Debug, Constructor)]
#[derive(Debug)]
struct Pair
{
    bp: Vector2::<f64>, // Baseline point
    op: Vector2::<f64>, // Offset point
}

#[derive(Debug)]
pub struct Correspondences
{
    pairs: Vec::<Pair>, // Stores list of pairs in (baseline, offset) of corresponding points
    staged_baseline: Option<Vector2::<f64>>,
    staged_offset: Option<Vector2::<f64>>,
}


impl Correspondences
{
    pub fn new() -> Correspondences
    {
        Correspondences{
            pairs: Vec::<Pair>::new(),
            staged_baseline: None,
            staged_offset: None,
        }
    }
    
    // Queues a baseline point, adds the pair if there's already a queued offset point
    pub fn stage_baseline(&mut self, bp: Vector2::<f64>) -> ()
    {
        self.staged_baseline = Some(bp);
    }
    pub fn stage_offset(&mut self, op: Vector2::<f64>) -> ()
    {
        self.staged_offset = Some(op);
    }

    pub fn commit_staged(&mut self) -> ()
    {
        if self.staged_baseline.is_some() && self.staged_offset.is_some()
        {
            self.pairs.push(Pair{
                                 bp:self.staged_baseline.unwrap(), 
                                 op:self.staged_offset.unwrap()
                                }
                           );
            self.staged_baseline = None;
            self.staged_offset = None;
        }
    }

    //// Drawing
    //TODO: Should also take in the size of the camera atm?
    pub fn draw(&self, warpset: &WarpSet, baseline_size: &Vec2, mouse_world_pos: &Vec2) -> ()
    {
        fn vec64_to_vec2(v: Vector2::<f64>) -> Vec2
        {
            vec2(v.x as f32, v.y as f32)
        }

        // First, draw the proposed line if there is one to draw
        if self.staged_baseline.is_some() || self.staged_offset.is_some()
        {
            if self.staged_baseline.is_some()
            {
                // From baseline to mouse
                let bp = self.staged_baseline.unwrap();
                Self::draw_thick_line_between(&(vec64_to_vec2(bp) * (*baseline_size)), mouse_world_pos, 0.1);
            }else{
                // From mouse to offset
            }
        }

        // Then, go and draw all the added lines from the correspondences list
        //let start = vec2(0.,0.);
        //let end = vec2(1.,1.);
        //let thickness = 0.2;
        //Self::draw_thick_line_between(&start, &end, thickness);
    }
    fn draw_thick_line_between(a: &Vec2, b: &Vec2, thickness: f32) -> ()
    {
        fn to_3d(v: Vec2) -> Vec3
        {
            vec3(v.x, UI_HEIGHT, v.y)
        }

        let a = *a;
        let b = *b;
        let diff = b-a;
        if diff.length() < ARBITRARY_SMALL
        {
            // Just draw a square at the midpoint and then quit
            let p = a+diff/2.;
            draw_plane(to_3d(p), vec2(thickness,thickness), None, GREEN);
            return;
        }

        // If not, then draw the line by constructing the angled rect
        let dir = (b-a).normalize();
        let orth_dir = Vec2::from_angle(-PI/2.).rotate(dir);

        let top_left = a+orth_dir*thickness/2.0;
        let top_right = top_left + diff;
        let bottom_right = top_right - orth_dir*thickness;
        let bottom_left = top_left - orth_dir*thickness;

        // The rect is constructed from two triangles
        let vertices = [
            Vertex::new2(to_3d(top_left), vec2(0., 0.), RED  ),
            Vertex::new2(to_3d(top_right), vec2(0., 0.), GREEN),
            Vertex::new2(to_3d(bottom_right), vec2(0., 0.), GREEN),
            Vertex::new2(to_3d(bottom_left), vec2(0., 0.), RED ),
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let mesh = Mesh {
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
            texture: None,
        };

        draw_mesh(&mesh);
    }
}
