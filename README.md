# fqlink

`fqlink` is a small command-line utility to help you retrieve ENA fastq links from NCBI accession codes. The input is provided through STDIN with each line representing an NCBI accession code (srx, srr, gsm) and an optional related name, separated by whitespace (space, tab, etc.). The output is either in the aria2 input file format or Aspera download info JSON format.

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [Options](#options)
- [Troubleshooting](#troubleshooting)
- [Notes](#notes)
- [License](#license)

## Installation

Download the binary from release, put it wherever in your `PATH`.

You can also use cargo to install from GitHub:

```bash
cargo install --git https://github.com/TTTPOB/fqlink.git
```


## Usage

Read from STDIN, and print to STDOUT
```bash
fqlink [OPTIONS] <input_file >output_file
```

The output file can then be used in aria2c
```bash
aria2c -x16 -s16 -k1m -i output_file
```


Input:
- Accession codes (srx, srr, gsm) and related (optional) names, names should not contain whitespace
- Tags should be whitespace (space, tab, etc.) delimited
- One item per line

Example input:

```
SRR123456 SampleNameA
SRR654321 SampleNameB
```

## License
MIT