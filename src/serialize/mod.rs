// SPDX-License-Identifier: (Apache-2.0 OR MIT)

mod attribute_dict;
mod bytes;
mod dataclass;
mod datetime;
mod datetimelike;
mod default;
mod dict;
mod ext;
mod int;
mod list;
mod numpy;
mod serializer;
mod str;
mod tuple;
mod uuid;
mod writer;

pub use serializer::serialize;
