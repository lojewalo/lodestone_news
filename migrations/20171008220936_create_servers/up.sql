create table servers (
  id integer primary key not null,
  title text not null,
  url text not null,
  created timestamp not null default current_timestamp
)
