#![allow(unused)]
#![feature(let_chains, try_blocks)]
#![warn(unused_imports, unused_must_use)]

mod args;
mod datetime;

use args::Subcommand;
use comfy_table::{presets::NOTHING, Attribute, Cell, CellAlignment, Color, Table};
use std::borrow::Cow;
use time::format_description;
use yabusame::{connection::ClientConnection, Delta, Message, Priority, Response, Task, TaskDelta};

use crate::args::Args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = argh::from_env::<Args>();
    let mut connection = ClientConnection::new(&args.server).await?;

    let message = match args.subcommand {
        Subcommand::Add(new_args) => Message::Add(Task::new(
            None,
            false,
            new_args.description,
            new_args.priority,
            new_args.due_date,
        )),

        Subcommand::List(_) => Message::List,

        Subcommand::Update(update_args) => Message::Update(
            update_args.task_id,
            TaskDelta {
                complete: update_args.completed,
                // why doesn't `Cow<'static, str>: FromStr`?
                description: match update_args.description {
                    Delta::Unchanged => Delta::Unchanged,
                    Delta::Changed(s) => Delta::Changed(s.into()),
                },
                priority: update_args.priority,
                due_date: update_args.due_date,
            },
        ),

        Subcommand::Remove(remove_args) => Message::Remove(remove_args.task_id),
    };

    match connection.send(message).await? {
        Response::Nothing => {}

        Response::Tasks(tasks) => {
            if tasks.is_empty() {
                println!("you have no tasks; use `yabu add [description]` to add one");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(NOTHING).set_header(vec![
                "task",
                "fin",
                "description",
                "priority",
                "due date",
            ]);

            table
                .get_column_mut(0)
                .unwrap()
                .set_cell_alignment(CellAlignment::Right);

            table
                .get_column_mut(1)
                .unwrap()
                .set_cell_alignment(CellAlignment::Center);

            let date_time_format = format_description::parse(
                "[year]-[month]-[day] [hour padding:none repr:12]:[minute][period case:lower]",
            )?;

            for task in tasks {
                let completed = if task.complete { "X" } else { " " };

                let priority = match task.priority {
                    Priority::Lowest => Cell::new("lowest"),
                    Priority::Low => Cell::new("low").fg(Color::Blue),
                    Priority::Medium => Cell::new("medium").fg(Color::DarkMagenta),
                    Priority::High => Cell::new("high").fg(Color::Yellow),
                    Priority::Critical => Cell::new("critical")
                        .fg(Color::Red)
                        .add_attribute(Attribute::Bold),
                };

                let due_date: Cow<'static, str> = match task.due_date {
                    Some(due_date) => due_date.format(&date_time_format)?.into(),
                    None => "".into(),
                };

                let mut description = Cell::new(&task.description);

                if task.complete {
                    description = description.add_attribute(Attribute::CrossedOut);
                }

                table.add_row(vec![
                    Cell::new(task.id_or_error()?.to_string()),
                    Cell::new(completed),
                    description,
                    priority,
                    Cell::new(due_date),
                ]);
            }

            println!("{table}");
        }

        Response::Error(err) => return Err(err.into()),
    }

    Ok(())
}
