//use log::{debug, info, warn, error};
use na::Vector2;

use macroquad::prelude::*;
use csv::Reader;
use crate::WarpSet;
use std::fs::File;
use std::io::Write;

const TRACK_HEIGHT:f32 = 6.0;


#[derive(Debug, Clone)]
pub struct Track
{
    points: Vec::<Vector2::<f64>>, // Stores the points of this track
    display_points: Vec<Vec3>, // Stores the points of this track, offset into baselinespace for
                               // display
}

impl Track
{
    pub fn new(csv_path: &str) -> Option<Track>
    {
        if csv_path == ""
        {
            println!("No track filepath provided.");
            return Some(Track{
                points: Vec::<Vector2::<f64>>::new(),
                display_points: Vec::<Vec3>::new()
            });
        }

        let mut reader = Reader::from_path(csv_path).expect("Oh no there's no file!");
        let mut points = Vec::<Vector2::<f64>>::new();
        for result in reader.records()
        {
            let record = result.expect("Failure to unwrap malformed CSV file line.");
            let p = Vector2::<f64>::new(record.get(0)?.parse::<f64>().unwrap(),
                                      record.get(1)?.parse::<f64>().unwrap()
                                     );
                                      
            points.push(p);
        }
        Some(
            Track{
                points: points,
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
            self.display_points.push(Vec3::new(baseline_p.x as f32 * baseline_size.x,
                                               TRACK_HEIGHT,
                                               baseline_p.y as f32 * baseline_size.y)
                                    );
        }
    }
    // Computes and updates *only* the display points that have been
    // effected by the last move in the WarpSet.
    pub fn update_display_points(&mut self, baseline_size: &Vec2, warpset: &WarpSet) -> ()
    {
        //for p in &self.display_points
        for i in 0..self.points.len()
        {
            let point = self.points[i];
            if !warpset.has_point_been_warped(point) { continue }; // Skip if the point is not
                                                                      // in the zone that's been
                                                                      // warped

            let baseline_p = warpset.map_point_from_offset_to_baseline(&point);
            self.display_points[i] = Vec3::new(baseline_p.x as f32 * baseline_size.x,
                                               TRACK_HEIGHT,
                                               baseline_p.y as f32 * baseline_size.y);
                                    
        }
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
