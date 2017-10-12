use make_hyper_great_again::Client;

use scraper::{Html, Selector, ElementRef};

use hyper_rustls::HttpsConnector;

use database::models::news_item::{NewsKind, NewNewsItem};
use errors::*;

use diesel::prelude::*;

use chrono::NaiveDateTime;

use std::io::Read;

const NEWS_URL: &'static str = "https://na.finalfantasyxiv.com/lodestone/news/";

pub struct NewsScraper {
  client: Client<HttpsConnector>
}

impl Default for NewsScraper {
  fn default() -> Self {
    Self::new()
  }
}

impl NewsScraper {
  pub fn new() -> Self {
    NewsScraper {
      client: Client::create_connector(|c| HttpsConnector::new(4, &c.handle())).unwrap()
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
      ::diesel::insert(&new_items)
        .into(news_items::table)
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

      let (title, tag) = match kind {
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

          (title, tag)
        },
        NewsKind::Topic => {
          let text = li.select(&title_selector).next()
            .and_then(|v| v.first_child())
            .and_then(ElementRef::wrap)
            .map(|v| v.text().collect());
          match text {
            Some(t) => (t, None),
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

      let id = match href.split('/').last() {
        Some(i) => i,
        None => {
          warn!("invalid href in news item");
          continue;
        }
      };

      let news_item = NewNewsItem {
        title: title.trim().to_string(),
        url: format!("http://na.finalfantasyxiv.com{}", href),
        lodestone_id: id.to_string(),
        kind: kind,
        created: datetime,
        tag: tag.map(|x| x.trim().to_string())
      };
      items.push(news_item);
    }

    items
  }
}
