use clap::Parser;
use futures_util::future::join_all;
use std::io::prelude::*;
use std::io::BufReader;
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

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    let stdin = BufReader::new(std::io::stdin());
    let mut line_iter = stdin.lines();
    let mut get_info_tasks = Vec::new();
    while let Some(Ok(line)) = line_iter.next() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let code = AccessionCodes::from_str(line).unwrap();
        get_info_tasks.push(async move { code.get_download_info().await });
    }
    let result = join_all(get_info_tasks).await;
    let result = result
        .into_iter()
        .filter_map(|x| x)
        .flatten()
        .collect::<Vec<_>>();
    if args.ascp {
        for r in result {
            println!("[");
            println!("  {}", r.to_ascp());
            println!("]");
        }
    } else {
        for r in result {
            println!("{}", r.to_aria2());
        }
    }
}

mod types;
