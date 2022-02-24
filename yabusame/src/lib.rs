use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    num::{NonZeroU32, TryFromIntError}, fmt::{Display, Formatter, self},
};
use thiserror::Error;
use time::PrimitiveDateTime;

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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Task {
    pub id: Option<TaskId>,
    pub complete: bool,
    pub description: Cow<'static, str>,
    pub priority: u8,
    pub due_date: Option<PrimitiveDateTime>,
}

impl Task {
    pub fn new<S: Into<Cow<'static, str>>>(
        id: Option<TaskId>,
        complete: bool,
        description: S,
        priority: u8,
        due_date: Option<PrimitiveDateTime>,
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
