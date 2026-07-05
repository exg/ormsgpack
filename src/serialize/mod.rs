// SPDX-License-Identifier: (Apache-2.0 OR MIT)

mod bytearray;
mod bytes;
mod dataclass;
mod datetime;
mod datetimelike;
mod default;
mod dict;
#[path = "enum.rs"]
mod enum_;
mod ext;
mod fragment;
mod list;
mod memoryview;
mod numpy;
mod pydantic;
mod serializer;
mod state;
mod str;
mod tuple;
mod uuid;
mod writer;

pub use serializer::serialize;
pub use state::State;
