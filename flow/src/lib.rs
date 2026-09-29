use std::{error::Error, fmt::Display, io};

mod file_writer;
mod instance;
mod machine;
mod storage;

pub use instance::{Instance, InstanceSavePolicy, Move};
pub use machine::{Machine, State, StateName};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(crate) fn user_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}

pub(crate) trait ResultContext<T> {
    fn context(self, message: impl Display) -> Result<T>;
    fn with_context(self, message: impl FnOnce() -> String) -> Result<T>;
}

impl<T, E: Display> ResultContext<T> for std::result::Result<T, E> {
    fn context(self, message: impl Display) -> Result<T> {
        self.map_err(|error| user_error(format!("{message}: {error}")))
    }

    fn with_context(self, message: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|error| user_error(format!("{}: {error}", message())))
    }
}

pub(crate) trait OptionContext<T> {
    fn context(self, message: impl Display) -> Result<T>;
    fn with_context(self, message: impl FnOnce() -> String) -> Result<T>;
}

impl<T> OptionContext<T> for Option<T> {
    fn context(self, message: impl Display) -> Result<T> {
        self.ok_or_else(|| user_error(message.to_string()))
    }

    fn with_context(self, message: impl FnOnce() -> String) -> Result<T> {
        self.ok_or_else(|| user_error(message()))
    }
}
