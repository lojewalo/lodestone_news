#![allow(clippy::unreadable_literal)]
#![feature(box_syntax)]
#![recursion_limit = "1024"]

#[macro_use] extern crate diesel;
#[macro_use] extern crate log;

use diesel::{
  Connection,
  connection::SimpleConnection,
  sqlite::SqliteConnection,
};

use crossbeam_channel as chan;

use chrono::Duration;

use signal_hook::iterator::Signals;

use self::errors::*;

use std::env;

pub mod iter;
pub mod database;
pub mod lodestone;
pub mod discord;
pub mod errors;
pub mod logging;

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

  let ns_tick = chan::tick(Duration::seconds(150).to_std().unwrap());
  let ds_tick = chan::tick(Duration::minutes(5).to_std().unwrap());

  let mut thread_handles = Vec::new();

  info!("Starting exit thread");

  let (exit_tx, exit_rx) = chan::unbounded();
  let signals = match Signals::new(&[signal_hook::SIGINT]) {
    Ok(s) => s,
    Err(e) => {
      error!("could not register signal handler: {}", e);
      return;
    },
  };

  thread_handles.push(std::thread::spawn(move || {
    for signal in signals.forever() {
      if signal != signal_hook::SIGINT {
        continue;
      }

      for _ in 0..2 {
        exit_tx.send(()).ok();
      }
      break;
    }
  }));

  info!("Starting scraper thread");

  let ns_exit_rx = exit_rx.clone();
  thread_handles.push(std::thread::spawn(move || {
    let scraper = lodestone::NewsScraper::new();
    loop {
      if let Err(e) = scraper.update_news() {
        warn!("Could not update Lodestone news: {}", e);
      }
      #[allow(clippy::all)]
      {
        chan::select! {
          recv(ns_tick) -> _ => {},
          recv(ns_exit_rx) -> _ => break,
        }
      }
    }
  }));

  info!("Starting Discord thread");

  thread_handles.push(std::thread::spawn(move || {
    let ds = discord::DiscordSender::new();
    loop {
      #[allow(clippy::all)]
      {
        chan::select! {
          recv(ds_tick) -> _ => {},
          recv(exit_rx) -> _ => break,
        }
      }
      if let Err(e) = ds.send_new_news() {
        warn!("Could not send Discord news: {}", e);
      }
    }
  }));

  info!("Waiting on joins");

  for handle in thread_handles {
    handle.join().unwrap();
  }

  info!("Done");
}
