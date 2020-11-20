use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    input::keyboard::KeyCode,
    prelude::*,
};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use serde::{Deserialize, Serialize};
use stick_solo::vis::*;

mod ceo;
mod fcn;

extern crate stick_solo;

use ceo::CEO;
use fcn::*;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Experiment {
    fcn: FCN,
    ceo: CEO,
}

fn run() -> Experiment {
    let mut fcn = FCN::new(vec![
        (12, Activation::Linear),
        (5, Activation::LeakyReLu(0.1)),
        (5, Activation::LeakyReLu(0.1)),
        (5, Activation::LeakyReLu(0.1)),
        (5, Activation::LeakyReLu(0.1)),
        (4, Activation::Linear),
    ]);

    let mut ceo = CEO::default();
    ceo.generations = 100;
    ceo.batch_size = 50;
    ceo.num_evalation_samples = 6;
    ceo.elite_frac = 0.25;
    ceo.initial_std = 3.0;
    ceo.noise_factor = 3.0;

    let ls = [0.2, 0.2, 0.2, 0.2];

    let _th_std = ceo.optimize(&ls, &mut fcn).unwrap();

    let exp = Experiment { fcn: fcn, ceo: ceo };

    exp
}

fn main() {
    use std::env;
    use std::fs::File;
    use std::io::BufReader;

    let args = env::args();
    let exp = if args.len() == 1 {
        // Run
        let exp = run();
        // Save
        use chrono::{Datelike, Timelike, Utc};
        let now = Utc::now();
        serde_json::to_writer(
            &File::create(format!(
                "{}-{}:{}.json",
                now.day(),
                now.month(),
                now.num_seconds_from_midnight()
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
}
