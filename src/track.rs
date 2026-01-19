//use log::{debug, info, warn, error};
use num_traits::float::Float;
use std::str::FromStr;
use na::Vector2;

use macroquad::prelude::*;
use csv::Reader;
use crate::WarpSet;
use std::fs::File;
use std::io::Write;

const TRACK_HEIGHT:f32 = 6.0;


#[derive(Debug, Clone)]
pub struct Track<T:Float> //+ std::str::FromStr> // T specifies the resolution of the points
{
    //points: Vec::<(T, T)>, // Stores the points of this track
    points: Vec::<Vector2::<T>>, // Stores the points of this track
    //points_baselinespace: Vec::<Vector2<T>>, // Stores the points of this track, offset into baselinespace
    display_points: Vec<Vec3>, // Stores the points of this track, offset into baselinespace for
                               // display
}

impl<F:Float + std::str::FromStr + std::fmt::Debug + 'static> Track<F>
{
    pub fn new(csv_path: &str) -> Option<Track<F>>
        where <F as FromStr>::Err: std::fmt::Debug
    {
        if csv_path == ""
        {
            println!("EGGGS!");
            return None;
        }

        let mut reader = Reader::from_path(csv_path).expect("Oh no there's no file!");
        let mut points = Vec::<Vector2::<F>>::new();
        for result in reader.records()
        {
            let record = result.expect("Failure to unwrap malformed CSV file line.");
            let p = Vector2::<F>::new(record.get(0)?.parse::<F>().unwrap(),
                                      record.get(1)?.parse::<F>().unwrap()
                                     );
                                      
            points.push(p);
            //println!("last point: {:?}", points[points.len()-1]);
        }
        Some(
            Track{
                points: points,
                //points_baselinespace: Vec::<Vector2<F>>::new(),
                display_points: Vec::<Vec3>::new(),
            }
        )
    }

    // Computes *all* the display points
    pub fn compute_display_points(&mut self, baseline_size: &Vec2, warpset: &WarpSet) -> ()
    {
        self.display_points.clear();
        for p in &self.points
        {
            let baseline_p = warpset.map_point_from_offset_to_baseline(&p);
            self.display_points.push(Vec3::new(baseline_p.x*baseline_size.x, TRACK_HEIGHT, baseline_p.y*baseline_size.y));
        }
    }
    // Computes and updates *only* the display points that have been
    // effected by the last move in the WarpSet.
    pub fn update_display_points(&self, basline_size: &Vec2, warpset: &WarpSet) -> ()
    {
    }

    // Actually saves the track warped 
    pub fn write_warped_track_to_file(&self, warpset: &WarpSet, path: &str) -> ()
    {
        println!("Writing warped track to file: {}...", path);
        let mut file = File::create(path).unwrap();
        for p in &self.points
        {
            let baseline_p = warpset.map_point_from_offset_to_baseline(&p);
            file.write(&format!("{},{}\n", baseline_p.x, baseline_p.y).as_bytes()).unwrap();
        }
        println!("Warped track file written!");
    }

    pub fn draw_track(&self) -> ()
    {
        if self.display_points.len() > 1
        {
            let mut prev = &self.display_points[0];
            for p in &self.display_points
            {
                draw_line_3d(*prev, *p, GREEN);
                prev = p;
            }
        }
    }
}


pub fn test_read(path: &str) -> ()
{
    //let mut reader = Reader::from_path("data/big-files/mosaic/Ant03R01.csv").expect("Oh no there's no file!");
    let mut reader = Reader::from_path(path).expect("Oh no there's no file!");
    for result in reader.records()
    {
        let record = result.expect("Failure to unwrap malformed CSV file line.");
        println!("Here's the line: {:?}", record);
    }
}
