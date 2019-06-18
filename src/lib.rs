#![crate_name = "stomp"]
#![crate_type = "lib"]
#![deny(clippy::needless_lifetimes)]
#![deny(clippy::extra_unused_lifetimes)]

#![rustfmt::skip] //rustfmt reorders the module list, which breaks it.

#[macro_use]
extern crate log;
extern crate bytes;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate unicode_segmentation;
#[macro_use]
extern crate nom;

pub mod codec;
pub mod connection;
pub mod header; // this must come before frame, because it defines header_list!
pub mod frame;
pub mod message_builder;
pub mod option_setter;
pub mod session;
pub mod session_builder;
pub mod subscription;
pub mod subscription_builder;
pub mod transaction;
