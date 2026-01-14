// A Track is a structure that loads and stores a set of points from a csv.
// The structure handles loading, saving, and transforming to different coordinate spaces.

use std::fs::File;
use std::io::{Read, Write};
use log::info;
use nalgebra::Vector2;
use std::result::Result;
use std::error::Error;

use log::debug;

use crate::warp_set_solver::WarpSet;

#[derive(Debug)]
pub struct Track//<T>???
{
    path_root: String,
    offset_space_points: Vec<Vector2<f32>>,
    baseline_space_points: Vec<Vector2<f32>>,
}

impl Track
{
    pub fn new(path: String) -> Track
    {
        let path_root = {
            if(path.ends_with(".csv"))
            {
                path.split_at(path.len() - 4).0.to_string()
            }
            else
            {
                path
            }
        }.trim().to_string();

        let mut out = Track{path_root, offset_space_points: Vec::new(), baseline_space_points: Vec::new()};
        let _ = out.path_root != "" && out.attempt_load().is_ok();

        out
    }

    pub fn update_baseline_points(&mut self, warp_set: &WarpSet)
    {
        debug!("Updating {} track points", self.offset_space_points.len());
        self.baseline_space_points = self.offset_space_points.iter().map(|p| warp_set.get_offset_point_in_baseline_space(p)).collect();
        debug!("\t...done!");
    }
    pub fn get_baseline_space_points(&self) -> &Vec<Vector2<f32>> // Note that the reference is immutable
    {
        &self.baseline_space_points
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>>
    {
        info!("Saving baseline-space track data to {}-baselinespace.csv...", self.path_root);
        let mut file = File::create(format!("{}-baselinespace.csv", self.path_root))?;
        for point in self.baseline_space_points.iter()
        {
            file.write(format!("{},{}\n", point.x, point.y).as_bytes())?;
        }
        info!("\t...saved!");
        Ok(())
    }

    fn attempt_load(&mut self) -> Result<(), Box<dyn Error>>
    {
        info!("Loading track from {}.csv...", self.path_root);
        let mut f = File::open(self.path_root.clone() + ".csv")?;
        let mut contents = String::new();
        let mut points = Vec::new();
        f.read_to_string(&mut contents)?;
        for line in contents.lines()
        {
            let mut split = line.split(",");
            let x = split.next().unwrap().trim().parse::<f32>()?;
            let y = split.next().unwrap().trim().parse::<f32>()?;
            points.push(Vector2::new(x, y));
        }
        self.offset_space_points = points.to_owned();
        info!("\tLoaded {} points.", points.len());
        Ok(())
    }
}
