//#![cfg_attr(not(feature = "std"), no_std)]
#![no_std]

//#[cfg(not(feature = "std"))]
extern crate core2;

//#[cfg(not(feature = "std"))]
extern crate alloc;

#[macro_use]
extern crate log;

//#[cfg(not(feature = "std"))]
use core2::io;

/*
#[cfg(feature = "std")]
use std as core;
#[cfg(feature = "std")]
use std::io;
*/

mod buf_stream;
mod stream_slice;

pub use buf_stream::*;
pub use io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
pub use stream_slice::*;
