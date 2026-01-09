use log::{debug, info, warn, error};
use num_traits::float::Float;

use macroquad::prelude::*;
use csv::Reader;


#[derive(Debug, Clone)]
pub struct Track<T:Float> //+ std::str::FromStr> // T specifies the resolution of the points
{
    points: Vec::<(T, T)>, // Stores the points of this track
    points_baselinespace: Vec::<(T, T)>, // Stores the points of this track, offset into baselinespace
}

impl<F:Float + std::str::FromStr> Track<F>
{
    pub fn new(csv_path: &str) -> Option<Track<F>>
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
            //println!("Here's the line: {:?}", record);
            println!("Record[0]: {:?}", record.get(0)?.parse::<F>().unwrap());
            points.push((record.get(0)?.parse::<F>().unwrap(), record.get(1)?.parse::<F>().unwrap()));
        }
        None
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
