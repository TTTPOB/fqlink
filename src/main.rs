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
}

async fn fetch_info_and_print(code: &AccessionCodes, ascp: bool) {
    let download_info = code.get_download_info().await;
    if let Some(download_info) = download_info {
        for i in download_info {
            if ascp {
                println!("{}", i.to_ascp());
            } else {
                println!("{}", i.to_aria2());
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    let stdin = BufReader::new(std::io::stdin());
    let mut line_iter = stdin.lines();

    let mut tasks = Vec::new();
    if args.ascp {
        println!("[");
    }
    while let Some(Ok(line)) = line_iter.next() {
        let line = line.trim();
        if line.is_empty() {
            continue; // skip empty line
        }
        let code = AccessionCodes::from_str(line).unwrap();
        tasks.push(task::spawn(async move {
            fetch_info_and_print(&code, args.ascp).await;
        }));
        // sleep 200ms to avoid too many requests
        time::sleep(time::Duration::from_millis(200)).await;
    }
    join_all(tasks).await;
    if args.ascp {
        println!("]");
    }
}

mod types;
