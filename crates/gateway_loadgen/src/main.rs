use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use clap::Parser;
use clap_derive::Parser;
use hdrhistogram::Histogram;
use rand::Rng;
use tokio::{sync::Semaphore, time::Instant};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    url: String,

    #[arg(short, long)]
    connections: usize,

    #[arg(short, long)]
    rps: u64,

    #[arg(short, long)]
    duration: u64,

    #[arg(short, long)]
    burst: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    run_load(args).await
}

async fn run_load(args: Args) {
    let client = reqwest::Client::new();
    let semaphore = Arc::new(Semaphore::new(args.connections));
    let total_completed = Arc::new(AtomicU64::new(0));
    let histogram = Arc::new(tokio::sync::Mutex::new(Histogram::<u64>::new(3).unwrap()));

    let mut handles = Vec::new();

    let start_time = Instant::now();
    let end_time = start_time + Duration::from_secs(args.duration);

    for _ in 0..args.burst {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let url = args.url.clone();
        let total = total_completed.clone();
        let hist = histogram.clone();

        let scheduled_at = Instant::now();

        handles.push(tokio::spawn(async move {
            let _ = client.get(&url).send().await;
            let latency = scheduled_at.elapsed().as_micros() as u64;

            {
                let mut h = hist.lock().await;
                let _ = h.record(latency);
            }

            total.fetch_add(1, Ordering::Relaxed);
            drop(permit);
        }));
    }

    let period = Duration::from_secs_f64(1.0 / args.rps as f64);
    let mut interval = tokio::time::interval(period);

    while Instant::now() < end_time {
        interval.tick().await;

        let scheduled_at = Instant::now();

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let url = args.url.clone();
        let total = total_completed.clone();
        let hist = histogram.clone();

        handles.push(tokio::spawn(async move {
            let _ = client.get(&url).send().await;

            let latency = scheduled_at.elapsed().as_micros() as u64;

            {
                let mut h = hist.lock().await;
                let _ = h.record(latency);
            }

            total.fetch_add(1, Ordering::Relaxed);
            drop(permit);
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    let total = total_completed.load(Ordering::Relaxed);

    let h = histogram.lock().await;

    println!();
    println!("==== Load Test Summary ====");
    println!("Duration: {:.2}s", elapsed);
    println!("Total Completed: {}", total);
    println!("Effective RPS: {:.2}", total as f64 / elapsed);

    if total > 0 {
        println!("Latency (microseconds):");
        println!("  p50:  {}", h.value_at_quantile(0.50));
        println!("  p95:  {}", h.value_at_quantile(0.95));
        println!("  p99:  {}", h.value_at_quantile(0.99));
        println!("  max:  {}", h.max());
    }
}
