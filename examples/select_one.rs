use aws_config::{BehaviorVersion, Region};
use clap::Parser;
use tokio_postgres_dsql::{Error, Opts};

#[derive(Parser)]
#[command(name = "select_one")]
#[command(about = "Connect to Aurora DSQL and run SELECT 1")]
struct Cli {
    /// Aurora DSQL cluster endpoint
    #[arg(long)]
    endpoint: String,

    /// AWS region (can also be set via AWS_REGION environment variable)
    #[arg(long, env = "AWS_REGION")]
    region: String,

    /// Number of iterations to run
    #[arg(long, default_value = "1")]
    iters: usize,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    // Build connection string
    let conninfo = format!(
        "host={} port=5432 user=admin dbname=postgres sslmode=require",
        cli.endpoint
    );

    println!("Connecting to {} in region {}...", cli.endpoint, cli.region);

    // Build AWS SDK config with the specified region
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(cli.region))
        .load()
        .await;

    let opts = Opts::new(&conninfo, sdk_config)?;
    let mut conn = opts.connect_one().await?;

    println!("Connected! Running SELECT 1 {} time(s)...", cli.iters);

    for i in 0..cli.iters {
        println!("\nIteration {}/{}", i + 1, cli.iters);

        match run_query(&mut conn).await {
            Ok(value) => println!("Success! Result: {}", value),
            Err(e) => eprintln!("Error: {}", e),
        }

        // Sleep between iterations (except after the last one)
        if i + 1 < cli.iters {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    Ok(())
}

async fn run_query(conn: &mut tokio_postgres_dsql::SingleConnection) -> Result<i32, Error> {
    let client = conn.borrow().await?;
    let rows = client.query("SELECT 1 as value", &[]).await?;

    let value: i32 = rows.get(0).map(|row| row.get(0)).unwrap_or(0);
    Ok(value)
}
