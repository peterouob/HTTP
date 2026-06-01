use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RouterError {
    DuplicateKey,
    NullKey,
    NullValue,
}

pub(crate) type RouterResult<T> = Result<T, RouterError>;

impl RouterError {
    #[inline]
    fn description_str(&self) -> &'static str {
        match *self {
            RouterError::DuplicateKey => "Key already exists",
            RouterError::NullKey => "Key does not exist",
            RouterError::NullValue => "Value does not exist",
        }
    }
}

impl fmt::Display for RouterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description_str())
    }
}

impl std::error::Error for RouterError {
    fn description(&self) -> &str {
        self.description_str()
    }
}
