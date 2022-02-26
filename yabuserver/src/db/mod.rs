use anyhow::anyhow;
use num_traits::{FromPrimitive, ToPrimitive};
use rusqlite::{params, Connection};
use time::OffsetDateTime;
use yabusame::{Priority, Task, TaskId};

pub const DEFAULT_DATABASE_URL: &str = "yabuserver.db";

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn connect(database_url: &str) -> anyhow::Result<Self> {
        let mut connection = Connection::open(database_url)?;

        // TODO: hardcoded
        connection.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                task_id INTEGER PRIMARY KEY,
                complete INTEGER CHECK(complete <= 1) NOT NULL,
                description TEXT NOT NULL,
                priority INTEGER NOT NULL,
                due_date INTEGER
            )",
            [],
        )?;

        Ok(Self { connection })
    }

    pub fn all_tasks(&self) -> anyhow::Result<Vec<Task>> {
        let mut res = Vec::new();
        let mut statement = self.connection.prepare("SELECT * FROM tasks")?;
        let mut rows = statement.query([])?;

        while let Some(row) = rows.next()? {
            let priority = row.get(3)?;

            res.push(Task::new(
                Some(row.get::<_, u32>(0)?.try_into()?),
                row.get::<_, bool>(1)?,
                row.get::<_, String>(2)?,
                Priority::from_u32(priority)
                    .ok_or_else(|| anyhow!("can't convert {} to a `Priority`", priority))?,
                row.get::<_, Option<i64>>(4)?
                    .map(OffsetDateTime::from_unix_timestamp)
                    .transpose()?,
            ));
        }

        Ok(res)
    }

    pub fn add_task(&self, task: Task) -> anyhow::Result<()> {
        self.connection.execute(
            "INSERT INTO tasks (complete, description, priority, due_date) VALUES (?1, ?2, ?3, ?4)",
            params![
                task.complete,
                task.description,
                task.priority.to_u32(),
                task.due_date.map(|due_date| due_date.unix_timestamp()),
            ],
        )?;

        Ok(())
    }

    pub fn update_task(&self, task_id: TaskId, new_task: Task) -> anyhow::Result<()> {
        self.connection.execute(
            "UPDATE tasks
            SET (complete = ?1, description = ?2, priority = ?3, due_date = ?4)
            WHERE task_id = ?5",
            params![
                new_task.complete,
                new_task.description,
                new_task.priority.to_u32(),
                new_task.due_date.map(|due_date| due_date.unix_timestamp()),
                task_id.0.get(),
            ],
        )?;

        Ok(())
    }

    pub fn remove_task(&self, task_id: TaskId) -> anyhow::Result<()> {
        self.connection
            .execute("DELETE FROM tasks WHERE id=?1", params![task_id.0.get()])?;
        Ok(())
    }
}
