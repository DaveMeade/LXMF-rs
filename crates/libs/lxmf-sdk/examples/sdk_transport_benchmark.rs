use lxmf_sdk::{
    MessageId, NegotiationRequest, SdkBackend, ZmqPipelineBackendClient, ZmqPipelineBackendConfig,
};
use serde_json::json;
use std::hint::black_box;
use std::time::Instant;

struct Args {
    transport: String,
    endpoint: String,
    operation: String,
    iterations: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let samples = match args.transport.as_str() {
        "zmq" => run(
            ZmqPipelineBackendClient::new(ZmqPipelineBackendConfig::local(args.endpoint.clone()))?,
            &args.operation,
            args.iterations,
        )?,
        "http" | "unix" => run(
            lxmf_sdk::RpcBackendClient::new(args.endpoint.clone()),
            &args.operation,
            args.iterations,
        )?,
        other => return Err(format!("unsupported transport {other}").into()),
    };
    println!("{}", serde_json::to_string(&summarize(&args, samples))?);
    Ok(())
}

fn run<B: SdkBackend>(
    backend: B,
    operation: &str,
    iterations: usize,
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);
    match operation {
        "negotiate" => {
            let request: NegotiationRequest = serde_json::from_value(json!({
                "supported_contract_versions": [2],
                "requested_capabilities": [],
                "profile": "desktop-full",
                "bind_mode": "local_only",
                "auth_mode": "local_trusted",
                "overflow_policy": "reject",
                "block_timeout_ms": null,
                "rpc_backend": null,
                "extensions": {}
            }))?;
            backend.negotiate(request.clone())?;
            for _ in 0..iterations {
                let started = Instant::now();
                black_box(backend.negotiate(request.clone())?);
                samples.push(started.elapsed().as_nanos() as f64);
            }
        }
        "snapshot" => {
            backend.snapshot()?;
            for _ in 0..iterations {
                let started = Instant::now();
                black_box(backend.snapshot()?);
                samples.push(started.elapsed().as_nanos() as f64);
            }
        }
        "operation_registry" => {
            backend.operation_registry()?;
            for _ in 0..iterations {
                let started = Instant::now();
                black_box(backend.operation_registry()?);
                samples.push(started.elapsed().as_nanos() as f64);
            }
        }
        "status" => {
            let id = MessageId("benchmark-missing-message".to_owned());
            backend.status(id.clone())?;
            for _ in 0..iterations {
                let started = Instant::now();
                black_box(backend.status(id.clone())?);
                samples.push(started.elapsed().as_nanos() as f64);
            }
        }
        "poll_events" => {
            backend.poll_events(None, 1)?;
            for _ in 0..iterations {
                let started = Instant::now();
                black_box(backend.poll_events(None, 1)?);
                samples.push(started.elapsed().as_nanos() as f64);
            }
        }
        "router_stats" => {
            backend.router_stats()?;
            for _ in 0..iterations {
                let started = Instant::now();
                black_box(backend.router_stats()?);
                samples.push(started.elapsed().as_nanos() as f64);
            }
        }
        other => return Err(format!("unsupported benchmark operation {other}").into()),
    }
    Ok(samples)
}

fn summarize(args: &Args, mut samples: Vec<f64>) -> serde_json::Value {
    samples.sort_by(f64::total_cmp);
    let percentile = |fraction: f64| {
        let index = ((samples.len() - 1) as f64 * fraction).round() as usize;
        samples[index]
    };
    json!({
        "transport": args.transport,
        "operation": args.operation,
        "iterations": args.iterations,
        "p50_ns": percentile(0.50),
        "p95_ns": percentile(0.95),
        "p99_ns": percentile(0.99),
        "samples_ns": samples,
    })
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let mut transport = None;
    let mut endpoint = None;
    let mut operation = None;
    let mut iterations = 100usize;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--transport" => transport = args.next(),
            "--endpoint" => endpoint = args.next(),
            "--operation" => operation = args.next(),
            "--iterations" => {
                iterations = args.next().ok_or("missing --iterations value")?.parse()?;
            }
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    if iterations == 0 {
        return Err("iterations must be positive".into());
    }
    Ok(Args {
        transport: transport.ok_or("missing --transport")?,
        endpoint: endpoint.ok_or("missing --endpoint")?,
        operation: operation.ok_or("missing --operation")?,
        iterations,
    })
}
