use database::models::news_item::NewsItem;
use database::models::send_record::NewSendRecord;
use database::models::server::Server;

use errors::*;

use chrono::{Utc, Duration, DateTime};

use diesel::prelude::*;
use diesel::dsl::sql;
use diesel::insert_into;

use reqwest::Client;

use serde_json;

use std::thread::sleep;
use std::io::Read;

pub struct DiscordSender {
  client: Client
}

impl Default for DiscordSender {
  fn default() -> Self {
    Self::new()
  }
}

impl DiscordSender {
  pub fn new() -> Self {
    DiscordSender {
      client: Client::new()
    }
  }

  pub fn send_new_news(&self) -> Result<()> {
    let to_send: Vec<(Server, NewsItem)> = ::CONNECTION.with(|c| {
      use database::schema::{servers, news_items};
      sql::<(servers::SqlType, news_items::SqlType)>("select
        servers.*, news_items.*
        from servers, news_items
        where (servers.id, news_items.id) not in (select server_id, news_id from send_records)
        and news_items.created >= servers.created;")
        .load(c)
        .chain_err(|| "could not load items to send")
    })?;

    let mut successful_sends = Vec::new();

    for (server, item) in to_send {
      info!("Sending {} ({}) to {} ({})", item.title, item.id, server.title, server.id);
      let mut embed = json!({
        "type": "rich",
        "timestamp": DateTime::<Utc>::from_utc(item.created, Utc).to_rfc3339(),
        "color": item.kind.color(item.tag.as_ref()),
        "title": item.title,
        "url": item.url,
        "description": item.description,
        "fields": []
      });
      if let Some(ref image) = item.image {
        embed["image"] = json!({
          "url": image
        });
      }
      if let Some(ref fields) = item.fields {
        if let Ok(f) = serde_json::from_str::<Vec<serde_json::Value>>(fields) {
          embed["fields"].as_array_mut().unwrap().extend(f);
        }
      }
      embed["fields"].as_array_mut().unwrap().push(json!({
        "name": "Kind",
        "value": item.kind.to_string(),
        "inline": true
      }));
      if let Some(ref tag) = item.tag {
        embed["fields"].as_array_mut().unwrap().push(json!({
          "name": "Tag",
          "value": tag,
          "inline": true
        }));
      }
      let data = json!({
        "embeds": [embed]
      });
      let res = self.client.post(&server.url)
        .json(&data)
        .send();
      let mut data = match res {
        Ok(r) => r,
        Err(e) => {
          warn!("Error sending news item {} to server {}: {}", item.id, server.id, e);
          continue;
        }
      };
      let mut content = String::new();
      if let Err(e) = data.read_to_string(&mut content) {
        warn!("Could not read webhook response when sending item {} to sever {}: {}", item.id, server.id, e);
        continue;
      }
      if !data.status().is_success() {
        warn!("Webhook send was not successful for item {} on server {}. Content below:", item.id, server.id);
        warn!("{}", content);
      } else {
        trace!("Webhook send successful for item {} on server {}", item.id, server.id);
        successful_sends.push(NewSendRecord {
          server_id: server.id,
          news_id: item.id
        });
      }
      sleep(Duration::seconds(1).to_std().unwrap());
    }

    ::CONNECTION.with(|c| {
      use database::schema::send_records;
      insert_into(send_records::table)
        .values(&successful_sends)
        .execute(c)
        .chain_err(|| "could not update send records")
    })?;

    Ok(())
  }
}
