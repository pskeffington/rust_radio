//! Bounded response-body timing samples for publisher-approved live feeds.
//! These are diagnostic transport measurements, not packet timing or audible acceptance.
use rust_radio_ip::net::open_station_classified;
use std::io::Read;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires public network access to approved station feeds"]
fn approved_live_stream_chunk_timing_samples() {
    for station in ["skylab", "wcrb"] {
        let cancelled = Arc::new(AtomicBool::new(false));
        let (mut input, guard) = open_station_classified(station, Arc::clone(&cancelled))
            .unwrap_or_else(|error| panic!("{station}: {error}"));
        let started = Instant::now();
        let mut total_bytes = 0usize;
        let mut buffer = [0u8; 8192];
        while started.elapsed() < Duration::from_secs(8) && total_bytes < 512 * 1024 {
            match input.reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => total_bytes += count,
                Err(error) => {
                    eprintln!("{station}: sampling stopped after read error: {error}");
                    break;
                }
            }
        }
        let elapsed = started.elapsed();
        cancelled.store(true, Ordering::Release);
        let timing = guard.timing_snapshot();
        println!("{station}: elapsed={elapsed:?} bytes={total_bytes} timing={timing:?}");
        drop(input);
        drop(guard);
        assert!(total_bytes > 0, "{station}: no audio bytes observed");
        assert!(timing.chunk_count > 0, "{station}: no body chunks observed");
    }
}
