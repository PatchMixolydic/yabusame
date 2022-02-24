#![allow(unused)]
#![feature(try_blocks)]
#![warn(unused_imports, unused_must_use)]

mod args;
mod datetime;

use crate::args::Args;

fn main() -> anyhow::Result<()> {
    let args = argh::from_env::<Args>();
    println!("{:?}", args);
    Ok(())
}
