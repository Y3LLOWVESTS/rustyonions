//! RO:WHAT — Minimal OAP/1 handshake over any AsyncRead/Write stream.
//! RO:WHY  — Establish version & capability agreement before frames.
//! RO:INVARIANTS — Fixed-size preamble; bounded IO; timeout guarded.

use crate::protocol::error::{ProtoError, ProtoResult};
use crate::protocol::flags::Caps;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::{timeout, Duration};

const MAGIC: &[u8; 4] = b"OAP1";
const WIRE_HELLO_LEN: usize = 4 /*MAGIC*/ + 1 /*ver*/ + 4 /*caps*/;
pub const VERSION: u8 = 1;

#[derive(Debug, Clone, Copy)]
pub struct Negotiated {
    pub version: u8,
    pub caps: Caps,
}

fn encode_hello(buf: &mut [u8; WIRE_HELLO_LEN], ver: u8, caps: Caps) {
    buf[0..4].copy_from_slice(MAGIC);
    buf[4] = ver;
    buf[5..9].copy_from_slice(&(caps.bits()).to_be_bytes());
}

fn decode_hello(buf: &[u8; WIRE_HELLO_LEN]) -> ProtoResult<(u8, Caps)> {
    if &buf[0..4] != MAGIC {
        return Err(ProtoError::BadPreamble {
            got: [buf[0], buf[1], buf[2], buf[3], buf[4]],
        });
    }
    let ver = buf[4];
    let mut caps_b = [0u8; 4];
    caps_b.copy_from_slice(&buf[5..9]);
    let caps = u32::from_be_bytes(caps_b);
    Ok((ver, Caps::from_bits_truncate(caps)))
}

/// Perform a 1-RTT symmetric hello exchange with a timeout.
pub async fn handshake<IO>(io: &mut IO, ours: Caps, dur: Duration) -> ProtoResult<Negotiated>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf_out = [0u8; WIRE_HELLO_LEN];
    let mut buf_in = [0u8; WIRE_HELLO_LEN];

    encode_hello(&mut buf_out, VERSION, ours);

    let fut = async {
        // Write our hello, then flush
        io.write_all(&buf_out).await?;
        io.flush().await?;

        // Read peer hello (exact len)
        io.read_exact(&mut buf_in).await?;
        ProtoResult::Ok(())
    };

    timeout(dur, fut)
        .await
        .map_err(|_| ProtoError::HandshakeTimeout)??;

    let (peer_ver, peer_caps) = decode_hello(&buf_in)?;
    if peer_ver != VERSION {
        return Err(ProtoError::BadPreamble {
            got: [buf_in[0], buf_in[1], buf_in[2], buf_in[3], buf_in[4]],
        });
    }

    // Minimal check: both must support GOSSIP_V1 for now.
    let needed = Caps::GOSSIP_V1;
    if !peer_caps.contains(needed) || !ours.contains(needed) {
        return Err(ProtoError::CapabilityMismatch);
    }

    Ok(Negotiated {
        version: peer_ver,
        caps: peer_caps & ours,
    })
}
