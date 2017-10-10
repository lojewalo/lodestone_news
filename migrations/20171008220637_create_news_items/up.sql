create table news_items (
  id integer primary key not null,
  title text not null,
  url text not null,
  lodestone_id text not null,
  kind smallint not null,
  created timestamp not null,
  tag text
)
