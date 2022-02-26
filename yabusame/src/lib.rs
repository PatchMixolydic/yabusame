#![feature(derive_default_enum)]

use num_derive::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeJsonError;
use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    io::Error as IoError,
    mem,
    num::{NonZeroU32, ParseIntError, TryFromIntError},
    str::FromStr,
};
use thiserror::Error;
use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const DEFAULT_SERVER_PORT: u16 = 11180;
pub const URL_SCHEME: &str = "yabu";

#[derive(Debug, Error)]
pub enum YabuError {
    #[error("task id cannot be 0")]
    TaskIdZero(#[from] TryFromIntError),
    #[error("tried to get the id of a task that didn't have one")]
    TaskHasNoId,
    #[error("task {0} does not exist")]
    TaskDoesntExist(TaskId),
    #[error("unknown priority {0}")]
    UnknownPriority(String),
    #[error("io error")]
    IoError(#[from] IoError),
    #[error("error while serializing a value")]
    SerializationError(#[from] SerdeJsonError),
}

#[derive(Clone, Debug, Deserialize, Error, Serialize)]
pub enum YabuRpcError {
    #[error("task {0} does not exist")]
    TaskDoesntExist(TaskId),
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

impl FromStr for TaskId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(NonZeroU32::from_str(s)?))
    }
}

impl TryFrom<u32> for TaskId {
    type Error = YabuError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(Self(NonZeroU32::try_from(value)?))
    }
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, FromPrimitive, PartialEq, Serialize, ToPrimitive,
)]
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
        // All match arms are ASCII-only, so this should be fine
        // (and faster)
        match s.to_ascii_lowercase().as_str() {
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

    pub fn apply_delta(&mut self, delta: TaskDelta) {
        // trying to use `self.description` directly fails borrowcheck
        let mut description = Cow::Borrowed("");
        mem::swap(&mut self.description, &mut description);

        self.complete = delta.complete.apply_to(self.complete);
        self.description = delta.description.apply_to(description);
        self.priority = delta.priority.apply_to(self.priority);
        self.due_date = delta.due_date.apply_to(self.due_date);
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub enum Delta<T> {
    #[default]
    Unchanged,
    Changed(T),
}

impl<T> Delta<T> {
    #[must_use = "this returns the new value without changing the original"]
    pub fn apply_to(self, old: T) -> T {
        match self {
            Delta::Unchanged => old,
            Delta::Changed(new) => new,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TaskDelta {
    pub complete: Delta<bool>,
    pub description: Delta<Cow<'static, str>>,
    pub priority: Delta<Priority>,
    pub due_date: Delta<Option<OffsetDateTime>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Message {
    Add(Task),
    List,
    Update(TaskId, TaskDelta),
    Remove(TaskId),
}

impl Message {
    pub async fn read_from_socket<R: AsyncReadExt + Unpin>(
        mut socket: R,
    ) -> Result<Self, YabuError> {
        // read the payload length
        let mut length_buf = [0u8; 2];
        socket.read_exact(&mut length_buf).await?;
        let length = usize::from(u16::from_le_bytes(length_buf));

        // now for the payload
        let mut buf = vec![0; length];
        socket.read_exact(&mut buf).await?;
        Ok(serde_json::from_slice::<Self>(&buf)?)
    }

    pub async fn write_to_socket<W: AsyncWriteExt + Unpin>(
        &self,
        mut socket: W,
    ) -> Result<(), YabuError> {
        let buffer = serde_json::to_vec(self)?;

        // first, write out the payload length
        let length_bytes = u16::try_from(buffer.len())?.to_le_bytes();
        socket.write_all(&length_bytes).await?;

        // ...then write the payload
        socket.write_all(&buffer).await?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Response {
    Nothing,
    Tasks(Vec<Task>),
    Error(YabuRpcError),
}

impl Response {
    pub async fn read_from_socket<R: AsyncReadExt + Unpin>(
        mut socket: R,
    ) -> Result<Self, YabuError> {
        // read the payload length
        let mut length_buf = [0u8; 2];
        socket.read_exact(&mut length_buf).await?;
        let length = usize::from(u16::from_le_bytes(length_buf));

        // now for the payload
        let mut buf = vec![0; length];
        socket.read_exact(&mut buf).await?;
        Ok(serde_json::from_slice::<Self>(&buf)?)
    }

    pub async fn write_to_socket<W: AsyncWriteExt + Unpin>(
        &self,
        mut socket: W,
    ) -> Result<(), YabuError> {
        let buffer = serde_json::to_vec(self)?;

        // first, write out the payload length
        let length_bytes = u16::try_from(buffer.len())?.to_le_bytes();
        socket.write_all(&length_bytes).await?;

        // ...then write the payload
        socket.write_all(&buffer).await?;
        Ok(())
    }
}
