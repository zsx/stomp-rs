#![feature(scoped)]
extern crate env_logger;
extern crate stomp;
use stomp::frame::Frame;
use stomp::frame::Transmission;
use stomp::header::{Header, SuppressedHeader, ContentType};
use stomp::subscription::AckOrNack::Ack;
use stomp::subscription::AckMode;
use stomp::connection::{HeartBeat, Credentials};
use stomp::session::ReceiptHandler;
use std::thread;

const TOTAL_MESSAGES : u64 = 10_000;
const INTERVAL : u64 = 100;

fn main() {
  env_logger::init().unwrap();

  let destination = "/queue/sullivan";
  let mut messages_received: u64 = 0;

  let mut subscribe_session = match stomp::session("127.0.0.1", 61613)
    .start() {
      Ok(session) => session,
      Err(error)  => panic!("Could not connect to the server: {}", error)
    };

  let subscription = subscribe_session.subscription(destination, |frame: &Frame| {
    messages_received += 1;
    if messages_received % INTERVAL == 0 {
      println!("{} messages received ", messages_received);
    }
    if messages_received >= TOTAL_MESSAGES {
      println!("Receive complete.");
    }
    Ack
  }).start();

  let join_guard = thread::scoped(move || {
    let mut messages_sent: u64 = 0;
    let mut publish_session = match stomp::session("127.0.0.1", 61613)
      .start() {
        Ok(session) => session,
        Err(error)  => panic!("Could not connect to the server: {}", error)
      };
    loop {
      publish_session.message(destination, "Modern major general")
      .with(ContentType("text/plain"))
      .send();
      messages_sent += 1;
      if messages_sent % INTERVAL == 0 {
        println!("{} messages sent", messages_sent);
      }
      if messages_sent >= TOTAL_MESSAGES {
        println!("Send complete.");
        break;
      }
    }
    publish_session.disconnect();
    println!("Disconnected.");
  });

/*
  loop {
    let _ = match Frame::read(&mut subscribe_session.connection.tcp_stream) {
      Ok(Transmission::HeartBeat) => println!("Heartbeat"),
      Ok(Transmission::CompleteFrame(frame)) => {
        println!("Frame: {}", frame);
        messages_received += 1;
        if messages_received % 100 == 0 || messages_received > 9900 { 
          println!("{} messages received", messages_received);
        }
      },
      _  => println!("Something went wrong.")
    };
  }
*/
  subscribe_session.listen(); // Loops infinitely, awaiting messages

  subscribe_session.disconnect();
}
