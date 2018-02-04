use database::schema::*;

use std::borrow::Borrow;
use std::error::Error;

use diesel::Queryable;
use diesel::types::{FromSql, FromSqlRow, HasSqlType};
use diesel::sql_types::SmallInt;
use diesel::expression::AsExpression;
use diesel::expression::helper_types::AsExprOf;
use diesel::backend::Backend;
use diesel::row::Row;
use diesel::sqlite::Sqlite;

use chrono::NaiveDateTime;

use database::models::SqlError;

insertable! {
  #[derive(Debug, Queryable, Identifiable)]
  pub struct NewsItem,
  #[derive(Debug, Insertable)]
  #[table_name = "news_items"]
  pub struct NewNewsItem {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub image: Option<String>,
    pub lodestone_id: String,
    pub kind: NewsKind,
    pub created: NaiveDateTime,
    pub tag: Option<String>
  }
}

#[derive(Debug)]
pub enum NewsKind {
  SpecialNotice,
  News,
  Topic
}

impl NewsKind {
  fn as_i16(&self) -> i16 {
    match *self {
      NewsKind::SpecialNotice => 0,
      NewsKind::News => 1,
      NewsKind::Topic => 2
    }
  }

  fn from_i16(i: i16) -> Option<NewsKind> {
    match i {
      0 => Some(NewsKind::SpecialNotice),
      1 => Some(NewsKind::News),
      2 => Some(NewsKind::Topic),
      _ => None
    }
  }

  pub fn color<S: Borrow<String>>(&self, tag: Option<S>) -> Option<u32> {
    if let Some(tag) = tag {
      match tag.borrow().to_lowercase().as_str() {
        "maintenance" => return 0xd3730c.into(),
        "important" => return 0xd30c0c.into(),
        "recovery" => return 0x34d30c.into(),
        "follow-up" => return 0x0c80d3.into(),
        _ => {}
      }
    }
    match *self {
      NewsKind::News => None,
      NewsKind::SpecialNotice => 0x0cd3cd.into(),
      NewsKind::Topic => 0x620cd3.into(),
    }
  }
}

impl ToString for NewsKind {
  fn to_string(&self) -> String {
    match *self {
      NewsKind::News => "News",
      NewsKind::Topic => "Topic",
      NewsKind::SpecialNotice => "Special notice"
    }.to_string()
  }
}

impl<DB> Queryable<SmallInt, DB> for NewsKind
  where DB: Backend + HasSqlType<SmallInt>,
        NewsKind: FromSql<SmallInt, DB>
{
  type Row = Self;

  fn build(row: Self::Row) -> Self {
    row
  }
}

impl FromSql<SmallInt, Sqlite> for NewsKind {
  fn from_sql(bytes: Option<&<Sqlite as Backend>::RawValue>) -> Result<Self, Box<Error + Send + Sync>> {
    let bytes = match bytes {
      Some(b) => b,
      None => return Err(box SqlError::new("unexpected null"))
    };
    let u = bytes.read_integer() as i16;
    match NewsKind::from_i16(u) {
      Some(n) => Ok(n),
      None => Err(box SqlError::new("unknown news kind"))
    }
  }
}

impl<DB> FromSqlRow<SmallInt, DB> for NewsKind
  where DB: Backend + HasSqlType<SmallInt>,
        NewsKind: FromSql<SmallInt, DB>
{
  fn build_from_row<T: Row<DB>>(row: &mut T) -> Result<Self, Box<Error + Send + Sync>> {
    FromSql::from_sql(row.take())
  }
}

impl<'a> AsExpression<SmallInt> for &'a NewsKind {
  type Expression = AsExprOf<i16, SmallInt>;

  fn as_expression(self) -> Self::Expression {
    AsExpression::<SmallInt>::as_expression(self.as_i16())
  }
}
