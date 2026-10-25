use fid::{
    collect,
    config::Config,
    editorial, export,
    store::{error, hash, Store},
    time, Result,
};
use std::{path::PathBuf, time::Duration};

const USAGE: &str = "Usage: fid daily [--state DIR]\n       fid export [--state DIR] [--out DIR]";

enum Command {
    Daily { state: PathBuf },
    Export { state: PathBuf, output: PathBuf },
}

fn command() -> Result<Command> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or_else(|| error(USAGE))?;
    let mut state = PathBuf::from(".fid");
    let mut output = PathBuf::from("web/dist");
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| error(USAGE))?;
        match flag.as_str() {
            "--state" => state = value.into(),
            "--out" if command == "export" => output = value.into(),
            _ => return Err(error(USAGE)),
        }
    }
    match command.as_str() {
        "daily" => Ok(Command::Daily { state }),
        "export" => Ok(Command::Export { state, output }),
        _ => Err(error(USAGE)),
    }
}

async fn daily(store: &Store) -> Result<()> {
    let date = time::daily(time::now())?.date;
    if store.edition(Some(&date), None)?.is_some() {
        println!("The {date} edition is already published; skipping collection and generation.");
        return Ok(());
    }
    let config = Config::from_env()?;
    tokio::time::timeout(Duration::from_secs(600), collect::all(&config, store)).await??;
    // The cutoff follows collection so the daily snapshot includes the fetched documents.
    let window = time::daily(time::now())?;
    let run = hash(&format!("{}:{}", window.date, window.cutoff));
    if !store.acquire("generation", &run)? {
        return Err(error("Generation is already running"));
    }
    store.start_run(&run, &window.date, window.cutoff, time::now())?;
    let result = tokio::time::timeout(
        Duration::from_secs(780),
        editorial::generate(&config, store, &window, &run, None),
    )
    .await?;
    result?;
    store.snapshot()?;
    println!("Published the {} edition.", window.date);
    Ok(())
}

async fn run() -> Result<()> {
    let command = command()?;
    let state = match &command {
        Command::Daily { state } | Command::Export { state, .. } => state,
    };
    let store = Store::open(state)?;
    store.restore()?;
    match command {
        Command::Daily { .. } => daily(&store).await,
        Command::Export { output, .. } => export::site(&store, &output),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Fid failed: {error}");
        std::process::exit(1);
    }
}
