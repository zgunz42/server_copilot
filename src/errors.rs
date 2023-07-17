use std::{fmt::{self, Display}, error::Error};

use teloxide::RequestError;


#[derive(Debug)]
struct MyError {
    message: String,
}

impl Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for MyError {}

impl From<MyError> for RequestError {
    fn from(error: MyError) -> Self {
        RequestError::Api(teloxide::ApiError::BotBlocked)
    }
}