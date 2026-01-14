//use log::{debug, info, warn, error};
use num_traits::float::Float;
use std::str::FromStr;
use na::Vector2;

use macroquad::prelude::*;
use csv::Reader;
use crate::WarpSet;

const TRACK_HEIGHT:f32 = 6.0;


#[derive(Debug, Clone)]
pub struct Track<T:Float> //+ std::str::FromStr> // T specifies the resolution of the points
{
    points: Vec::<(T, T)>, // Stores the points of this track
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
            return None;
        }

        let mut reader = Reader::from_path(csv_path).expect("Oh no there's no file!");
        let mut points = Vec::<(F, F)>::new();
        for result in reader.records()
        {
            let record = result.expect("Failure to unwrap malformed CSV file line.");
            points.push((record.get(0)?.parse::<F>().unwrap(), record.get(1)?.parse::<F>().unwrap()));
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

    pub fn compute_display_points(&self, baseline_size: &Vec2, warpset: &WarpSet) -> ()
    {
    }
    pub fn update_display_points(&self, basline_size: &Vec2, warpset: &WarpSet) -> ()
    {
    }

    // Actually saves the track warped 
    pub fn save_warped_track(&self, warpset: &WarpSet) -> ()
    {
    }

    pub fn draw_track(&self) -> ()
    {
        //fn baseline_space_to_world<F:Float>(v: Vector2<F>) -> Vec3
        //{
        //    vec3(v.x*&baseline_size.x, TRACK_HEIGHT ,v.y*baseline_size.y)
        //}
        if self.display_points.len() > 1
        {
            let mut prev = &self.display_points[0];
            //let mut prev = vec3(prev.x.to_f32().expect("ahhhh") *baseline_size.x,
            //                    TRACK_HEIGHT,
            //                    prev.y.to_f32().expect("ahhhh")*baseline_size.y
            //                   );
            for p in &self.display_points
            {
                println!("{:?}", p);
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
