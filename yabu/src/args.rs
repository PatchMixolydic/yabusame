use argh::FromArgs;
use std::fmt::Debug;

use crate::datetime::DateTime;

/// Foo;
#[derive(Debug, FromArgs)]
pub struct Args {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    New(New),
}

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "new", description = "")]
struct New {
    #[argh(option, short = 'p', description = "priority for this task")]
    priority: Option<u8>,

    #[argh(
        option,
        short = 'd',
        description = "date by which this task should be completed"
    )]
    due_date: Option<DateTime>,

    #[argh(positional)]
    description: String,
}
