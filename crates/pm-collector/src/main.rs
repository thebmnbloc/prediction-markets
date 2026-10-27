use std::fs::File;
use std::io::BufReader;
use std::process::ExitCode;

use engineering_challenge::util::dtos::challenge_orderbook::{
    AnyMessage, PriceLevelUpdateDomain, SnapshotDomain,
};
use engineering_challenge::util::{AlignedSbeReader, OrderbookSchema};

use pm_collector::engine::Engine;
use pm_collector::output::SbeSink;
use pm_collector::replay::replay;

/// Snapshot every minute, as the challenge requires.
const SNAPSHOT_INTERVAL_US: i64 = 60_000_000;

const USAGE: &str = "usage:
  pm-collector replay <session.jsonl> <out.sbe>   replay a recorded session into challenge SBE
  pm-collector decode <file.sbe>                  print every message in a challenge SBE file";

fn run(args: &[String]) -> anyhow::Result<()> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["replay", input, output] => {
            let file = BufReader::new(File::open(input)?);
            let mut engine = Engine::new(SNAPSHOT_INTERVAL_US);
            let mut sink = SbeSink::create(output)?;
            let commands = replay(file, &mut engine, &mut sink)?;
            eprintln!(
                "replay complete: {} resync request(s) emitted",
                commands.len()
            );
            for command in &commands {
                eprintln!("  {command:?}");
            }
            Ok(())
        }
        ["decode", input] => {
            let mut reader = AlignedSbeReader::<OrderbookSchema>::open(input)?;
            while let Some(message) = reader.read_msg()? {
                match message {
                    AnyMessage::Snapshot(decoded) => {
                        let dto = SnapshotDomain::try_from_decoder(decoded)?;
                        println!("{dto:?}");
                    }
                    AnyMessage::PriceLevelUpdate(decoded) => {
                        let dto = PriceLevelUpdateDomain::try_from_decoder(decoded)?;
                        println!("{dto:?}");
                    }
                    _ => anyhow::bail!("unsupported message type in {input}"),
                }
            }
            Ok(())
        }
        _ => anyhow::bail!("{USAGE}"),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
