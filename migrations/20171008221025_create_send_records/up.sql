pragma foreign_keys = on;
create table send_records (
  server_id integer not null,
  news_id integer not null,

  primary key(server_id, news_id),

  foreign key(server_id) references servers(id),
  foreign key(news_id) references news_items(id)
)
