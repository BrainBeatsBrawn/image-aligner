#![allow(unused_doc_comments)]

extern crate nalgebra as na;
extern crate argparse;

use argparse::{ArgumentParser, StoreTrue, Store};
use std::time::SystemTime;

use macroquad::prelude::*;
use chrono::Utc;

mod warpset;
use warpset::WarpSet;

mod track;
use track::Track;

//const FRAGMENT_SHADER: &'static str = include_str!("display_shader.frag");
//const VERTEX_SHADER: &'static str = include_str!("display_shader.vert");


const OPACITIES: [(KeyCode, f32); 9] = [(KeyCode::Key1,0.1),
                                        (KeyCode::Key2,0.2),
                                        (KeyCode::Key3,0.3),
                                        (KeyCode::Key4,0.4),
                                        (KeyCode::Key5,0.5),
                                        (KeyCode::Key6,0.6),
                                        (KeyCode::Key7,0.7),
                                        (KeyCode::Key8,0.8),
                                        (KeyCode::Key9,0.9),
                                       ];


#[macroquad::main("Track Aligner")]
async fn main() {

    // Make the sahders here, since it's a pain to make them elsewhere.
    //let offset_display_material = load_material(
    //    ShaderSource::Glsl {
    //        vertex: VERTEX_SHADER,
    //        fragment: FRAGMENT_SHADER,
    //    },
    //    MaterialParams {
    //        ..Default::default()
    //    }
    //).unwrap();
    
    // Constants
    const SCROLL_SPEED: f32 = 0.2; // 20% size
    const MOVE_SPEED: f32 = 1.0; // scale for movement
    const CAM_HEIGHT:f32 = 10.0;
    const BASELINE_HEIGHT: f32 = 0.0;

    env_logger::init();

    info!("Track Aligner Running!");
    
    //// Parse command line arguments
    let mut baseline_image_path = String::new();
    let mut baseline_bush_segmentation_path = String::new();
    let mut offset_image_path = String::new();
    let mut offset_bush_segmentation_path = String::new();
    let mut start_points_input = String::new();
    let mut start_points_filepath = String::new();
    let mut input_track_filepath = String::new();
    let mut process_points_and_exit = false;

    // UI properties
    //let mut ui_config = UiConfig::new();

    // Handles parsing the arguments
    {
        let mut ap = ArgumentParser::new();
        //ap.set_description("Aligns two the same region based on bush segmentation.\n
        //                   'q' - quit.\n
        //                   'w'/'s' - change offset opacity.\n
        //                   'a' - switch display mode.\n
        //                   'p' - play/pause the solver.\n
        //                   'o' - save the current warp mesh to that specified by '-o'.\n
        //                   't' - save the current track data.\n
        //                   tab - subdivide the current warp mesh.\n
        //                   Click and drag to move around, scroll to zoom in and out.\n
        //                   ");
        ap.set_description("A PROGRAM!");
        ap.refer(&mut baseline_image_path)
            .add_argument("BASELINE_IMAGE", Store, "Path to the baseline image (the one that will not be warped)")
            .required();
        ap.refer(&mut baseline_bush_segmentation_path)
            .add_argument("BASELINE_SEG", Store, "Path to the baseline bush segmentation image")
            .required();
        ap.refer(&mut offset_image_path)
            .add_argument("OFFSET_IMAGE", Store, "Path to the offset image (the one that will be warped)")
            .required();
        ap.refer(&mut offset_bush_segmentation_path)
            .add_argument("OFFSET_IMAGE_SEG", Store, "Path to the offset bush segmentation image")
            .required();
        ap.refer(&mut start_points_input)
            .add_option(&["-s", "--start-points"], Store, "Comma-separated list of four points in the baseline image-space that will be used as a starting point for the warp search. Row-major order. Each point is a comma-separated pair of numbers, e.g. \"(0,0),(1,0),(0,1),(1,1)\" (including quotation marks)");
        ap.refer(&mut start_points_filepath)
            .add_option(&["-i", "--input-file"], Store, "Path to a file from a previously saved session (saving is done with the 's' key).");
        ap.refer(&mut input_track_filepath)
            .add_option(&["-t", "--input-track-file"], Store, "Path to a track CSV file to load.");
        ap.refer(&mut process_points_and_exit)
            .add_option(&["-p", "--process-points"], StoreTrue, "If enabled, the program simply immediately processes the track supplied with -t against the supplied pointset and then closes.");
        ap.parse_args_or_exit();
    }
    
    let mut track = Track::new(&input_track_filepath).expect("Input track had malformed numbers.\n");
    let start_points = warpset::get_start_points_from_string(start_points_input).expect("Start points malformed.");
    info!("Start points provided: {:?}", start_points);

    // If we're just going to process and exit, then do that now.
    if process_points_and_exit
    {
        info!("Warping input track...");
        // Create placeholder image that doesn't take a millenia to load
        let placeholder_img: Texture2D = Texture2D::empty();
        let mut warpset;
        if start_points_filepath.trim() == "" {
            warpset = WarpSet::new(&start_points, &placeholder_img).expect("Failed to create warpset from start points.");
        }else{
            warpset = WarpSet::from_file(&start_points_filepath, &placeholder_img).expect("Failed to create warpset from file.");
        };
        save_warped_track(&track, &warpset, &input_track_filepath);
        return;
    }
    
    //// Configure parsed variables
    info!("Loading baseline image from {}...", baseline_image_path);
    let baseline_image: Texture2D = load_texture(baseline_image_path.as_str()).await.unwrap();
    info!("\t...done.");
    info!("Loading baseline bush segmentation image from {}...", baseline_bush_segmentation_path);
    //let baseline_segmentation_image: Texture2D = load_texture(baseline_bush_segmentation_path.as_str()).await.unwrap();
    //let baseline_segmentation_data: Image = baseline_segmentation_image.get_texture_data();
    info!("\t...done.");
    info!("Loading offset image from {}...", offset_image_path);
    let offset_image: Texture2D = load_texture(offset_image_path.as_str()).await.unwrap();
    info!("\t...done.");
    info!("Loading offset bush segmentation image from {}...", offset_bush_segmentation_path);
    //let offset_segmentation_image: Texture2D = load_texture(offset_bush_segmentation_path.as_str()).await.unwrap();
    //let offset_segmentation_data: Image = offset_segmentation_image.get_texture_data();
    info!("\t...done.");


    //// Begin processing
    // Calculate the width and height of the baseline image so we can draw the baseline image to it's true aspect.
    let baseline_size = vec2(baseline_image.width() as f32 / baseline_image.height() as f32, 1.0);
    let baseline_radial_size = baseline_size/2.; // For some reason the draw_plane method uses radii
    let mut warpset;
    if start_points_filepath.trim() == "" {
        warpset = WarpSet::new(&start_points, &offset_image).expect("Failed to create warpset from start points.");
    }else{
        warpset = WarpSet::from_file(&start_points_filepath, &offset_image).expect("Failed to create warpset from file.");
    };

    let mut camera = Camera3D{
        position: vec3(0.0, CAM_HEIGHT, 0.0),
        target: vec3(0.0, 0.0, 0.0), // Camera is looking at the origin
        up: vec3(0.0, 0.0, -1.0), // Camera's "up" direction is the negative z axis so that the camera is looking down the the xz plane forming the same setup as the xy in UV-space
        fovy: 1., // Near plane width in orthographic view
        projection: Projection::Orthographics,
        //viewport: Seome((0,0,0,0)), // If we ever want to have a UI, the graphics will be rendered to the right
        ..Default::default()
    };
    

    let mut last_time = SystemTime::now();
    track.compute_display_points(&baseline_size, &warpset);
    let mut draw_mesh = true;
    loop {
        clear_background(LIGHTGRAY);

        set_camera(&camera);

        // Draw a point at the mouse for debug purposes
        //let mouse_pos_w = get_mouse_wcoords(&camera);
        //draw_sphere(vec3(mouse_pos_w.x, MOUSE_HEIGHT, mouse_pos_w.y), 0.05, None, YELLOW);
        let mouse_pos_bl = get_mouse_baseline_coords(&camera, &baseline_size);

        // Update the warpset to track actions relative to the mouse
        warpset.update_closest_point(&mouse_pos_bl, is_mouse_button_down(MouseButton::Left));
        /// TODO: Update any track data, probably only in the regions that have been effected, _if_
        /// they've been effected.
        // if the mouse is down (probably in the warpset via warpset.mouse_was_down? or something:
        //   track.update_baseline_cache(warpset.getLastEffectedRegion);
        /// Where getLastEffectedRegion computes the bounded 'square' that contains the effected
        /// points.
        /// TODO: Switch to (and implement) "update_display_points" so it's faster.
        //track.compute_display_points(&baseline_size, &warpset);
        track.update_display_points(&baseline_size, &warpset);


        // Handle mouse interactions
        {
            let (_, scroll) = mouse_wheel(); // Returns delta - either -1, 0, or 1
            camera.fovy *= 1.0 + scroll * SCROLL_SPEED;

            let delta_pos = mouse_delta_position();
            if is_mouse_button_down(MouseButton::Right)
            {
                camera.position.x += delta_pos.x * camera.fovy * MOVE_SPEED;
                camera.position.z += delta_pos.y * camera.fovy * MOVE_SPEED;
                camera.target.x = camera.position.x;
                camera.target.z = camera.position.z;
            }
        }


        // Handle keyboard interactions
        if is_key_pressed(KeyCode::Q)
        {
            break;
        }
        if is_key_pressed(KeyCode::Tab)
        {
            warpset.subdivide();
        }
        if is_key_pressed(KeyCode::Up)
        {
            warpset.offset_opacity = (warpset.offset_opacity + 0.1).min(1.0);
        }
        if is_key_pressed(KeyCode::Down)
        {
            warpset.offset_opacity = (warpset.offset_opacity - 0.1).max(0.0);
        }
        if is_key_pressed(KeyCode::S)
        {
            println!("Saving warpset...");
            let filename = Utc::now().to_string() + ".wrp";
            warpset.write_to_file(&filename);
        }
        if is_key_pressed(KeyCode::T)
        {
            save_warped_track(&track, &warpset, &input_track_filepath);
        }
        for (keycode, opacity) in OPACITIES
        {
            if is_key_pressed(keycode)
            {
                warpset.offset_opacity = opacity;
            }
        }
        if is_key_pressed(KeyCode::M)
        {
            draw_mesh = !draw_mesh;
        }


        //// Handle drawing of the UI components
        // baseline image
        draw_plane(vec3(baseline_radial_size.x, BASELINE_HEIGHT, 0.5),
                   baseline_radial_size, Some(&baseline_image),
                   WHITE);
        
        // background grid
        draw_grid(20, 1., BLACK, GRAY);

        warpset.draw_warped_offset_image(&offset_image, &baseline_radial_size);
        if draw_mesh
        {
            warpset.draw_ui(&baseline_size);
        }

        // Track
        track.draw_track();

        set_default_camera();

        // Draw the fps
        let now = SystemTime::now();
        let diff = now.duration_since(last_time).expect("cannot calculate fps.").as_nanos();
        let fps = 1_000_000_000 / diff;
        last_time = now;
        draw_text(format!("FPS: {fps}").as_str(), 5., 12., 20.0, RED);

        next_frame().await
    }
}

fn get_mouse_wcoords(cam: &Camera3D) -> Vec2
{
    // Assembling it the old-fashioned way, because it's easier:
    let mouse_pos_local = mouse_position_local()*0.5 * vec2(screen_width()/screen_height(), 1.) * cam.fovy;
    mouse_pos_local + vec2(cam.position.x, cam.position.z)
}

fn get_mouse_baseline_coords(cam: &Camera3D, baseline_size: &Vec2) -> Vec2
{
    let mouse_w_coords = get_mouse_wcoords(cam);
    mouse_w_coords/(*baseline_size)
}

fn save_warped_track(track: &Track, warpset: &WarpSet, input_track_filepath: &str)
{
    println!("Saving track...");
    let filename = input_track_filepath.clone().to_owned();
    let filename = filename.replace(".csv", "-warped.csv");
    track.write_warped_track_to_file(&warpset, &filename);
    println!("Track saved!");
}
