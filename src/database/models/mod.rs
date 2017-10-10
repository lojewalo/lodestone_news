macro_rules! insertable {
  ($(#[$($meta:meta),+])* pub struct $name:ident, $(#[$($new_meta:meta),+])* pub struct $new_name:ident {
    $(pub $field_name:ident: $kind:ty),+
  }) => {
    $(#[$($meta),+])*
    pub struct $name {
      pub id: i32,
      $(pub $field_name: $kind),+
    }

    $(#[$($new_meta),+])*
    pub struct $new_name {
      $(pub $field_name: $kind),+
    }
  }
}

use std::error::Error;
use std::fmt::{Display, Formatter, Error as FmtError};

pub mod news_item;
pub mod server;
pub mod send_record;

#[derive(Debug)]
pub struct SqlError(String);

impl SqlError {
  pub fn new<S: AsRef<str>>(s: S) -> Self {
    SqlError(s.as_ref().to_string())
  }
}

impl Display for SqlError {
  fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
    write!(f, "{}", self.0)
  }
}

impl Error for SqlError {
  fn description(&self) -> &str {
    "there was an sql error"
  }

  fn cause(&self) -> Option<&Error> {
    None
  }
}
