use argh::{FromArgValue, FromArgs};
use std::fmt::Debug;
use time::OffsetDateTime;
use url::Url;
use yabusame::connection::{default_server, url_from_str};
use yabusame::{Delta, Priority, TaskId};

use crate::datetime::{delta_time_from_str, offset_date_time_from_str};

fn delta_from_str<T: FromArgValue>(s: &str) -> Result<Delta<T>, String> {
    if s.is_empty() {
        Ok(Delta::Unchanged)
    } else {
        Ok(Delta::Changed(T::from_arg_value(s)?))
    }
}

/// Foo;
#[derive(Debug, FromArgs)]
pub struct Args {
    #[argh(
        option,
        short = 's',
        description = "server where ",
        default = "default_server()",
        from_str_fn(url_from_str)
    )]
    pub server: Url,

    #[argh(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
pub enum Subcommand {
    Add(Add),
    List(List),
    Update(Update),
    Remove(Remove),
}

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "add", description = "")]
pub struct Add {
    #[argh(
        option,
        short = 'p',
        description = "priority for this task",
        default = "Default::default()"
    )]
    pub priority: Priority,

    #[argh(
        option,
        short = 'd',
        description = "date by which this task should be completed",
        from_str_fn(offset_date_time_from_str)
    )]
    pub due_date: Option<OffsetDateTime>,

    #[argh(positional)]
    pub description: String,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "list", description = "")]
pub struct List {}

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "update", description = "")]
pub struct Update {
    #[argh(
        option,
        short = 'c',
        description = "is the task finished?",
        default = "Default::default()",
        from_str_fn(delta_from_str)
    )]
    pub completed: Delta<bool>,

    #[argh(
        option,
        short = 'p',
        description = "priority for this task",
        default = "Default::default()",
        from_str_fn(delta_from_str)
    )]
    pub priority: Delta<Priority>,

    #[argh(
        option,
        short = 'd',
        description = "date by which this task should be completed (use '-' or 'none' to remove)",
        default = "Default::default()",
        from_str_fn(delta_time_from_str)
    )]
    pub due_date: Delta<Option<OffsetDateTime>>,

    #[argh(
        option,
        short = 'i',
        description = "information about this task",
        default = "Default::default()",
        from_str_fn(delta_from_str)
    )]
    pub description: Delta<String>,

    #[argh(positional)]
    pub task_id: TaskId,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "remove", description = "")]
pub struct Remove {
    #[argh(positional)]
    pub task_id: TaskId,
}
