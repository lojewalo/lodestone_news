use database::schema::*;
use database::models::news_item::NewsItem;

insertable! {
  #[derive(Debug, Queryable, Identifiable, Associations)]
  // #[belongs_to(Server)]
  #[belongs_to(NewsItem, foreign_key = "news_id")]
  pub struct SendRecord,
  #[derive(Debug, Insertable)]
  #[table_name = "send_records"]
  pub struct NewSendRecord {
    pub server_id: i32,
    pub news_id: i32
  }
}
