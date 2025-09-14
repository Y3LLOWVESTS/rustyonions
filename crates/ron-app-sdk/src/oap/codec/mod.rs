#![forbid(unsafe_code)]

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use super::frame::OapFrame;
use crate::constants::{DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME};
use crate::errors::{Error, Result};

mod decoder;
mod encoder;

pub struct OapCodec {
    pub(super) max_frame: usize,
    // Not used in Bronze yet; reserved for COMP guard-rails in Silver.
    pub(super) _max_decompressed: usize,
}

impl Default for OapCodec {
    fn default() -> Self {
        Self {
            max_frame: DEFAULT_MAX_FRAME,
            _max_decompressed: DEFAULT_MAX_DECOMPRESSED,
        }
    }
}

impl OapCodec {
    pub fn new(max_frame: usize, max_decompressed: usize) -> Self {
        Self {
            max_frame,
            _max_decompressed: max_decompressed,
        }
    }
}

impl Decoder for OapCodec {
    type Item = OapFrame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        decoder::decode_frame(self, src)
    }
}

impl Encoder<OapFrame> for OapCodec {
    type Error = Error;

    fn encode(&mut self, item: OapFrame, dst: &mut BytesMut) -> Result<()> {
        encoder::encode_frame(self, item, dst)
    }
}
