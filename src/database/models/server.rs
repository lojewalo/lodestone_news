use chrono::NaiveDateTime;

use crate::database::schema::*;

insertable! {
  #[derive(Debug, Queryable, Identifiable)]
  pub struct Server,
  #[derive(Debug, Insertable)]
  #[table_name = "servers"]
  pub struct NewServer {
    pub title: String,
    pub url: String,
    pub created: NaiveDateTime,
  }
}
