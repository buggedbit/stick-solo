extern crate stick_solo;

mod ceo;
mod fcn;
mod utils;
mod world;

use bevy::prelude::*;
use ceo::CEO;
use fcn::*;
use ndarray::prelude::*;
use serde::{Deserialize, Serialize};
use std::{env, fs::File, io::BufReader};
use stick_solo::act::one_holding_switchable_nr_couple::OneHoldingSwitchableNRCouple;
use stick_solo::game::{
    base_plugins::BasePlugins,
    camera_plugin::CameraPlugin,
    goal_couple_plugin::{GoalCouple, GoalCouplePlugin},
    one_holding_switchable_nr_couple_plugin::OneHoldingSwitchableNRCouplePlugin,
    pause_plugin::Pause,
    pause_plugin::PausePlugin,
    status_bar_plugin::{StatusBarPlugin, Ticks},
};
use utils::{control, decode, encode, random_sample_solve, GoalQsCouple};
use world::World;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Experiment {
    fcn: FCN,
    ceo: CEO,
    world: World,
}

fn main() {
    let args = env::args();
    let exp = if args.len() == 1 {
        // Optimize
        let pi = std::f32::consts::PI;
        let world = World {
            origin: Vec2::new(0.0, -0.1),
            holding_ls: vec![0.2, 0.2],
            holding_q_clamps: vec![(None, None), (Some(-pi), Some(-0.0))],
            non_holding_ls: vec![0.2, 0.2],
            non_holding_q_clamps: vec![(None, None), (Some(-pi), Some(-0.0))],
            unscaled_relative_goal_region: (Vec2::new(-1.0, -1.0), Vec2::new(0.1, 1.0)),
        };
        // let world = World {
        //     origin: Vec2::new(0.5, -0.5),
        //     holding_ls: vec![0.2, 0.2],
        //     holding_q_clamps: vec![(None, None), (Some(0.0), Some(pi))],
        //     non_holding_ls: vec![0.2, 0.2],
        //     non_holding_q_clamps: vec![(None, None), (Some(0.0), Some(pi))],
        //     unscaled_relative_goal_region: (Vec2::new(-0.1, -0.5), Vec2::new(0.5, 0.5)),
        // };
        let mut fcn = FCN::new(vec![
            (
                world.holding_ls.len() + world.non_holding_ls.len() + 2,
                Activation::Linear,
            ),
            (16, Activation::LeakyReLu(0.1)),
            (16, Activation::LeakyReLu(0.1)),
            (2, Activation::Linear),
        ]);
        let ceo = CEO {
            generations: 500,
            batch_size: 50,
            num_episodes: 15,
            num_episode_ticks: 200,
            elite_frac: 0.25,
            initial_std: 1.0,
            noise_factor: 1.0,
            ..Default::default()
        };
        let (mean_reward, _th_std) = ceo.optimize(&mut fcn, &world).unwrap();
        let exp = Experiment {
            fcn: fcn,
            ceo: ceo,
            world: world,
        };
        // Save
        use chrono::{Datelike, Timelike, Utc};
        let now = Utc::now();
        serde_json::to_writer_pretty(
            &File::create(format!(
                "{}-{}:{}@{:.2}.json",
                now.month(),
                now.day(),
                now.num_seconds_from_midnight(),
                mean_reward
            ))
            .unwrap(),
            &exp,
        )
        .unwrap();
        exp
    } else {
        if args.len() != 2 {
            panic!("Bad cmd line parameters.");
        }
        // Load from file
        let args = args.collect::<Vec<String>>();
        let file = File::open(&args[1]).unwrap();
        let reader = BufReader::new(file);
        let exp: Experiment = serde_json::from_reader(reader).unwrap();
        exp
    };
    println!("{:?}", exp);

    // Visualize
    let world = exp.world.clone();
    App::build()
        .add_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_resource(WindowDescriptor {
            width: 2000,
            height: 1000,
            ..Default::default()
        })
        .add_plugins(BasePlugins)
        .add_plugin(CameraPlugin)
        .add_plugin(exp.world)
        .add_resource(exp.fcn)
        .add_resource(GoalQsCouple(Array::zeros(0), Array::zeros(0)))
        .add_plugin(OneHoldingSwitchableNRCouplePlugin::new(
            OneHoldingSwitchableNRCouple::new_right_holding(
                world.origin,
                &world.holding_ls,
                &world.sample_holding_qs(),
                &world.holding_q_clamps(),
                &world.non_holding_ls,
                &world.sample_non_holding_qs(),
                &world.non_holding_q_clamps(),
                0.01,
            ),
        ))
        .add_plugin(GoalCouplePlugin::new(GoalCouple(
            Vec2::new(0.0, 0.0),
            world.sample_goal(),
        )))
        .add_plugin(StatusBarPlugin)
        .add_plugin(PausePlugin)
        .add_startup_system(initial_set_goal_system.system())
        .add_system(interactive_set_goal_system.system())
        .add_system(control_system.system())
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .run();
}

fn set_goal(
    agent: &OneHoldingSwitchableNRCouple,
    goal_qs_couple: &mut GoalQsCouple,
    goal_couple: &mut GoalCouple,
    fcn: &FCN,
) {
    let holding_origin = agent.holding().get_current_state().1.clone();
    let non_holding_goal = goal_couple.1;
    // Network pipeline
    let (input, scale) = encode(&agent, &non_holding_goal);
    let forward_pass = fcn.at(&input);
    let holding_goal = decode(&forward_pass, scale, holding_origin);
    // Setting GoalCouple and GoalQsCouple
    *goal_couple = GoalCouple(holding_goal, non_holding_goal);
    random_sample_solve(agent, goal_couple, goal_qs_couple);
}

fn initial_set_goal_system(
    agent: Res<OneHoldingSwitchableNRCouple>,
    mut goal_qs_couple: ResMut<GoalQsCouple>,
    mut goal_couple: ResMut<GoalCouple>,
    fcn: Res<FCN>,
) {
    set_goal(&agent, &mut goal_qs_couple, &mut goal_couple, &fcn);
}

fn interactive_set_goal_system(
    agent: Res<OneHoldingSwitchableNRCouple>,
    mut goal_qs_couple: ResMut<GoalQsCouple>,
    mut ticks: ResMut<Ticks>,
    mut goal_couple: ResMut<GoalCouple>,
    fcn: Res<FCN>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.pressed(KeyCode::I)
        || keyboard_input.pressed(KeyCode::K)
        || keyboard_input.pressed(KeyCode::J)
        || keyboard_input.pressed(KeyCode::L)
    {
        set_goal(&agent, &mut goal_qs_couple, &mut goal_couple, &fcn);
        ticks.0 = 0;
    }
}

fn control_system(
    mut agent: ResMut<OneHoldingSwitchableNRCouple>,
    pause: Res<Pause>,
    mut ticks: ResMut<Ticks>,
    goal_qs_couple: Res<GoalQsCouple>,
    goal_couple: ResMut<GoalCouple>,
) {
    if pause.0 {
        return;
    }
    control(&mut agent, &goal_qs_couple, &goal_couple, ticks.0);
    ticks.0 += 1;
}