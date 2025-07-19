use std::{env::home_dir, fs::{read_dir, File}, io::BufReader, path::PathBuf, process::exit, time::Duration};
use std::fs;

use console::style;
use dialoguer::Select;
use dialoguer;
use gpx::{read, Gpx};
use humanize_duration::prelude::DurationExt;
use readable::up::UptimeFull;

use crate::utils::{calculate_travel_time, read_gpx};

mod utils;

#[derive(PartialEq)]
enum Terrain {
    Unknown,
    Road,
    Path,
    Track,
    Alpine
}

impl From<usize> for Terrain {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Road,
            1 => Self::Path,
            2 => Self::Track,
            3 => Self::Alpine,
            _ => Self::Unknown
        }
    }
}

fn main() {
    println!("Mountain snail - Hiking time calculator.");

    let (is_gpx_file, file_path) = get_path();
    let speed_adjustement = get_speed_adjustement();

    if is_gpx_file {
        analyse_gpx(file_path, speed_adjustement as f64);
    }
    else {
        analyse_by_splits(file_path, speed_adjustement);
    }
}

fn analyse_gpx(gpx_file_path: String, speed_adjustement: f64) {
    let file = File::open(gpx_file_path).unwrap();
    let reader = BufReader::new(file);

    let gpx: Gpx = match read(reader) {
        Ok(gpx) => gpx,
        Err(e) => {
            println!("{} {e:?}", style(format!("Error reading GPX file:")).red());
            exit(-2);
        },
    };

    println!("GPX file has {} track(s), {} route(s).", style(gpx.tracks.len()).bold(), style(&gpx.routes.len()).bold());

    let mut track_index: usize = 0;
    if gpx.tracks.len() > 1 {
        let names: Vec<String> = gpx.tracks.iter().map(|track| {
            return match &track.name {
                Some(name) => name.clone(),
                None => String::from(""),
            }
        }).collect();

        track_index = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select GPX track")
            .items(&names)
            .interact()
            .unwrap();
    }

    println!("{} {} {}",
        style("Chosen track:").bold(),
        style("·").black().bright(),
        style(format!("\"{}\" (track n°{})", 
            match &gpx.tracks[track_index].name { Some(name) => name.clone(), None => String::from("Default") },
            track_index + 1
        )).green()
    );

    let edit_track_times = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Add time to GPX points ?")
        .interact()
        .unwrap();

    let track = gpx.tracks[track_index].clone();
    
    let stats = read_gpx(&track, speed_adjustement, edit_track_times);

    println!("  {}", style("Track info:").bold());
    println!("    {} {} m D+ {} m D-", style(">").blue(), stats.d_plus.round_ties_even(), stats.d_minus.round_ties_even());
    println!("    {} {} km", style(">").blue(), (stats.distance * 100.).round() / 100.);
    println!("    {} Range: {} m - {} m", style(">").blue(), stats.min_height, stats.max_height);
    println!("    {} Time: {}", style(">").blue(), UptimeFull::from(stats.duration));
    println!("    {} Average altitude: {} m", style(">").blue(), stats.average_altitude.round_ties_even());
}

fn analyse_by_splits(splits_file_path: String, speed_adjustement: f32) {
    let splits_string: String = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Splits (meters): ")
        .with_initial_text("1000")
        .validate_with(|input: &String| -> Result<(), String> {
            let result= input.parse::<i32>();
            // Path invalid or fs error:
            if result.is_err() {
                Err(result.err().unwrap().to_string())
            }
            else {
                Ok(())
            }
        })
        .interact_text()
        .unwrap();
    let splits_length = splits_string.parse::<i32>().expect("Split length not parseable into i32");

    let splits: utils::Splits = serde_json::from_reader(
        std::io::BufReader::new(fs::File::open(splits_file_path).expect("Cannot open splits file.")))
        .expect("Failed to read splits file.");

    println!("{} split(s) found.\nPath info: {}", 
        style(format!("{}", splits.splits.len())).bold(), 
        style(format!("{}", utils::stats(&splits, splits_length))).bold()
    );

    println!("Splits:");
    let times = calculate_travel_time(&splits.splits, splits_length, speed_adjustement as f64);
    let mut total_time = Duration::new(0, 0);
    let mut split_number = [0, 1];
    for duration in times {
        total_time += duration;

        println!("{} : {} -- {}", 
            style(format!("{split_number:?}")).dim(),
            duration.human(humanize_duration::Truncate::Second),
            total_time.human(humanize_duration::Truncate::Second)
        );
        
        split_number[0] += 1;
        split_number[1] += 1;
    }

    println!("Total time: {}", style(total_time.human(humanize_duration::Truncate::Minute)).bold());
}

fn get_terrain() -> Terrain {
    let choices = vec!["road", "path", "track", "alpine", "manual"];
    dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Terrain")
        .items(&choices)
        .default(1)
        .interact()
        .unwrap()
        .into()
}

fn get_path() -> (bool, String) {
    let choices = vec!["GPX", "JSON splits"];
    let is_gpx_file = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Type")
        .items(&choices)
        .interact()
        .unwrap() == 0;

    if is_gpx_file {
        match home_dir() {
            Some(home_dir) => {
                let entries = fs::read_dir(home_dir.join("Documents"))
                .map(|result| result.map(|e| e.unwrap().path()))
                .unwrap()
                .collect::<Vec<PathBuf>>();

                let selections = entries.iter()
                    .map(|path|{
                        if path.is_dir() {
                            read_dir(path)
                                .map(|result|  result.map(|e| e.unwrap().path()))
                                .unwrap()
                                .collect::<Vec<PathBuf>>()
                            }
                            else {
                                vec![path.clone()]
                            }})
                    .flatten()
                    .filter(|path_buf| {
                        if let Some(extension) = path_buf.extension() {
                            extension == "gpx"
                        }
                        else {
                            false
                        }
                    })
                    .map(|path_buf| String::from(path_buf.to_str().unwrap()))
                    .collect::<Vec<String>>();

                let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Choose file")
                    .items(&selections)
                    .interact_opt()
                    .unwrap();

                if let Some(index) = selection {
                    return (true, selections[index].to_owned());
                }
            },
            None => {},
        }
    }
    
    let mut splits_file_path_input_history = dialoguer::BasicHistory::new().max_entries(1);
    let string = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt(if is_gpx_file { "GPX file path:" } else { "Splits file path:" })
        .with_initial_text(if is_gpx_file { "" } else { "./splits.json" })
        .history_with(&mut splits_file_path_input_history)
        .validate_with(|input: &String| -> Result<(), &str> {
            let result = fs::exists(input);
            // Path invalid or fs error:
            if result.is_err() || !result.unwrap() {
                Err("Path doesn't exist")
            }
            else {
                Ok(())
            }
        })
        .interact_text()
        .unwrap();

    (is_gpx_file, string)
}

fn get_speed_adjustement() -> f32 {
    // let use_known_speed_values = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
    //     .with_prompt("Choose hiking terrain")
    //     .interact()
    //     .unwrap();

    return match get_terrain() {
        Terrain::Unknown => {
            let variable_string = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Walking speed adjustement (bigger == slower):")
                .with_initial_text("0.16")
                .validate_with(|input: &String| -> Result<(), String> {
                    let result= input.parse::<f32>();
                    // Path invalid or fs error:
                    if result.is_err() {
                        Err(result.err().unwrap().to_string())
                    }
                    else {
                        Ok(())
                    }
                })
                .interact_text()
                .unwrap();

            variable_string.parse::<f32>().expect("Variable not parseable into f32")
        },
        Terrain::Road => 0.05,
        Terrain::Path => 0.08,
        Terrain::Track => 0.175,
        Terrain::Alpine => 0.28,
    }
}