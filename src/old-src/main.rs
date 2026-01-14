#![allow(unused_parens)]
#![allow(unused_imports)]

extern crate nalgebra as na;
extern crate argparse;
extern crate rand;

mod warp_set_solver;
use warp_set_solver::{WarpSet, GaProperties, find_warp_points, SolverControlMessage, SolverState};
mod track;
use track::Track;

use argparse::{ArgumentParser, StoreTrue, Store};
use macroquad::prelude::*;
use macroquad::prelude::ShaderSource;
use macroquad::math::{mat4, Mat4, mat3, Mat3};
use macroquad::ui::{hash, root_ui, widgets::{self, Group}, Drag, Ui};
use miniquad::graphics::{BlendState, BlendFactor, Equation, BlendValue};
use na::{Vector2,Vector3,Matrix3,Matrix4,Matrix3x4,Matrix2x4};
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::cmp::{min, max};
use std::fmt;
use std::io::{stdout, stderr};
use std::fs::File;
use std::io::Read;
use std::process::exit;
use ::rand::prelude::*;
use std::sync::mpsc;

use std::rc::Rc;

use log::{debug, info, warn, error, log_enabled, Level};

#[derive(Debug,Clone)]
pub struct UiConfig
{
    render_debug_ui: bool, // Whether we'll render the debug UI
    output_path: String, // Path to save things to
    save_and_quit: bool, // Whether we'll just startup, process points, save and shutdown.
    track_path: String, // Path to the tracks file
}

impl UiConfig
{
    pub fn new() -> UiConfig
    {
        UiConfig {
            render_debug_ui: true,
            output_path: String::new(),
            save_and_quit: false,
            track_path: String::new(),
        }
    }
}

#[macroquad::main("Track Aligner")]
async fn main() {

    env_logger::init();

    info!("Track Aligner Running!");

    //// Parse command line arguments
    let mut baseline_image_path = String::new();
    let mut baseline_bush_segmentation_path = String::new();
    let mut offset_image_path = String::new();
    let mut offset_bush_segmentation_path = String::new();
    let mut start_points_input = String::new();

    // UI properties
    let mut ui_config = UiConfig::new();

    // GA properties and control strings
    let mut ga_props = GaProperties::new();
    let mut controls_string = String::new();
    let mut ga_props_string = String::new();
    let mut auto_string = String::new();

    // Handles parsing the arguments
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Aligns two the same region based on bush segmentation.\n
                           'q' - quit.\n
                           'w'/'s' - change offset opacity.\n
                           'a' - switch display mode.\n
                           'p' - play/pause the solver.\n
                           'o' - save the current warp mesh to that specified by '-o'.\n
                           't' - save the current track data.\n
                           tab - subdivide the current warp mesh.\n
                           Click and drag to move around, scroll to zoom in and out.\n
                           ");
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
        ap.refer(&mut ga_props_string)
            .add_option(&["-G", "--ga-props"], Store, "String representation of the GA properties (run with '-G \"-h\"' to see help for this option)");
        ap.refer(&mut controls_string)
            .add_option(&["-z", "--solver-debug"], Store, "Controls for solver debug info. 'm' - show mutations, 'q' - do it quickly, 's' - do it slowly");
        ap.refer(&mut ui_config.output_path)
            .add_option(&["-o", "--output"], Store, "Filepath to save warp meshes to. Will appent '.wrp' if extension not specified. Place a hash symbol as the first character to indicate instant save-and-close behaviour.");
        ap.refer(&mut ui_config.track_path)
            .add_option(&["-p", "--points"], Store, "Filepath to a csv containing a list of 2D points (a track) in offset-space to load. Each line should contain a pair of numbers to indicate x and y positions, separated by a comma. E.g. \"0,0\". The track will then be displayed on the solver. When points are saved in baselinespace, '-baselinespace' is appended to the filename.");
        ap.refer(&mut ui_config.save_and_quit)
            .add_option(&["-q", "--quick-save"], StoreTrue, "Load the specified points into baselinespace, saves their baseline version, and then close the program immediately. Will not apply if no points are specified.");
        ap.parse_args_or_exit();
    }

    if(ui_config.output_path.len() == 0) {
        ui_config.output_path = "output.wrp".to_string();
    }
    if(!ui_config.output_path.ends_with(".wrp")) {
        ui_config.output_path.push_str(".wrp");
    }

    ui_config.save_and_quit &= ui_config.track_path.len() > 0;

    let mut track = Track::new(ui_config.track_path.clone());

    //// Handle parsed arguments
    // handle the ga properties string
    let ga_props_defaults = GaProperties::new();
    if (ga_props_string.len() != 0)
    {
        // Split into a vector of Strings
        let ga_props_args = ("-G ".to_owned() + ga_props_string.as_str()).split(" ")
                            .collect::<Vec<&str>>()
                            .iter().map(|s| s.to_string()).collect::<Vec<String>>();


        // Debug print args:
        if(ga_props_args.len() > 1)
        {
            // Parse the sub-arguments
            let mut ga_ap = ArgumentParser::new();
            ga_ap.set_description("This sub-argument section controls the properties for the genetic algorithm solver.");

            //let static_str: &'static str = Box::leak(format!("Population Size for the genetic algorithm. Default: {}", ga_props_defaults.population_size).into_boxed_str());
            // Leaking all of these strings into the heap doesn't feel nice, but argparse doesn't
            // take ownership of any of it's strings, so I'll do it to keep string definitions here
            // rather than all over the place.
            //  TODO: Could make a static default props and use that to avoid this? Maybe?
            //  TODO: Why does the argparser want to exist outside of this scope anyway?
            ga_ap.refer(&mut ga_props.population_size)
                .add_option(&["-s", "--population-size"], Store, Box::leak(format!("Population Size for the genetic algorithm. Default: {}", ga_props_defaults.population_size).into_boxed_str()));
            ga_ap.refer(&mut ga_props.mutation_rate)
                .add_option(&["-r", "--mutation-rate"], Store, Box::leak(format!("Mutation Rate for the genetic algorithm. Default: {}", ga_props_defaults.mutation_rate).into_boxed_str()));
            ga_ap.refer(&mut ga_props.mutation_scale)
                .add_option(&["-c", "--mutation-scale"], Store, Box::leak(format!("Mutation sCale for the genetic algorithm. Default: {}", ga_props_defaults.mutation_scale).into_boxed_str()));
            ga_ap.refer(&mut ga_props.selection_power)
                .add_option(&["-p", "--selection-power"], Store, Box::leak(format!("Selection Power for the genetic algorithm. Default: {}", ga_props_defaults.selection_power).into_boxed_str()));
            ga_ap.refer(&mut ga_props.generational_stability_cutoff)
                .add_option(&["-g", "--generational-stability-cutoff"], Store, Box::leak(format!("Generational stability cutoff for the genetic algorithm. How many generations to run without improvements before triggering a subdivision or a mutation size decrease. Set to 0 for manual subdivison. Default: {}", ga_props_defaults.generational_stability_cutoff).into_boxed_str()));
            ga_ap.refer(&mut ga_props.plateau_angle_threshold)
                .add_option(&["-t", "--plateau-threshold"], Store, Box::leak(format!("Angle for plateau Threshold detection in degrees. Default: {}", ga_props_defaults.plateau_angle_threshold).into_boxed_str()));
            ga_ap.refer(&mut ga_props.subdivision_depth)
                .add_option(&["-d", "--subdivision-depth"], Store, Box::leak(format!("Maximum subdivision Depth for the genetic algorithm when in auto mode. Set to 0 for manual subdivision. Default: {}", ga_props_defaults.subdivision_depth).into_boxed_str()));
            ga_ap.refer(&mut auto_string)
                .add_option(&["-a", "--auto"], Store, "Auto controls. 's' - automatically Start on program launch, 'd' - auto subDivide on generational cutoff, 'c' auto downsCale on generational cutoff (not implemented), 'f' full auto (all). Default: ''");
            let parsed = ga_ap.parse(ga_props_args, &mut stdout(), &mut stderr());
            if parsed.is_err() {
                // If an error is detected, argparser will have given the full error message
                // already, so we just exit.
                return;
            }

        }

    }

    // Handle the ga debug controls
    if controls_string.contains("m") {
        ga_props.show_all_mutations = true;
    }
    if controls_string.contains("q") {
        ga_props.mutation_show_speed = 10;
    }else if controls_string.contains("s") {
        ga_props.mutation_show_speed = 1000;
    }
    if controls_string.contains("u") {
        ga_props.render_debug_ui = true;
    }
    
    // Handle the auto controls
    if auto_string.contains("d") || auto_string.contains("f") {
        ga_props.auto_subdivide = true;
    }
    if auto_string.contains("c") || auto_string.contains("f") {
        ga_props.auto_downscale = true;
    }
    if auto_string.contains("s") || auto_string.contains("f") {
        ga_props.start_running = true;
    }

    // Load the images
    info!("Loading baseline image from {}...", baseline_image_path);
    let baseline_image: Texture2D = load_texture(baseline_image_path.as_str()).await.unwrap();
    info!("\t...done.");
    info!("Loading baseline bush segmentation image from {}...", baseline_bush_segmentation_path);
    let baseline_segmentation_image: Texture2D = load_texture(baseline_bush_segmentation_path.as_str()).await.unwrap();
    let baseline_segmentation_data: Image = baseline_segmentation_image.get_texture_data();
    info!("\t...done.");
    info!("Loading offset image from {}...", offset_image_path);
    let offset_image: Texture2D = load_texture(offset_image_path.as_str()).await.unwrap();
    info!("\t...done.");
    info!("Loading offset bush segmentation image from {}...", offset_bush_segmentation_path);
    let offset_segmentation_image: Texture2D = load_texture(offset_bush_segmentation_path.as_str()).await.unwrap();
    let offset_segmentation_data: Image = offset_segmentation_image.get_texture_data();
    info!("\t...done.");
    let start_points = 
    {
        let mut out = Vec::<Vector2<f32>>::new();

        if(start_points_input == "")
        {
            info!("No points provided, using default values (unit square)...");
            out = vec![
                Vector2::new(0.0,0.0),
                Vector2::new(1.0,0.0),
                Vector2::new(0.0,1.0),
                Vector2::new(1.0,1.0)
            ];
        }else{
            info!("Custom start points provided, parsing...");
            if(start_points_input.ends_with(".wrp")){
                info!("\tWarp mesh file '{}' provided, loading...", start_points_input);
                let mut f = File::open(start_points_input.as_str()).unwrap();
                let mut data = String::new();
                match f.read_to_string(&mut data)
                {
                    Ok(_) => {
                        start_points_input = data;
                    },
                    Err(e) => {
                        error!("Error reading file: {}", e);
                    }
                }
            }
            let point_strings = start_points_input.split("),(").collect::<Vec<&str>>();
            for point in point_strings
            {
                let point_str = point.trim().trim_matches(|c| c == '(' || c == ')');
                let vals = point_str.split(",").collect::<Vec<&str>>();
                let x = vals[0].parse::<f32>();
                let y = vals[1].parse::<f32>();
                if(vals.len() != 2 || x.is_err() || y.is_err())
                {
                    error!("Invalid point \"{}\", found in supplied start points argument \"{}\"",point_str, start_points_input);
                    return;
                }
                out.push(Vector2::new(x.unwrap(),y.unwrap()));
            }
            info!("\t...done.");
        }
        out
    };

    // Create the initial warpset
    let initial_warpset: WarpSet = WarpSet::new(&start_points)
        .expect("Error: The given points do not form a valid warpset");

    info!("Initial warpset:");
    initial_warpset.print_warped_points();


    //let initial_warpset = initial_warpset;//.subdivide().subdivide();//.subdivide().subdivide();//.subdivide().subdivide().subdivide();
    if(ui_config.save_and_quit)
    {
        track.update_baseline_points(&initial_warpset);
        let _ = track.save();
        exit(0);
    }

    //// Configure inter-thread mutex variables
    // The warp-set from the offset space to the baseline space, used to generate the warp point matrices
    // one in the offset space and one in the baseline space.
    let warpset: Arc<Mutex<WarpSet>> = Arc::new(Mutex::new(initial_warpset));
    // Tracks whether the warpset has been updated, signalling a copy is required for rendering
    let warpset_dirty: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    // Tracks output data
    let learning_curve: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

    // Configure control channels
    let (solver_control_tx, solver_control_rx) = mpsc::channel::<SolverControlMessage>();
    let (solver_state_tx, solver_state_rx) = mpsc::channel::<SolverState>();

    debug!("Starting the genetic algorithm...");

    ui_config.render_debug_ui = ga_props.render_debug_ui; // Copy across the debug rendering UI stuff

    //// Spawn a task to do the logic processing
    let warpset_handle = Arc::clone(&warpset);
    let warpset_dirty_handle = Arc::clone(&warpset_dirty);
    let learning_curve_handle = Arc::clone(&learning_curve);
    let _warp_finder_handle: thread::JoinHandle<_> = thread::spawn(move || {
        find_warp_points(warpset_handle
                         , warpset_dirty_handle
                         , &baseline_segmentation_data
                         , &offset_segmentation_data
                         , ga_props
                         , learning_curve_handle
                         , solver_control_rx
                         , solver_state_tx
                        );
    });

    debug!("Starting main loop...");
    
    // Handle the display loop in the main thread
    display_loop( Arc::clone(&warpset)
                 , Arc::clone(&warpset_dirty)
                 , Arc::clone(&learning_curve)
                 , baseline_image, offset_image
                 , baseline_segmentation_image, offset_segmentation_image
                 , solver_control_tx
                 , solver_state_rx
                 , ui_config
                 , track
                 ).await;
}

#[derive(Debug, Clone, Copy)]
enum DisplayMode
{
    Images,
    Segmentation,
}
impl fmt::Display for DisplayMode
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match self {
            DisplayMode::Images => write!(f, "image"),
            DisplayMode::Segmentation => write!(f, "segmentation"),
        }
    }
}

async fn display_loop(warpset: Arc<Mutex<WarpSet>>
                     , warpset_dirty: Arc<Mutex<bool>>
                     , learning_curve: Arc<Mutex<Vec<f32>>>
                     , baseline_image: Texture2D
                     , offset_image: Texture2D
                     , baseline_segmentation: Texture2D
                     , offset_segmentation: Texture2D
                     , solver_control_tx: mpsc::Sender<SolverControlMessage>
                     , solver_state_rx: mpsc::Receiver<SolverState>
                     , ui_config: UiConfig
                     , mut track: Track
                     )
{
    // Assemble the view material
    let offset_display_material = load_material(
        ShaderSource {
            glsl_vertex: Some(VERTEX_SHADER),
            glsl_fragment: Some(FRAGMENT_SHADER),
            //glsl_fragment: Some(TEST_FRAGMENT),
            metal_shader: None,
        },
        MaterialParams {
            uniforms: vec![
                           ("warpPoints".to_owned(), UniformType::Mat4), // First row: offset-space xs, 2nd: offset ys, 3rd: baseline xs, 4th: baseline ys
                           ("offsetOpacity".to_owned(), UniformType::Float1), // How transparent the offset image bits in the subregions are
                           ("debugColor".to_owned(), UniformType::Float3), // Colour for debugging
                           ("drawDebug".to_owned(), UniformType::Int1), // This should be a bool, but I don't want to get caught up in changing Macroquad
                          ],
            textures: vec![
                           "offsetImage".to_owned(), // The offset image
                          ],
            // Make it support transparency
            pipeline_params: PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
            ..Default::default()
        },
    ).unwrap();

    // Cache a number of variables that we will need for use in the drawing loop
    // matrix/pointset drawing information is cached so we don't have to unlock all mutexes every frame
    //let mut point_matrices: Vec::<(Mat4,Vec2)> = warpset.lock().unwrap().get_vector_of_mat4s();
    let mut point_matrices: Vec<Mat4> = warpset.lock().unwrap().get_vector_of_mat4s();
    let mut subregion_count: usize = point_matrices.len();
    track.update_baseline_points(&warpset.lock().unwrap());
    //offset_display_material.set_uniform("regionSize", warpset.lock().unwrap().get_region_size());
    *warpset_dirty.lock().unwrap() = true;

    info!("Display configured for {} subregions.", subregion_count);
    
    // TODO: When cloning the bit below, convert to vec2s for GPU handling
    let mut lc_values = Vec::<f32>::new();
    let mut lc_spacing = 0.0;
    let mut offset_opacity = 0.8 as f32;
    //let mut grid_opacity = 0.5 as f32;
    let mut display_mode = DisplayMode::Images;
    let mut selected_baseline_image = &baseline_image;
    
    // Constants
    const SCROLL_SPEED: f32 = 0.2; // 20% size
    const MOVE_SPEED: f32 = 1.0; // scale for movement
    const OPACITY_STEP: f32 = 0.1 as f32;

    // Configure texture paramters
    offset_display_material.set_texture("offsetImage", offset_image.clone());
    offset_display_material.set_uniform("offsetOpacity", 0.8 as f32);
    offset_display_material.set_uniform("drawDebug", if ui_config.render_debug_ui { 1 } else { 0 });

    // Calculate the width and height of the baseline image so we can draw the baseline image to it's true aspect.
    let plane_size = vec2(1.0, baseline_image.height() as f32 / baseline_image.width() as f32);

    // Configure a 3D orthographic camera
    let mut camera = Camera3D{
        position: vec3(0.0, 2.0, 0.0), // Camera is 2 units above the xy plane at the origin
        target: vec3(0.0, 0.0, 0.0), // Camera is looking at the origin
        up: vec3(0.0, 0.0, -1.0), // Camera's "up" direction is the negative z axis so that the camera is looking down the the xz plane forming the same setup as the xy in UV-space
        fovy: 1., // Near plane width in orthographic view
        projection: Projection::Orthographics,
        //viewport: Seome((0,0,0,0)), // If we ever want to have a UI, the graphics will be rendered to the right
        ..Default::default()
    };
    
    //// 3D Perspective camera for debugging
    //let mut camera = Camera3D {
    //        position: vec3(0.0, 2., 0.),
    //        up: vec3(0., 0., -1.),
    //        target: vec3(0., 0., 0.),
    //        fovy: 0.3,
    //        ..Default::default()
    //    };

    // A vector to ensure that each region is drawn with a consistent colour across frames
    let mut debug_colors : Vec<Vec3> = Vec::new();

    //let mut track_points : Vec<Vector2<f32>> = Vec::new();

    // Main loop
    let mut old_size : usize = 0;
    let mut solver_state = SolverState::Unknown;
    loop {
        // Get solver state
        for received in solver_state_rx.try_iter() {
            solver_state = received;
        }
        
        //// Keyboard controls
        if is_key_pressed(KeyCode::Q)
        {
            break; // Quit on 'q'
        }
        if is_key_pressed(KeyCode::W) {
            offset_opacity = (offset_opacity + OPACITY_STEP).min(1.0);
            offset_display_material.set_uniform("offsetOpacity", offset_opacity);
        }
        if is_key_pressed(KeyCode::S) {
            offset_opacity = (offset_opacity - OPACITY_STEP).max(0.0);
            offset_display_material.set_uniform("offsetOpacity", offset_opacity);
        }
        if is_key_pressed(KeyCode::A) {
            match display_mode {
                DisplayMode::Images => {
                    display_mode = DisplayMode::Segmentation;
                    offset_display_material.set_texture("offsetImage", offset_segmentation.clone());
                    selected_baseline_image = &baseline_segmentation;
                },
                DisplayMode::Segmentation => {
                    display_mode = DisplayMode::Images;
                    offset_display_material.set_texture("offsetImage", offset_image.clone());
                    selected_baseline_image = &baseline_image;
                },
            }
        }
        if is_key_pressed(KeyCode::Tab) {
            solver_control_tx.send(SolverControlMessage::Subdivide).unwrap();
        }
        if is_key_pressed(KeyCode::P) {
            solver_control_tx.send(SolverControlMessage::TogglePlayPause).unwrap();
        }
        if is_key_pressed(KeyCode::O) {
            // write the current warpset to file
            warpset.lock().unwrap().write_to_file(&ui_config.output_path);
        }
        if is_key_pressed(KeyCode::T) {
            // Write the track data to it's output file using the current warp mesh.
            track.update_baseline_points(&warpset.lock().unwrap());
            let _ = track.save();
        }

        //// Camera movement controls
        {
            let (_, scroll) = mouse_wheel(); // Returns delta - either -1, 0, or 1
            camera.fovy *= 1.0 + scroll * -SCROLL_SPEED;

            let delta_pos = mouse_delta_position();
            if is_mouse_button_down(MouseButton::Left)
            {
                camera.position.x += delta_pos.x * camera.fovy * MOVE_SPEED;
                camera.position.z += delta_pos.y * camera.fovy * MOVE_SPEED;
                camera.target.x = camera.position.x;
                camera.target.z = camera.position.z;
            }
        }
        
        // Check if the warp points have been updated, and copy them in if they have
        if *warpset_dirty.lock().unwrap()
        {
            // warpset retrival
            let warpset = warpset.lock().unwrap();
            point_matrices = warpset.get_vector_of_mat4s();
            subregion_count = point_matrices.len();

            // Configure some debug colors:
            if(old_size != subregion_count)
            {
                debug_colors = Vec::<Vec3>::new();
                for _ in 0..subregion_count
                {
                    debug_colors.push(vec3(random(), random(), random()));
                }
                old_size = subregion_count;
            }

            // learning curve stuff
            lc_values = learning_curve.lock().unwrap().clone();
            let mut max_lc_val = 0.0;
            let mut min_lc_valu = 10000000.0;
            for i in lc_values.iter()
            {
                if *i > max_lc_val
                {
                    max_lc_val = *i;
                }
                if *i < min_lc_valu
                {
                    min_lc_valu = *i;
                }
            }
            for i in 0..lc_values.len()
            {
                lc_values[i] = 1.0 - (lc_values[i] - min_lc_valu) / (max_lc_val - min_lc_valu); // Normalize the learning curve values
                
                // Scale down to be on the bottom half of the screen only
                lc_values[i] = lc_values[i]*0.5 + 0.5 - 0.01;
            }
            lc_spacing = screen_width() / (lc_values.len().max(2) -1) as f32;

            // Update the tracks with the new warpset
            track.update_baseline_points(&warpset);
            // Get  the track data
            //track_points = track.get_baseline_space_points().clone();

            //debug!("Incorporated {} subregion(s)", subregion_count);
            *warpset_dirty.lock().unwrap() = false;
        }
        
        //// Rendering
        // Set the camera position
        set_camera(&camera);

        // Draw a grey background:
        clear_background(Color::new(0.1, 0.1, 0.1, 1.0));
        
        // Draw the baseline image to a plane juuuust below the xy plane
        draw_plane(vec3(0., -0.001, 0.), plane_size, Some(selected_baseline_image), WHITE);

        // Draw each sub-region by iterating over the warpset
        gl_use_material(&offset_display_material);
        for (i, points) in point_matrices.iter().enumerate()
        {
            // Set the uniforms
            offset_display_material.set_uniform("warpPoints", *points); // Points needs to be dereferenced to get the whole matrix 
            offset_display_material.set_uniform("debugColor", debug_colors[i]);

            // Draw the subregion as a slice in vertical space - only the subregion is opaque
            draw_plane(vec3(0.0, (i as f32/subregion_count as f32), 0.0),
                       plane_size, // The subregion is drawn in respect to the baseline plane size
                       None,
                       RED
                      );
        }
        // Go back to the default material
        gl_use_default_material();

        // Draw the 3D debug indicator
        //let pos:Vector3<f32> = *debug_indicator.lock().unwrap()*2.0;
        //let indicator_size = 0.02;
        //draw_plane(vec3((pos.x-1.0)*plane_size.x, 1.1, plane_size.y*(pos.y-1.0)),
        //           vec2(indicator_size, indicator_size),
        //           None,
        //           Color::new(if pos.z < 0.5 {1.0} else {0.0}, if pos.z < 0.5 {0.0} else {1.0}, 0.0, 1.0)
        //          );

        // Tracks are drawn 1.5 units above the split subregions object.
        let track_points : Vec<Vector2<f32>> = track.get_baseline_space_points().clone();
        const top_left : Vector2<f32> = Vector2::new(-1.0, -1.0);
        const line_height : f32 = 1.5;
        for i in 1..track_points.len()
        {
            let start = top_left + 2.0*track_points[i-1];
            let start = vec3(start.x, line_height, start.y);
            let end = top_left + 2.0*track_points[i];
            let end = vec3(end.x, line_height, end.y);
            draw_line_3d(start, end, BLACK);
        }

        // Go back to default camera for UI stuff
        set_default_camera();

        // Draw UI
        let subregions = point_matrices.len();
        draw_text(format!("Subregions: {}  Viewing: {}", subregions, display_mode).as_str(), 2.0, 15.0, 20.0, RED);
        draw_text(format!("offset opacity: {:.2}", offset_opacity).as_str(), 2.0, 35.0, 20.0, RED);
        match solver_state {
            SolverState::Paused => draw_text("Solver paused", 2.0, 55.0, 20.0, RED),
            SolverState::Running => draw_text("Solver running", 2.0, 55.0, 20.0, GREEN),
            _ => (),
        }

        // Draw the learning curve
        for i in 1..lc_values.len()
        {
            draw_line(
                (i-1) as f32 * lc_spacing, lc_values[i-1] * screen_height(),
                   i  as f32 * lc_spacing, lc_values[i] * screen_height(),
                1.0, GREEN
            );
        }

        // Wait on the next frame
        next_frame().await;
    }
}

//uniform sampler2D _ScreenTexture; // The screen texture, weirdly inverted for some reason (inserted by macroquad)
// Draws an individual subregion, configured by the offset- and baseline- space points p1 thru 4.
const FRAGMENT_SHADER: &'static str = include_str!("display_shader.frag");
const VERTEX_SHADER: &'static str = include_str!("display_shader.vert");
