use iter::NewsText;

use reqwest::Client;

use scraper::{Html, Selector, ElementRef};

use database::models::news_item::{NewsKind, NewNewsItem};
use errors::*;

use diesel::prelude::*;
use diesel::dsl::count;

use chrono::NaiveDateTime;

use serde_json;

use std::io::Read;

const NEWS_URL: &'static str = "https://na.finalfantasyxiv.com/lodestone/news/";

pub struct NewsScraper {
  client: Client
}

impl Default for NewsScraper {
  fn default() -> Self {
    Self::new()
  }
}

impl NewsScraper {
  pub fn new() -> Self {
    NewsScraper {
      client: Client::new()
    }
  }

  pub fn update_news(&self) -> Result<()> {
    let news = self.download_news()?;
    let parsed = self.parse_news(&news);
    NewsScraper::insert_new_news(parsed)
  }

  pub fn insert_new_news(items: Vec<NewNewsItem>) -> Result<()> {
    info!("Checking for new items");
    let new_ids: Vec<String> = items.iter().map(|x| x.lodestone_id.to_string()).collect();
    let existing_ids: Vec<String> = ::CONNECTION.with(|c| {
      use database::schema::news_items;
      news_items::table.select(news_items::lodestone_id)
        .filter(news_items::lodestone_id.eq_any(&new_ids))
        .load(c)
        .chain_err(|| "could not load existing ids")
    })?;
    let new_items: Vec<NewNewsItem> = items.into_iter()
      .filter(|x| !existing_ids.contains(&x.lodestone_id))
      .collect();
    if new_items.is_empty() {
      info!("No new items found");
      return Ok(());
    }
    ::CONNECTION.with(|c| {
      use database::schema::news_items;
      ::diesel::insert_into(news_items::table)
        .values(&new_items)
        .execute(c)
        .chain_err(|| "could not insert new items")
    })?;
    info!("Added {} new item{}", new_items.len(), if new_items.len() == 1 { "" } else { "s" });
    Ok(())
  }

  pub fn download_news(&self) -> Result<String> {
    info!("Downloading news");
    let mut response = self.client.get(NEWS_URL).send().chain_err(|| "could not download news")?;

    let mut content = String::new();
    response.read_to_string(&mut content).chain_err(|| "could not read news")?;
    Ok(content)
  }

  pub fn parse_news(&self, news: &str) -> Vec<NewNewsItem> {
    info!("Parsing news");
    let html = Html::parse_document(news);
    let special_notices_selector = Selector::parse("div.news__content.parts__space--add > ul:nth-of-type(1) > li").unwrap();
    let news_selector = Selector::parse("div.news__content.parts__space--add > ul:nth-of-type(2) > li").unwrap();
    let topics_selector = Selector::parse("div.news__content.parts__space--add > ul:nth-of-type(3) > li").unwrap();
    let title_selector = Selector::parse("p.news__list--title").unwrap();
    let time_script_selector = Selector::parse("time.news__list--time > script").unwrap();
    let first_image_selector = Selector::parse("img:nth-of-type(1)").unwrap();
    let second_para_selector = Selector::parse("p:nth-of-type(2)").unwrap();

    let mut lis: Vec<_> = html.select(&news_selector).map(|x| (NewsKind::News, x)).collect();
    lis.append(&mut html.select(&topics_selector).map(|x| (NewsKind::Topic, x)).collect());
    lis.append(&mut html.select(&special_notices_selector).map(|x| (NewsKind::SpecialNotice, x)).collect());

    let mut items = Vec::with_capacity(lis.len());
    for (kind, li) in lis {
      let child = match kind {
        NewsKind::News | NewsKind::SpecialNotice => li.first_child().and_then(|v| v.value().as_element()),
        NewsKind::Topic => li.select(&title_selector).next().and_then(|v| v.first_child().and_then(|x| x.value().as_element())),
      };

      let child = match child {
        Some(c) => c,
        None => {
          warn!("could not get news item child");
          continue
        }
      };

      let href = match child.attr("href") {
        Some(h) => h,
        None => {
          warn!("invalid link in news item");
          continue;
        }
      };
      let url = format!("https://na.finalfantasyxiv.com{}", href);

      let url = format!("https://na.finalfantasyxiv.com{}", href);

      let id = match href.split('/').last() {
        Some(i) => i,
        None => {
          warn!("invalid href in news item");
          continue;
        }
      };

      let count: Result<i64> = ::CONNECTION.with(|c| {
        use database::schema::news_items;
        news_items::table.select(count(news_items::id))
          .filter(news_items::lodestone_id.eq(&id))
          .first(c)
          .chain_err(|| "could not load existing ids")
      });

      match count {
        Ok(i) if i > 0 => continue,
        Err(e) => {
          warn!("could not check if id was in database: {}", e);
          continue;
        },
        _ => {}
      }

      let (title, tag, image, description, fields) = match kind {
        NewsKind::News | NewsKind::SpecialNotice => {
          let title = match li.select(&title_selector).next() {
            Some(t) => t,
            None => {
              warn!("missing title in news item");
              continue;
            }
          };

          let tag: Option<String> = title.first_child()
            .and_then(ElementRef::wrap)
            .map(|c| c.text().collect::<String>())
            .map(|tag| tag[1..tag.len() - 1].to_string());

          let text_iter = title.text();
          let title: String = if tag.is_some() {
            text_iter.skip(1).collect()
          } else {
            text_iter.collect()
          };

          let (desc, fields) = match self.parse_news_fields(&url) {
            Ok(x) => x,
            Err(e) => {
              warn!("could not parse fields: {}", e);
              continue;
            }
          };

          let fields = match serde_json::to_string(&fields).chain_err(|| "could not serialize") {
            Ok(f) => f,
            Err(e) => {
              warn!("could not parse/serialize fields: {}", e);
              continue;
            }
          };

          (title, tag, None, desc, Some(fields))
        },
        NewsKind::Topic => {
          let text = li.select(&title_selector).next()
            .and_then(|v| v.first_child())
            .and_then(ElementRef::wrap)
            .map(|v| v.text().collect());
          let image = li.select(&first_image_selector).next()
            .and_then(|e| e.value().attr("src"))
            .map(ToString::to_string);
          let description = li.select(&second_para_selector).next()
            .map(|v| NewsText::new(v.traverse(), " ").collect());
          match text {
            Some(t) => (t, None, image, description, None),
            None => {
              warn!("invalid topic/special notice: no title");
              continue;
            }
          }
        }
      };

      let time_script = match li.select(&time_script_selector).next() {
        Some(ts) => ts,
        None => {
          warn!("news item missing time script");
          continue
        }
      };

      let time_script: String = time_script.text().collect();
      let time_string = match time_script.split("strftime(").nth(1).and_then(|v| v.split(',').next()) {
        Some(time) => time,
        None => {
          warn!("invalid script in news item");
          continue
        }
      };
      let time: i64 = match time_string.parse() {
        Ok(t) => t,
        Err(_) => {
          warn!("invalid time in time script");
          continue
        }
      };
      let datetime = NaiveDateTime::from_timestamp(time, 0);

      let news_item = NewNewsItem {
        title: title.trim().to_string(),
        url,
        image,
        description,
        fields,
        lodestone_id: id.to_string(),
        kind,
        created: datetime,
        tag: tag.map(|x| x.trim().to_string())
      };
      items.push(news_item);
    }

    items
  }

  fn parse_news_fields(&self, url: &str) -> Result<(Option<String>, Vec<Field>)> {
    let mut response = self.client.get(url).send().chain_err(|| "could not download news item")?;
    let mut content = String::new();
    response.read_to_string(&mut content).chain_err(|| "could not read news item")?;

    let detail_selector = Selector::parse("div.news__detail__wrapper").unwrap();

    let html = Html::parse_document(&content);
    let content = html.select(&detail_selector).next().chain_err(|| "no content")?;
    let text: String = NewsText::new(content.traverse(), "\n").collect();

    let mut fields = Vec::new();
    let mut title: Option<&str> = None;
    let mut value = String::new();

    let desc = text.split('\n').next().map(|x| x.trim().to_string());

    for line in text.split('\n') {
      let line = line.trim();
      if let Some(t) = title {
        if line.is_empty() {
          title = None;
          if !value.is_empty() {
            fields.push(Field { name: t.to_string(), value: value.trim().to_string() });
            value.clear();
          }
          continue;
        }
        value.push_str(line);
        value.push_str("\n");
      }
      if !line.starts_with('[') || !line.ends_with(']') {
        continue;
      }
      title = Some(&line[1..line.len() - 1]);
    }
    Ok((desc, fields))
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Field {
  pub name: String,
  pub value: String
}
