// A module that implements the warp set solver.

// Include the WarpSet type and features
mod warp_set;
pub use warp_set::WarpSet;

use log::{debug, info, warn, error, log_enabled, Level};
use na::{Vector2, Vector3, Matrix3, Matrix4, Matrix3x4, Matrix2x4};
use macroquad::texture::Image;
use ::rand::prelude::*;
use std::thread; // For thread sleeping
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::sync::mpsc;

pub enum SolverControlMessage {
    Subdivide,
    Pause,
    Resume,
    TogglePlayPause,
}

pub enum SolverState {
    Paused,
    Running,
    Unknown,
}

impl SolverState {
    pub fn get_message_from_running(running : bool) -> SolverState
    {
        if running {SolverState::Running} else {SolverState::Paused}
    }
}

//// Solver properties
pub struct GaProperties {
    // Actual Ga properties
    pub population_size: usize, // The size of each generation in the GA
    pub mutation_rate: f32, // The chance that a warp point will be mutated
    pub mutation_scale: f32, // Scale of the mutation, between 0 and 1 as a multiple of the maximum movement distance a point can take before it makes associated subregions concave.
    pub smallest_subregion_scale: f32, // When dividing subregions, what is the smallest subregion width and height, relative to it's parent region
    pub selection_power: f32, // Defines the curve of the selection function where weight = <position scaled between 0 and 1>^selection_power
    pub generational_stability_cutoff: usize, // How many generations to run without improvements before subdividing
    pub stability_difference: f32, // How much of an improvement is required to be considered an improvement
    pub plateau_angle_threshold: f32, // The angle at which the plateau detection will trigger
    pub subdivision_depth: usize, // How many times to subdivide the image
    pub start_running: bool, // Automatically start the solver as soon as the program is running
    pub auto_subdivide: bool, // Automatically subdivide the image when the generational stability cutoff is reached
    pub auto_downscale: bool, // Automatically decrease mutation scale as the generational stability cutoff is reached

    // Debug and visualisation controls
    pub show_all_mutations : bool, // Shows every mutation after they've been generated
    pub mutation_show_speed : u64, // The speed at which mutations are shown
    pub render_debug_ui : bool, // Render the debug UI
}

impl GaProperties {
    pub fn new() -> GaProperties {
        GaProperties {
            population_size: 100,
            mutation_rate: 0.25,
            mutation_scale: 0.5,
            smallest_subregion_scale: 0.3,
            selection_power: 3.0,
            generational_stability_cutoff: 10,
            plateau_angle_threshold: 5.0,
            stability_difference: 0.0001,
            subdivision_depth: 6,

            start_running: false,
            auto_subdivide: false,
            auto_downscale: false,

            show_all_mutations : false,
            mutation_show_speed : 100,
            render_debug_ui : false,
        }
    }
}

#[derive(Clone)]
struct ScoredWarpSet {
    pub warp_set: WarpSet,
    pub score: f32,
}

impl ScoredWarpSet {
    pub fn score(&mut self, offset: &Image, baseline: &Image) {
        self.score = self.warp_set.score(offset, baseline);
    }
    pub fn from_warp_set(warp_set: &WarpSet) -> ScoredWarpSet {
        ScoredWarpSet {
            warp_set: warp_set.clone(),
            score: 0.0,
        }
    }
    pub fn subdivide(&mut self) -> () {
        self.warp_set = self.warp_set.get_subdivided();
        self.score = 0.0;
    }
}

struct Population {
    population: Vec<ScoredWarpSet>, // This could be a fixed-size array. It appears that this probably won't be possible in Rust, because it's not idiomatic to use an array that'll have a fixed run-time length
    generation_id: usize,
    subdivs: usize,
    stability_cutoff: usize,
    last_subdivision_generation: usize
}

impl Population {
    pub fn new(generational_cutoff: usize) -> Population {
        Population {
            population: Vec::new(),
            generation_id: 0,
            subdivs: 0,
            stability_cutoff: generational_cutoff,
            last_subdivision_generation: 0,
        }
    }

    pub fn spawn_from_warp_set(&mut self, warp_set: &WarpSet, ga_properties: &GaProperties) {
        self.population.push(ScoredWarpSet::from_warp_set(warp_set));
        while self.population.len() < ga_properties.population_size
        {
            // Generate a new warp set by mutating the first one
            let mut new_warp_set = self.population[0].clone();
            new_warp_set.warp_set.mutate(ga_properties.mutation_rate, ga_properties.mutation_scale);
            self.population.push(new_warp_set);
        }
    }

    pub fn score(&mut self, offset_segmentation: &Image, baseline_segmentation: &Image) {
        info!("Scoring population...");
        let now = Instant::now();
        let mut pi = 0u32;
        for p in self.population.iter_mut()
        {
            pi = pi+1;
            p.score(offset_segmentation, baseline_segmentation);
        }
        let time_taken = now.elapsed();

        // Sort by score
        self.population.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());

        let highest_scoring_scored_warpset = &self.population[0];
        info!("[best: {:05.4}] Scoring finished in {} seconds ({}ms per warpset)",
                highest_scoring_scored_warpset.score,
                time_taken.as_secs(),
                time_taken.as_millis() as f32/self.population.len() as f32);
    }

    pub fn show_all_mutations(&mut self, primary_warpset: &Arc<Mutex<WarpSet>>, warpset_dirty: &Arc<Mutex<bool>>, ga_properties: &GaProperties) {
        debug!("Showing all mutations...");
        for (i, scored_warpset) in self.population.iter().enumerate()
        {
            debug!("[score: {:05.4}] Showing individual warpset {} from population of {}...",
                    scored_warpset.score,
                    i,
                    self.population.len()
                  );

            *primary_warpset.lock().unwrap() = scored_warpset.warp_set.clone();

            // Tell the renderer that we've updated the primary warpset
            *warpset_dirty.lock().unwrap() = true;

            // Wait for a bit
            thread::sleep(std::time::Duration::from_millis(ga_properties.mutation_show_speed));
        }
    }

    pub fn get_best_warpset(&self) -> &ScoredWarpSet {
        &self.population[0]
    }

    pub fn generate_next_population(&mut self, ga_properties: &GaProperties)
    {
        info!("Generating population {}...", self.generation_id);
        self.generation_id += 1;
        let mut next_pop: Vec<ScoredWarpSet> = Vec::new();
        next_pop.push(self.population[0].clone()); // Always put in the best from the last one
        while next_pop.len() < ga_properties.population_size
        {
            let mut new_warp_set = self.population[Self::get_weighted_random_index(ga_properties.population_size, ga_properties.selection_power)].clone();
            new_warp_set.warp_set.mutate(ga_properties.mutation_rate, ga_properties.mutation_scale);
            next_pop.push(new_warp_set)
        }
        self.population = next_pop;
    }

    pub fn get_subdivs(&self) -> usize {
        self.subdivs
    }

    fn get_weighted_random_index(pop_size: usize, selection_power: f32) -> usize
    {
        let mut rng = rand::thread_rng();
        (rng.gen::<f32>().powf(selection_power) * pop_size as f32) as usize
    }

    fn subdivide(&mut self) -> ()
    {
        info!("Subdividing...");
        for p in self.population.iter_mut()
        {
            p.subdivide();
        }
        self.subdivs += 1;
        self.last_subdivision_generation = self.generation_id;
    }

    fn get_generations_ago_subdiv_happened(&self) -> usize
    {
        self.generation_id - self.last_subdivision_generation
    }
}

//// Actually the solver
//   Note: This should probably be a struct, but currently, that'd require reworking the way it
//   interacts with the UI, so for now I'll leave it as-is. Maybe I'll put everything in a an
//   instance of a struct that gets used during this function.
pub fn find_warp_points(primary_warpset: Arc<Mutex<WarpSet>>
                      , warpset_dirty: Arc<Mutex<bool>>
                      , baseline_seg: &Image
                      , offset_seg: &Image
                      , ga_properties: GaProperties
                      , scores_out: Arc<Mutex<Vec<f32>>>
                      , control_rx: mpsc::Receiver<SolverControlMessage>
                      , state_tx: mpsc::Sender<SolverState>
                      )
{
    info!("Starting warp point finder...");

    // Create variations on the first set (moving within N% of the region size)
    // GA to get best points
    // Plot score per generation
    // Perform subdivision once score reaches steady state
    // Rinse and repeat until max_depth is reached

    //// Create population and add the first warp set
    let mut population:Population = Population::new(ga_properties.generational_stability_cutoff);

    info!("Generating first population...");
    population.spawn_from_warp_set(&primary_warpset.lock().unwrap(), &ga_properties);

    //let mut last_div_point : (usize, f32) = (0, 0.0); // Scores the last point we subdivided at and it's score
    //let mut subdivs: usize = 0;
    let mut running = ga_properties.start_running;
    state_tx.send(SolverState::get_message_from_running(running)).unwrap();

    let mut plateau_tracker: Vec<f32> = Vec::new();
    loop {
        //// Handle control messages
        for received in control_rx.try_iter()
        {
            match received {
                SolverControlMessage::Subdivide => {
                    info!("Subdivivsion requested.");
                    population.subdivide();
                    let highest_scoring_scored_warpset = population.get_best_warpset();
                    *primary_warpset.lock().unwrap() = highest_scoring_scored_warpset.warp_set.clone();
                    *warpset_dirty.lock().unwrap() = true;
                }
                SolverControlMessage::Pause => {
                    info!("Pausing solver.");
                    if(!running)
                    {
                        info!("\tNote: Solver is already paused.");
                    }
                    running = false;
                    state_tx.send(SolverState::Paused).unwrap();
                }
                SolverControlMessage::Resume => {
                    info!("Resuming solver.");
                    if(running)
                    {
                        info!("\tNote: Solver is already running.");
                    }
                    running = true;
                    state_tx.send(SolverState::Running).unwrap();
                }
                SolverControlMessage::TogglePlayPause => {
                    info!("Toggling solver play/pause.");
                    running = !running;
                    state_tx.send(SolverState::get_message_from_running(running)).unwrap();
                }
                _ => {}
            }
        }

        if !running {
            // Just pause for a bit so we're not taking up the whole CPU.
            // Not terribly efficient, but reasonable.
            thread::sleep(std::time::Duration::from_millis(400));
            continue;
        }

        //// Check if the population's plateau'd and do things if it has
        let recent_plateau: &_ = &plateau_tracker[(plateau_tracker.len() as i32-ga_properties.generational_stability_cutoff as i32).max(0) as usize..];
        let recent_len = recent_plateau.len();
        if (recent_len > 2)
        {
            // Get highest and lowest in all tracked generations.
            let lowest = plateau_tracker.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let highest = plateau_tracker.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

            let relative_score = recent_plateau[0] - recent_plateau[recent_len-1];
            let range = highest - lowest;
            let vert_diff = relative_score/(range + 0.00001); // Normalize against the range across *all* tracked generations
            let hoz_diff = recent_len as f32 / ga_properties.generational_stability_cutoff as f32; // Normalize by cutoff length
            let angle = (vert_diff / hoz_diff).atan().to_degrees();
            debug!("Plateau detector Angle: {}", angle); // TODO: Display plateau angle in UI?
            if(angle < ga_properties.plateau_angle_threshold && population.get_subdivs() < ga_properties.subdivision_depth) 
            {
                info!("Solver plateau detected!");
                plateau_tracker.clear();
                // TODO: Decide between subdividing and lowering mutation scale
                population.subdivide();
                // TODO: Don't subdivide when we've reached subdiv limit!!!!
            }
        }

        // Score the population
        population.score(offset_seg, baseline_seg);

        // Debug: Show each of the variations if they want it
        if(ga_properties.show_all_mutations)
        {
            population.show_all_mutations(&primary_warpset, &warpset_dirty, &ga_properties);
        }

        // Show the current best scoring warpset
        let highest_scoring_scored_warpset = population.get_best_warpset();
        debug!("Showing best warpset (grid size: {})...", highest_scoring_scored_warpset.warp_set.get_grid_size());
        *primary_warpset.lock().unwrap() = highest_scoring_scored_warpset.warp_set.clone();
        scores_out.lock().unwrap().push(highest_scoring_scored_warpset.score);
        plateau_tracker.push(highest_scoring_scored_warpset.score);
        *warpset_dirty.lock().unwrap() = true;

        //// If we've plateaued, subdivide

//        // If the best score is better than the last best, then record that in the div point so
//        // that next round will be the start of a new plateau
//        if(best_score < last_div_point.1)
//        {
//            last_div_point.0 = gen_id;
//        }
//        
//        // If we've plateaued for long enough, subdivide
//        if(gen_id - last_div_point.0 > ga_properties.generational_stability_cutoff)
//        {
//            // If we've subdivided enough, break
//            if(subdivs >= ga_properties.subdivision_depth && ga_properties.auto_subdivide)
//            {
//                info!("Done!");
//                break;
//            }
//
//            //// Check if auto-settle is enabled
//            //if(ga_properties.auto_settle)
//            //{
//            //    //TODO: Do the settling???
//            //}
//
//            // Subdivide all the warpsets
//            info!("GA plateaued. Subdividing warpsets...");
//            for p in population.iter_mut()
//            {
//                p.warp_set.subdivide();// = p.warp_set.subdivide();
//            }
//            last_div_point.0 = gen_id;
//            //last_generation_change_score = -1.0; // Indicate that we need to change the score to best score
//            subdivs = subdivs + 1;
//        }
//
//
        // Create the next population
        population.generate_next_population(&ga_properties);
    }
}
