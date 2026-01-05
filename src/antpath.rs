use macroquad::prelude::*;

const PATH_HEIGHT:f32 = 6.0;

#[derive(Debug, Clone)]
pub struct AntPath
{
    points: Vec<Vec2>,
}


pub fn new(track_path: String) -> AntPath
{
    AntPath{
        poits: vec![],
    }
}
