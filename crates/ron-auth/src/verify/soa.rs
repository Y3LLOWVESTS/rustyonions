//! SoA (struct-of-arrays) view over a slice of Caveat.

use crate::types::Caveat;
use ipnet::IpNet;
use serde_cbor::Value;
use std::str::FromStr;

pub struct CaveatsSoA<'a> {
    pub exp: Vec<u64>,
    pub nbf: Vec<u64>,
    pub aud: Vec<&'a str>,
    pub method: Vec<&'a Vec<String>>,
    pub path_prefix: Vec<&'a str>,
    pub ip_cidr: Vec<Option<IpNet>>,
    pub bytes_le: Vec<u64>,
    pub rate: Vec<(u64, u32)>,
    pub tenant: Vec<&'a str>,
    pub amnesia: Vec<bool>,
    pub gov_policy_digest: Vec<&'a str>,
    pub custom: Vec<(&'a str, &'a str, &'a Value)>, // (ns, name, cbor)
}

impl<'a> CaveatsSoA<'a> {
    pub fn from_slice(caveats: &'a [Caveat]) -> Self {
        let mut out = Self {
            exp: Vec::new(),
            nbf: Vec::new(),
            aud: Vec::new(),
            method: Vec::new(),
            path_prefix: Vec::new(),
            ip_cidr: Vec::new(),
            bytes_le: Vec::new(),
            rate: Vec::new(),
            tenant: Vec::new(),
            amnesia: Vec::new(),
            gov_policy_digest: Vec::new(),
            custom: Vec::new(),
        };

        for c in caveats {
            match c {
                Caveat::Exp(v) => out.exp.push(*v),
                Caveat::Nbf(v) => out.nbf.push(*v),
                Caveat::Aud(a) => out.aud.push(a.as_str()),
                Caveat::Method(ms) => out.method.push(ms),
                Caveat::PathPrefix(p) => out.path_prefix.push(p.as_str()),
                Caveat::IpCidr(s) => {
                    out.ip_cidr.push(IpNet::from_str(s).ok());
                }
                Caveat::BytesLe(n) => out.bytes_le.push(*n),
                Caveat::Rate { burst, per_s } => out.rate.push((*burst as u64, *per_s)),
                Caveat::Tenant(t) => out.tenant.push(t.as_str()),
                Caveat::Amnesia(b) => out.amnesia.push(*b),
                Caveat::GovPolicyDigest(d) => out.gov_policy_digest.push(d.as_str()),
                Caveat::Custom { ns, name, cbor } => {
                    out.custom.push((ns.as_str(), name.as_str(), cbor))
                }
            }
        }

        out
    }
}
