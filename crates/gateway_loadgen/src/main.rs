use clap::Parser;
use clap_derive::Parser;
use hdrhistogram::Histogram;
use rand::Rng;
use std::{sync::Arc, time::Duration};
use tokio::time::{Instant, sleep_until};

#[derive(Parser, Debug)]
#[command(version, about = "High-performance open-loop load tester")]
struct Args {
    #[arg(short, long)]
    url: String,

    #[arg(short, long)]
    workers: usize,

    #[arg(short, long)]
    rps: u64,

    #[arg(short, long)]
    duration: u64,

    #[arg(long, default_value = "1")]
    routes: usize,

    #[arg(long, default_value = "false")]
    csv: bool,
}

struct WorkerStats {
    histogram: Histogram<u64>,
    success_2xx: u64,
    rate_limited_429: u64,
    server_5xx: u64,
    network_errors: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    run(args).await;
}

async fn run(args: Args) {
    let client = reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let (tx, rx) = tokio::sync::mpsc::channel::<Instant>(100_000);
    let rx = Arc::new(tokio::sync::Mutex::new(rx));

    // Spawn worker pool
    let mut worker_handles = Vec::new();

    for _ in 0..args.workers {
        let rx = rx.clone();
        let client = client.clone();
        let base_url = args.url.clone();
        let routes = args.routes;

        worker_handles.push(tokio::spawn(async move {
            let mut stats = WorkerStats {
                histogram: Histogram::<u64>::new(3).unwrap(),
                success_2xx: 0,
                rate_limited_429: 0,
                server_5xx: 0,
                network_errors: 0,
            };

            while let Some(scheduled_at) = rx.lock().await.recv().await {
                let url = weighted_url(&base_url, routes);
                let ip = random_ip();

                let response = client.get(url).header("X-Forwarded-For", ip).send().await;

                let latency = scheduled_at.elapsed().as_micros() as u64;
                let _ = stats.histogram.record(latency);

                match response {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        match status {
                            200..=299 => stats.success_2xx += 1,
                            429 => stats.rate_limited_429 += 1,
                            500..=599 => stats.server_5xx += 1,
                            _ => {}
                        }
                    }
                    Err(_) => stats.network_errors += 1,
                }
            }

            stats
        }));
    }

    drop(rx); // prevent accidental cloning

    // Open-loop scheduler
    let start = Instant::now();
    let end = start + Duration::from_secs(args.duration);
    let period = Duration::from_secs_f64(1.0 / args.rps as f64);

    let mut next_tick = Instant::now();

    while Instant::now() < end {
        next_tick += period;

        let _ = tx.send(next_tick).await;

        sleep_until(next_tick).await;
    }

    drop(tx);

    // Merge results
    let mut global_hist = Histogram::<u64>::new(3).unwrap();
    let mut total_success = 0;
    let mut total_429 = 0;
    let mut total_5xx = 0;
    let mut total_network = 0;

    for handle in worker_handles {
        let stats = handle.await.unwrap();

        global_hist.add(&stats.histogram).unwrap();
        total_success += stats.success_2xx;
        total_429 += stats.rate_limited_429;
        total_5xx += stats.server_5xx;
        total_network += stats.network_errors;
    }

    let total = total_success + total_429 + total_5xx + total_network;
    let elapsed = start.elapsed().as_secs_f64();

    println!("\n==== Load Test Summary ====");
    println!("Duration: {:.2}s", elapsed);
    println!("Total Requests: {}", total);
    println!("Effective RPS: {:.2}", total as f64 / elapsed);

    println!("\nStatus Breakdown:");
    println!("  2xx: {}", total_success);
    println!("  429: {}", total_429);
    println!("  5xx: {}", total_5xx);
    println!("  Network Errors: {}", total_network);

    if total > 0 {
        println!("\nLatency (microseconds):");
        println!("  p50: {}", global_hist.value_at_quantile(0.50));
        println!("  p95: {}", global_hist.value_at_quantile(0.95));
        println!("  p99: {}", global_hist.value_at_quantile(0.99));
        println!("  max: {}", global_hist.max());
    }

    if args.csv {
        export_csv(&global_hist).unwrap();
        println!("\nCSV exported to latency.csv");
    }
}

fn random_ip() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "{}.{}.{}.{}",
        rng.gen_range(1..255),
        rng.gen_range(0..255),
        rng.gen_range(0..255),
        rng.gen_range(1..255)
    )
}

fn weighted_url(base: &str, routes: usize) -> String {
    let mut rng = rand::thread_rng();
    let route = rng.gen_range(1..=routes);
    format!("{}/route{}", base.trim_end_matches('/'), route)
}

fn export_csv(hist: &Histogram<u64>) -> Result<(), Box<dyn std::error::Error>> {
    let mut wtr = csv::Writer::from_path("latency.csv")?;

    wtr.write_record(&["quantile", "latency_microseconds"])?;

    for q in [0.50, 0.90, 0.95, 0.99, 1.0] {
        let value = hist.value_at_quantile(q);
        wtr.write_record(&[q.to_string(), value.to_string()])?;
    }

    wtr.flush()?;
    Ok(())
}
