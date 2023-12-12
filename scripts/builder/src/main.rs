use crate::bundlev1::BundleV1;
use core::panic;
use std::{env, error::Error, fs::File, path::PathBuf};

mod bundlev1;
mod util;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        panic!("Expected four arguments: <ver> <content> <file>")
    }

    let version = &args[1].to_owned();
    let content_dir = PathBuf::from(&args[2]);
    let target = &args[3].to_owned();

    match &version[..] {
        "v1" => BundleV1::make(Box::new(File::create(target)?), content_dir)?,
        _ => {
            panic!("Unknown bundle version {version}.")
        }
    }

    return Ok(());
}