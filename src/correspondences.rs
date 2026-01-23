use na::Vector2;
use crate::WarpSet;

use macroquad::prelude::*;

use std::f32::consts::PI;
use std::fs::read_to_string;
use std::fs::File;
use std::io::Write;

const UI_HEIGHT:f32 = 5.0;

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
    pub fn draw(&self, warpset: &WarpSet, baseline_size: &Vec2, mouse_world_pos: &Vec2, thickness: f32) -> ()
    {
        fn vec64_to_vec2(v: Vector2::<f64>) -> Vec2
        {
            vec2(v.x as f32, v.y as f32)
        }

        // First, draw the proposed line if there is one to draw
        const STAGED_ALPHA: f32 = 0.4;
        if self.staged_baseline.is_some() || self.staged_offset.is_some()
        {
            if self.staged_baseline.is_some() && self.staged_offset.is_some()
            {
                // TODO: Both are staged, so just draw the staged line
                let bp = vec64_to_vec2(self.staged_baseline.unwrap()) * (*baseline_size);
                let op = vec64_to_vec2(warpset.map_point_from_offset_to_baseline(&self.staged_offset.unwrap()));
                Self::draw_thick_line_between(&bp, &op, thickness, STAGED_ALPHA);
            }
            else if self.staged_baseline.is_some()
            {
                // From baseline to mouse
                let bp = self.staged_baseline.unwrap();
                Self::draw_thick_line_between(&(vec64_to_vec2(bp) * (*baseline_size)),
                                              mouse_world_pos, thickness, STAGED_ALPHA);
            }else{
                // From mouse to offset
                let op = warpset.map_point_from_offset_to_baseline(&self.staged_offset.unwrap());
                Self::draw_thick_line_between(mouse_world_pos, &vec64_to_vec2(op), thickness, STAGED_ALPHA);
            }
        }

        // Then, go and draw all the added lines from the correspondences list
        for pair in &self.pairs
        {
            let bp = vec64_to_vec2(pair.bp) * (*baseline_size);
            let op = vec64_to_vec2(warpset.map_point_from_offset_to_baseline(&pair.op));
            Self::draw_thick_line_between(&bp, &op, thickness, 1.0);
        }
    }
    fn draw_thick_line_between(a: &Vec2, b: &Vec2, thickness: f32, alpha: f32) -> ()
    {
        fn to_3d(v: Vec2) -> Vec3
        {
            vec3(v.x, UI_HEIGHT, v.y)
        }

        let aligned_colour = Color{r:0., g:1., b:0., a:alpha};

        let a = *a;
        let b = *b;
        let diff = b-a;

        let line_len = diff.length();
        let line_colour_ratio = (line_len.max(0.001)/0.05).min(1.0); //1.4142 is max distance 
        //let line_colour = Color{r:line_colour_ratio, g:1.-line_colour_ratio, b:0., a:alpha}; // red -> green
        //let line_colour = Color{r:line_colour_ratio, g:1., b:line_colour_ratio, a:alpha};// white -> green
        let line_colour = Color{r:1., g:1.-line_colour_ratio, b:1.-line_colour_ratio, a:alpha};// red -> white 

        let square_size = thickness/3.;
        if line_len < square_size/2.
        {
            // Just draw a square at the midpoint and then quit
            let p = a+diff/2.;
            draw_plane(to_3d(p), vec2(square_size,square_size), None, aligned_colour);
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
            Vertex::new2(to_3d(top_left), vec2(0., 0.), line_colour),
            Vertex::new2(to_3d(top_right), vec2(0., 0.), line_colour),
            Vertex::new2(to_3d(bottom_right), vec2(0., 0.), line_colour),
            Vertex::new2(to_3d(bottom_left), vec2(0., 0.), line_colour ),
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let mesh = Mesh {
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
            texture: None,
        };

        draw_mesh(&mesh);
    }

    pub fn write_to_file(&self, path: &str) -> ()
    {
        println!("Writing correspondences to file {}...", path);
        let mut file = File::create(path).unwrap();
        for p in &self.pairs
        {
            file.write(&format!("{},{}|{},{}\n", p.bp.x, p.bp.y, p.op.x, p.op.y).as_bytes()).unwrap();
        }
        println!("Correspondecne file written!");
    }
    pub fn read_from_file(&mut self, path: &str) -> ()
    {
        println!("Reading file \"{}\"...", path);
        //let file = File::open(path).expect("Error: unable to open specified correspondences file.");
        for line in read_to_string(path).unwrap().lines()
        {
            let point_pair = line.split("|").collect::<Vec<&str>>();
            let baseline_point = point_pair[0].split(",")
                                 .map(str::parse::<f64>)
                                 .collect::<Vec<Result<f64, _>>>();
            let offset_point = point_pair[1].split(",")
                               .map(str::parse::<f64>)
                               .collect::<Vec<Result<f64, _>>>();
            self.pairs.push(
                Pair{
                    bp: Vector2::<f64>::new(*baseline_point[0].as_ref().unwrap(), *baseline_point[1].as_ref().unwrap()),
                    op: Vector2::<f64>::new(*offset_point[0].as_ref().unwrap(), *offset_point[1].as_ref().unwrap())
                }
            );
            println!("baseline: {:?}", baseline_point);
        }
    }
}
