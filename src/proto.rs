pub mod myfeed_v1 {
    include!(concat!(env!("OUT_DIR"), "/myfeed.v1.rs"));
}

pub use myfeed_v1::*;
