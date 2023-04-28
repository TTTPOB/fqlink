use clap::Parser;
use futures_util::future::join_all;
use std::io::prelude::*;
use std::io::BufReader;
use tokio::{task, time};
use types::{AccessionCodes, DownloadableAccession};

#[derive(Parser)]
#[command(author, version, about = "Get ENA fastq link from NCBI accession\n\
Read from STDIN, and print to STDOUT\n\
Input should be whitespace (space, tab, ..) delimited NCBI accession codes (srx, srr, gsm) and related (optional) names\n\
One item per line\n\
Names should not contain whitespace\n\
Output is aria2 input file format, or aspera download info json\n\
", long_about=None)]
struct Cli {
    #[arg(
        short,
        long,
        help = "print aspera download info json, default is of aria2 input file format"
    )]
    ascp: bool,
    #[arg(
        short,
        long,
        help = "time interval of submitting api request to ebi, unit: ms",
        default_value_t = 200
    )]
    interval: u64,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    let stdin = BufReader::new(std::io::stdin());
    let mut line_iter = stdin.lines();

    let mut tasks = Vec::new();
    while let Some(Ok(line)) = line_iter.next() {
        let line = line.trim();
        if line.is_empty() {
            continue; // skip empty line
        }
        let code = AccessionCodes::from_str(line).unwrap();
        tasks.push(task::spawn(async move {
            let info = code.get_download_info().await;
            eprintln!(
                "Generated download info for {}, name {}",
                code.orig_accession(),
                code.name().unwrap_or("NA".to_string())
            );
            info
        }));
        // sleep 200ms to avoid too many requests
        time::sleep(time::Duration::from_millis(args.interval)).await;
    }
    let all_info = join_all(tasks).await;
    // unwrap all info
    let all_info = all_info
        .into_iter()
        .map(|x| x.unwrap())
        .filter_map(|x| x)
        .flat_map(|x| x.into_iter())
        .collect::<Vec<_>>();
    if args.ascp {
        println!("{}", serde_json::to_string_pretty(&all_info).unwrap());
    } else {
        for info in all_info {
            println!("{}", info.to_aria2());
        }
    }
}

mod types;
