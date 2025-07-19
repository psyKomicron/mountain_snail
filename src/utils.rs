use std::{fmt::Display, time::Duration};

use anyhow::Result;
use console::style;
use gpx::{Track, Waypoint};
use serde::Deserialize;
use time::OffsetDateTime;
use vincenty_core::{self, distance_from_coords};

#[derive(Deserialize)]
pub struct Splits {
    pub splits: Vec<(i32, i32)>
}

pub struct PathStats {
    pub distance: f64,
    pub d_plus: f64,
    pub d_minus: f64,
    pub duration: Duration,
    pub min_height: f64,
    pub max_height: f64,
    pub average_altitude: f64
}

impl Display for PathStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} meters - {}m D+ - {}m D-", self.distance, self.d_plus, self.d_minus)
    }
}

impl Default for PathStats {
    fn default() -> Self {
        Self { distance: Default::default(), d_plus: Default::default(), d_minus: Default::default(), duration: Default::default(), min_height: Default::default(), max_height: Default::default(), average_altitude: Default::default() }
    }
}


pub fn read_gpx(track: &Track, speed_adjustement: f64, edit_track_times: bool) -> PathStats {
    let segments = &track.segments;
    println!("  {} segments found.", style(segments.len()).bold());

    let now = OffsetDateTime::now_utc();

    let mut d_plus = 0.;
    let mut d_minus = 0.;
    
    let mut max_height = 0.0;
    let mut min_height = f64::MAX;
    let mut average_altitude = 0.0;
    
    let mut track_length = 0.0;

    let mut duration: Duration = Duration::default();
    
    for segment in segments {
        println!("  {} points.", &segment.points.len());

        for i in 1..segment.points.len() {
            let a = &segment.points[i - 1];
            let b = &segment.points[i];

            if let Ok(distance) = distance_3d(a, b) {
                track_length += distance;

                let mut delta_elevation = 0.0;

                let a_elevation = a.elevation;
                if let Some(b_elevation) = b.elevation && a_elevation.is_some() {
                    let a_elevation = a_elevation.unwrap();
                    if b_elevation > a_elevation {
                        d_plus += b_elevation - a_elevation;
                    }
                    else {
                        d_minus += a_elevation - b_elevation;
                    }

                    if max_height < b_elevation {
                        max_height = b_elevation;
                    }
                    if min_height > b_elevation {
                        min_height = b_elevation;
                    }

                    delta_elevation = b_elevation - a_elevation;
                    average_altitude = (average_altitude + b_elevation) / 2.;
                }

                if edit_track_times {
                    let time = (now + duration).into();
                    a.time = Some(time);
                }
                duration += slope_speed(delta_elevation, distance * 1000.0, speed_adjustement);
            }
            else {
                println!("  {}", style(format!("failed to calculate distance between point {} and {}", i - 1, i)).red());
            }
        }
    }

    PathStats { 
        distance: track_length, 
        d_plus, 
        d_minus, 
        duration, 
        min_height, 
        max_height,
        average_altitude
    }
}

pub fn stats(splits: &Splits, split_length: i32) -> PathStats {   
    /*PathStats { 
        distance: (splits.splits.len()) as f64 * split_length  as f64, 
        d_plus: splits.splits.iter().fold(0., |sum, tuple| sum + tuple.0 as f64), 
        d_minus: splits.splits.iter().fold(0., |sum, tuple| sum + tuple.1 as f64),
        duration: Duration::default(),
        min_height: 0.,
        max_height: 0.
    }*/
    PathStats::default()
}

pub fn calculate_travel_time(splits: &Vec<(i32, i32)>, split_length: i32, formula_adjustement: f64) -> Vec<Duration> {
    let mut time_table: Vec<Duration> = vec![];
    
    for split in splits {
        time_table.push(slope_speed((split.0 - split.1) as f64, split_length as f64, formula_adjustement));
    }

    time_table
}


fn slope_speed(delta_elevation: f64, distance: f64, formula_adjustement: f64) -> Duration {
    let segment_speed = 0.6_f64 * (3.5 * (delta_elevation / distance + formula_adjustement)).exp();
    let seconds = (segment_speed * distance).round() as u64;
    Duration::from_secs(seconds)
}

fn distance_3d(a: &Waypoint, b: &Waypoint) -> Result<f64> {
    let a_p = &a.point();
    let b_p = &b.point();

    distance_from_coords(&a_p.0, &b_p.0)
    
    /*
    let result = distance_from_coords(&a_p.0, &b_p.0)?;
    let a_elevation = a.elevation.unwrap_or(0.);
    let b_elevation = b.elevation.unwrap_or(0.);
    let delta_elevation = b_elevation - a_elevation;

    Ok((result.powi(2) + delta_elevation.powi(2)).sqrt())*/
}