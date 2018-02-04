alter table news_items rename to old_news_items;

create table news_items (
  id integer primary key not null,
  title text not null,
  url text not null,
  image text,
  lodestone_id text not null,
  kind smallint not null,
  created timestamp not null,
  tag text
);

insert into news_items (id, title, url, lodestone_id, kind, created, tag)
  select id, title, url, lodestone_id, kind, created, tag from old_news_items;

drop table old_news_items;
