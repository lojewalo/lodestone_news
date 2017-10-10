use database::models::news_item::NewsItem;
use database::models::send_record::NewSendRecord;
use database::models::server::Server;

use errors::*;

use chrono::{Utc, TimeZone, Duration};

use diesel::prelude::*;
use diesel::expression::sql;
use diesel::insert;

use make_hyper_great_again::Client;
use hyper_rustls::HttpsConnector;
use hyper::header::ContentType;

use std::thread::sleep;
use std::io::Read;

pub struct DiscordSender {
  client: Client<HttpsConnector>
}

impl Default for DiscordSender {
  fn default() -> Self {
    Self::new()
  }
}

impl DiscordSender {
  pub fn new() -> Self {
    DiscordSender {
      client: Client::create_connector(|c| HttpsConnector::new(4, &c.handle())).unwrap()
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
      let mut embed = json!({
        "type": "rich",
        "timestamp": Utc.timestamp(item.created.timestamp(), 0).to_rfc3339(),
        "fields": [
          {
            "name": "Title",
            "value": item.title,
            "inline": false
          },
          {
            "name": "Link",
            "value": item.url,
            "inline": false
          },
          {
            "name": "Kind",
            "value": item.kind.to_string(),
            "inline": true
          }
        ]
      });
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
        .header(ContentType::json())
        .body(data.to_string())
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
      insert(&successful_sends)
        .into(send_records::table)
        .execute(c)
        .chain_err(|| "could not update send records")
    })?;

    Ok(())
  }
}
