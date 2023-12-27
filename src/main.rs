mod cli;

use std::{fs, io};
use std::ffi::OsString;
use std::io::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::time::Instant;

use bimap::{BiHashMap, BiMap};
use clap::Parser;
use flate2::read::GzDecoder;
use indicatif::{HumanDuration, ProgressBar};
use tar::Archive;

fn main() -> anyhow::Result<()> {
    let now = Instant::now();
    let args = cli::Args::parse();

    let replacements = args.filter_chars.chars().collect::<Vec<char>>();

    let file_handle = File::open(&args.input)?;

    let mut tar: Archive<Box<dyn Read>>;

    if args.gzip || args.input.extension().unwrap_or_default() == "gz" {
        tar = Archive::new(Box::new(GzDecoder::new(&file_handle)));
    }
    else {
        tar = Archive::new(Box::new(&file_handle));
    }

    let archive_size = file_handle.metadata()?.len();

    let mut paths: BiMap<String, String> = BiHashMap::new();

    let mut mapping_file = args.tasks.mapping_file.map(|p| BufWriter::new(File::create(p).expect("Failed to create mapping file")));
    let mut errors: Vec<String> = Vec::new();

    println!("Processing archive...");
    let pb = ProgressBar::new(archive_size)
        .with_style(
            indicatif::ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:150.cyan/blue}] {percent}% ({bytes_per_sec}, ETA: {eta}) {msg}").ok().unwrap()
                .progress_chars("#>-"),
        );

    for mut entry in tar.entries()?
        .filter_map(|e| e.ok())
        .filter(|e| e.header().entry_type().is_file())
    {
        let p = entry.path()?;
        if let Some(mut original) = p.to_str().map(|s| s.to_string()) {
            // skip the original file
            if OsString::from(&original) == args.input.file_name().unwrap() {
                continue;
            }

            if !original.is_ascii() {
                eprintln!("Skipping non-ascii path: {original}");
                errors.push(format!("Skipping non-ascii path: {original}"));
                continue;
            }
            if paths.contains_left(&original) {
                eprintln!("Skipping duplicate path: {original}");
                errors.push(format!("Skipping duplicate path: {original}"));
                continue;
            }

            let mut lowercase = original.to_ascii_lowercase();
            if let Some(prefix) = &args.remove_prefix {
                if let Some(stripped) = lowercase.strip_prefix(prefix) {
                    lowercase = stripped.to_string();
                }
            }

            if args.clean_paths {
                lowercase = lowercase
                    .trim()
                    .trim_start_matches(|c| c == '.')
                    .trim_end_matches(|c| c == '.')
                    .chars().filter(|c| !replacements.contains(c))
                    // replace spaces with underscores if they are not already filtered out
                    .map(|c| if c == ' ' { '_' } else { c })
                    .collect::<String>().replace("__", "_");
            }

            if !lowercase.contains('.') {
                eprintln!("Skipping path without extension: {lowercase}");
                errors.push(format!("Skipping path without extension: {lowercase}"));
                continue;
            }

            while paths.contains_right(&lowercase) {
                lowercase = format!("_{}", lowercase);
            }

            paths.insert(original.to_string(), lowercase.clone());

            if original.contains(' ') {
                original = format!("'{original}'");
            }

            if lowercase.contains(' ') {
                lowercase = format!("'{}'", lowercase);
            }

            if let Some(buffer) = mapping_file.as_mut() {
                writeln!(buffer, "{original} {lowercase}")?;
            }

            if let Some(path) = &args.tasks.output {
                let out_path = path.join(lowercase);
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let mut out_file = File::create(out_path)?;

                io::copy(entry.by_ref(), &mut out_file)?;
            }
        }

        let seek = file_handle.try_clone()?.stream_position()?;
        pb.set_position(seek);
    }
    pb.finish_and_clear();
    println!("Done! ({})", HumanDuration(now.elapsed()));

    if !errors.is_empty() {
        if let Some(error_path) = &args.errors_file {
            if let Some(path) = error_path.parent() {
                fs::create_dir_all(path)?;
            }
            let mut errors_file = BufWriter::new(File::create(error_path)?);
            for error in errors {
                writeln!(errors_file, "{error}")?;
            }
        }
    }

    Ok(())
}
