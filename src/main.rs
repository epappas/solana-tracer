use anyhow::{Context, Result};
use clap::Parser;
use solana_tracer_core::tracer::SolanaTracer;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    transaction_signature: String,

    #[arg(short, long, default_value = "5")]
    max_depth: usize,

    #[arg(short, long, default_value = "10")]
    max_concurrency: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .context("setting default subscriber failed")?;

    let args = Args::parse();

    info!("Starting Solana transaction tracing");
    info!("Transaction signature: {}", args.transaction_signature);
    info!("Max depth: {}", args.max_depth);
    info!("Max concurrency: {}", args.max_concurrency);

    let tracer = SolanaTracer::new(args.max_depth, args.max_concurrency)
        .await
        .context("Failed to create SolanaTracer")?;

    match tracer.trace_transaction(&args.transaction_signature).await {
        Ok(trace) => {
            info!("Transaction tracing completed successfully");
            println!("{}", serde_json::to_string_pretty(&trace)?);
        }
        Err(e) => {
            error!("Failed to trace transaction: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
