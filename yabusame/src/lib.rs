#![feature(derive_default_enum)]

use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    num::{NonZeroU32, TryFromIntError},
    str::FromStr,
};
use thiserror::Error;
use time::{OffsetDateTime};

pub const DEFAULT_SERVER_PORT: u16 = 11180;
pub const URL_SCHEME: &str = "yabu";

#[derive(Clone, Debug, Error)]
pub enum YabuError {
    #[error("task id cannot be 0")]
    TaskIdZero(#[from] TryFromIntError),
    #[error("tried to get the id of a task that didn't have one")]
    TaskHasNoId,
    #[error("task {0} does not exist")]
    TaskDoesntExist(TaskId),
    // TODO: show available priorities
    #[error("unknown priority {0}")]
    UnknownPriority(String),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(transparent)]
pub struct TaskId(pub NonZeroU32);

impl Display for TaskId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl TryFrom<u32> for TaskId {
    type Error = YabuError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(Self(NonZeroU32::try_from(value)?))
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Priority {
    Lowest,
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

impl FromStr for Priority {
    type Err = YabuError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lowest" => Ok(Self::Lowest),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => Err(YabuError::UnknownPriority(s.to_string())),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Task {
    pub id: Option<TaskId>,
    pub complete: bool,
    pub description: Cow<'static, str>,
    pub priority: Priority,
    pub due_date: Option<OffsetDateTime>,
}

impl Task {
    pub fn new<S: Into<Cow<'static, str>>>(
        id: Option<TaskId>,
        complete: bool,
        description: S,
        priority: Priority,
        due_date: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            id,
            complete,
            description: description.into(),
            priority,
            due_date,
        }
    }

    pub fn id_or_error(&self) -> Result<TaskId, YabuError> {
        self.id.ok_or(YabuError::TaskHasNoId)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Message {
    New(Task),
    List,
    Update(TaskId, Task),
    Remove(TaskId),
}
