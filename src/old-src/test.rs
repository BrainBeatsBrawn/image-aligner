extern crate nalgebra as na;
extern crate rand;

use rand::{thread_rng, Rng};

#[allow(dead_code)]
#[allow(unused_imports)]

use macroquad::prelude::*;
use na::{Vector2, Vector3,Matrix3,Matrix4,Matrix3x4};
use ::rand::prelude::*;

fn main()
{
    debug!("Hello, world!");

    //let mat = Matrix3::<f32>::new(
    //    //1.0, 2.0, 3.0,
    //    //4.0, 5.0, 6.0,
    //    //7.0, 8.0, 9.0
    //    2.0, 0.0, 0.0,
    //    0.0, 2.0, 0.0,
    //    0.0, 0.0, 2.0
    //);

    //debug!("Matrix: {}",mat.try_inverse().unwrap());

    //const len: usize = 30;
    //const barsize: f32 = 80.0;
    //const samples: usize = 1000000;
    //const power: f32 = 2.5;

    //let mut results: [usize; len] = [0; len];
    //for i in 0..samples
    //{
    //    results[get_weighted_random_index(len, power)] += 1;
    //    //debug!("{}",get_weighted_random_index(10,3.0));
    //}

    //let max = results[0] as f32;
    //for i in 0..len
    //{
    //    let length = (results[i] as f32/max as f32 * barsize) as usize;
    //    let mut bar = String::new();
    //    for _ in 0..length
    //    {
    //        bar.push_str("#");
    //    }
    //    // Pad i to 3 digits, pad results[i] to 6 digits
    //    debug!("{:03}: {:06} {}",i,results[i],bar);
    //}
    
    //// Shuffle tests
    //let width = 5u32;
    //let height = 5u32;
    ////let mut ls = (0..(width*height)).collect::<Vec<u32>>();
    //let mut ls: Vec<u32> = (0..(width*height)).collect::<Vec<u32>>();
    ////let mut ls: Vec<u32> = (0..(width*height)).collect();
    ////let ls: &mut [u32] = &mut ls;
    //ls.shuffle(&mut thread_rng());
    //let ls:Vec<(u32,u32)> = ls.iter().map(|n| (n%width, n/height)).collect();
    //debug!("{:?}",ls);


    // min tests
    let v: Vector2<u32> = Vector2::new(10,2);
    let m: Vector2<u32> = Vector2::new(2,2);
    let v = v.sup(&m);
    debug!("{:?}",v);
}

fn get_weighted_random_index(pop_size: usize, selection_power: f32) -> usize
{
    let mut rng = rand::thread_rng();
    (rng.gen::<f32>().powf(selection_power) * pop_size as f32) as usize
}

