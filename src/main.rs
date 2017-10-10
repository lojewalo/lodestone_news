#![feature(box_syntax)]
#![recursion_limit = "1024"]

extern crate hyper;
extern crate hyper_rustls;
extern crate make_hyper_great_again;
#[macro_use]
extern crate serde_json;
extern crate scraper;
extern crate chrono;
extern crate dotenv;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate ctrlc;
#[macro_use]
extern crate chan;
extern crate fern;
extern crate ansi_term;

pub mod database;
pub mod lodestone;
pub mod discord;
pub mod errors;
pub mod logging;

use errors::*;

use diesel::Connection;
use diesel::connection::SimpleConnection;
use diesel::sqlite::SqliteConnection;

use chan::{async, tick};

use chrono::Duration;

use std::env;

thread_local! {
  pub static CONNECTION: SqliteConnection = {
    let location = env::var("LN_DATABASE_LOCATION").unwrap();
    let connection = SqliteConnection::establish(&location)
      .chain_err(|| format!("could not connect to sqlite database at {}", location)).unwrap();
    connection.batch_execute("PRAGMA foreign_keys = ON;").chain_err(|| "could not enable foreign keys").unwrap();
    connection
  };
}

fn main() {
  logging::init_logger().expect("Could not initialize logger");

  info!("Loading .env");

  dotenv::dotenv().ok();

  info!("Creating channels and tickers");

  let (ts_tx, ts_rx) = async();
  let ns_tick = tick(Duration::seconds(150).to_std().unwrap());
  let ds_tick = tick(Duration::minutes(5).to_std().unwrap());

  info!("Setting Ctrl-C handler");

  ctrlc::set_handler(move || {
    ts_tx.send(());
    ts_tx.send(());
  }).unwrap();

  info!("Starting scraper thread");

  let ns_ts_rx = ts_rx.clone();
  let news_scraper_handle = ::std::thread::spawn(move || {
    let scraper = lodestone::NewsScraper::new();
    loop {
      chan_select! {
        ns_tick.recv() => {},
        ns_ts_rx.recv() => break,
      }
      if let Err(e) = scraper.update_news() {
        warn!("Could not update Lodestone news: {}", e);
      }
    }
  });

  info!("Starting Discord thread");

  let discord_sender_handle = ::std::thread::spawn(move || {
    let ds = discord::DiscordSender::new();
    loop {
      chan_select! {
        ds_tick.recv() => {},
        ts_rx.recv() => break
      }
      if let Err(e) = ds.send_new_news() {
        warn!("Could not send Discord news: {}", e);
      }
    }
  });

  info!("Waiting on joins");

  news_scraper_handle.join().unwrap();
  discord_sender_handle.join().unwrap();

  info!("Done");
}
