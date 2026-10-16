//! Measure the mechanical owner-coordinate lookup, not any language law.
//! D8/D9 full-width admission is generated from owner-ratified source maps.\n//! D1/D2 and research D10 have no owner-occupancy projection: zero is not a function.

use sens::domain_ladder::DomainCoordinate;
use std::{env, hint::black_box, time::Instant};

fn percentile(sorted: &[f64], numerator: usize, denominator: usize) -> f64 {
    let index = ((sorted.len() - 1) * numerator).div_ceil(denominator);
    sorted[index]
}

fn scan(width: u8, capacity: u16, iterations: usize) -> usize {
    let mut admitted = 0usize;
    for i in 0..iterations {
        let bits = black_box((i as u16) % capacity);
        let coordinate = DomainCoordinate::new(black_box(width), bits)
            .expect("mechanically bounded exact-width source");
        admitted += usize::from(black_box(coordinate.owner_residency()) == Some(true));
    }
    black_box(admitted)
}

fn main() {
    let iterations = env::var("SENS_OCCUPANCY_ITERATIONS")
        .ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(100_000);
    let samples = env::var("SENS_OCCUPANCY_SAMPLES")
        .ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(9);
    assert!(iterations >= 1_000 && samples >= 5);

    println!("domain\tcapacity\tsamples\titerations\tresident_per_cycle\tp50_ns_per_lookup\tp95_ns_per_lookup");
    for (width, capacity, residents) in [
        (1u8, 2u16, 0usize),
        (2, 4, 0),
        (3, 8, 7),
        (4, 16, 14),
        (5, 32, 32),
        (6, 64, 64),
        (7, 128, 126),
        (8, 256, 256),
        (9, 512, 512),
        (10, 1024, 0),
    ] {
        // Verify the exact-width source occupancy before any timed series.
        let observed = scan(width, capacity, usize::from(capacity));
        assert_eq!(observed, residents, "D{width} owner-source occupancy drift");
        let _ = scan(width, capacity, iterations / 5);
        let mut timings = Vec::with_capacity(samples);
        for _ in 0..samples {
            let now = Instant::now();
            let checksum = scan(width, capacity, iterations);
            black_box(checksum);
            timings.push(now.elapsed().as_secs_f64() * 1.0e9 / iterations as f64);
        }
        timings.sort_by(f64::total_cmp);
        println!(
            "D{width}\t{capacity}\t{samples}\t{iterations}\t{residents}\t{:.3}\t{:.3}",
            percentile(&timings, 1, 2),
            percentile(&timings, 95, 100),
        );
    }
}
