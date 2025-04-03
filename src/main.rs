use clap::Parser;
use futures_util::future::join_all;
use is_terminal::IsTerminal;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use tokio::{task, time};
use types::{AccessionCodes, DownloadableAccession};

enum Out {
    Stdout(std::io::Stdout),
    BufStdOut(std::io::BufWriter<std::io::Stdout>),
}

impl Out {
    fn new() -> Self {
        if std::io::stdout().is_terminal() {
            Self::Stdout(std::io::stdout())
        } else {
            Self::BufStdOut(BufWriter::new(std::io::stdout()))
        }
    }
    fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        match self {
            Self::Stdout(stdout) => {
                writeln!(stdout, "{}", line)?;
            }
            Self::BufStdOut(buf_stdout) => {
                writeln!(buf_stdout, "{}", line)?;
            }
        }
        Ok(())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Stdout(stdout) => {
                stdout.flush()?;
            }
            Self::BufStdOut(buf_stdout) => {
                buf_stdout.flush()?;
            }
        }
        Ok(())
    }
}

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

    let mut out = Out::new();

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
    for info in all_info {
        out.write_line(&info.to_aria2()).expect("write error");
    }

    out.flush().expect("flush error");
}

mod types;
