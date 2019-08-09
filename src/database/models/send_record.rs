use crate::database::{
  schema::*,
  models::{
    news_item::NewsItem,
    server::Server,
  },
};

#[derive(Debug, Queryable, Associations)]
#[belongs_to(NewsItem, foreign_key = "news_id")]
#[belongs_to(Server, foreign_key = "server_id")]
pub struct SendRecord {
  pub server_id: i32,
  pub news_id: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "send_records"]
pub struct NewSendRecord {
  pub server_id: i32,
  pub news_id: i32,
}
