use std::io;
use std::os::unix::net::UnixStream;
use rand::Rng;
use ron_bus::api::{Envelope, IndexReq, IndexResp};
use ron_bus::uds::{recv, send};

const DEFAULT_SOCK: &str = "/tmp/ron/svc-index.sock";

pub fn put_address(addr: &str, dir: &str) -> io::Result<()> {
    let sock = std::env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());
    let mut stream = UnixStream::connect(&sock)?;

    let corr_id: u64 = rand::thread_rng().gen();
    let req = IndexReq::PutAddress { addr: addr.to_string(), dir: dir.to_string() };
    let env = Envelope {
        service: "svc.index".into(),
        method: "v1.put".into(),
        corr_id,
        token: vec![],
        payload: rmp_serde::to_vec(&req).unwrap(),
    };

    send(&mut stream, &env).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let reply = recv(&mut stream).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if reply.corr_id != corr_id {
        return Err(io::Error::new(io::ErrorKind::Other, "corr_id mismatch"));
    }

    match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
        Ok(IndexResp::PutOk) => Ok(()),
        _ => Err(io::Error::new(io::ErrorKind::Other, "put failed")),
    }
}
