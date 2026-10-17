//! The small part of the DNS wire format (RFC 1035) the sinkhole needs:
//! parse a single-question query and build replies that echo it.

use std::net::{Ipv4Addr, Ipv6Addr};

const HEADER_LEN: usize = 12;
const MAX_NAME_LEN: usize = 255;
const CLASS_IN: u16 = 1;

/// A response code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Rcode {
    /// No error.
    NoError = 0,
    /// The query was malformed.
    FormErr = 1,
    /// The name does not exist.
    NxDomain = 3,
    /// The kind of query is not supported.
    NotImp = 4,
    /// The server will not answer this.
    Refused = 5,
}

/// The record types the sinkhole answers specially.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    /// IPv4 address.
    A,
    /// IPv6 address.
    Aaaa,
    /// Anything else, by number.
    Other(u16),
}

impl From<u16> for RecordType {
    fn from(value: u16) -> Self {
        match value {
            1 => Self::A,
            28 => Self::Aaaa,
            other => Self::Other(other),
        }
    }
}

/// Why a packet could not be parsed as a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ParseError {
    /// Too short to even echo an id; drop it.
    Unanswerable,
    /// The header is readable but the rest is not.
    Malformed { header: [u8; HEADER_LEN] },
    /// A valid header asking for something other than a standard query.
    NotImplemented { header: [u8; HEADER_LEN] },
}

/// A parsed single-question query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    header: [u8; HEADER_LEN],
    /// The question section exactly as received.
    question: Vec<u8>,
    name: String,
    record_type: RecordType,
    class: u16,
}

impl Query {
    /// Parses a query with exactly one question and an uncompressed name.
    pub(super) fn parse(packet: &[u8]) -> Result<Self, ParseError> {
        let header: [u8; HEADER_LEN] = packet
            .get(..HEADER_LEN)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(ParseError::Unanswerable)?;
        let malformed = ParseError::Malformed { header };
        let flags = u16::from_be_bytes([header[2], header[3]]);
        if flags & 0x8000 != 0 {
            return Err(ParseError::Unanswerable);
        }
        if (flags >> 11) & 0xF != 0 {
            return Err(ParseError::NotImplemented { header });
        }
        if u16::from_be_bytes([header[4], header[5]]) != 1 {
            return Err(malformed);
        }

        let mut labels: Vec<String> = Vec::new();
        let mut pos = HEADER_LEN;
        let mut name_len = 0usize;
        loop {
            let len = usize::from(*packet.get(pos).ok_or_else(|| malformed.clone())?);
            pos += 1;
            if len == 0 {
                break;
            }
            if len > 63 {
                return Err(malformed);
            }
            let label = packet
                .get(pos..pos + len)
                .ok_or_else(|| malformed.clone())?;
            name_len += len + 1;
            if name_len > MAX_NAME_LEN {
                return Err(malformed);
            }
            labels.push(String::from_utf8_lossy(label).into_owned());
            pos += len;
        }
        let tail = packet.get(pos..pos + 4).ok_or_else(|| malformed.clone())?;
        let record_type = RecordType::from(u16::from_be_bytes([tail[0], tail[1]]));
        let class = u16::from_be_bytes([tail[2], tail[3]]);
        pos += 4;

        Ok(Self {
            header,
            question: packet[HEADER_LEN..pos].to_vec(),
            name: labels.join("."),
            record_type,
            class,
        })
    }

    /// The queried name, without a trailing dot.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The queried record type.
    #[must_use]
    pub fn record_type(&self) -> RecordType {
        self.record_type
    }

    /// A reply with no answers.
    pub(super) fn reply(&self, rcode: Rcode) -> Vec<u8> {
        self.build(rcode, None)
    }

    /// The sinkhole reply: the unspecified address for A and AAAA in class
    /// IN, an empty NOERROR answer otherwise.
    pub(super) fn sinkhole_reply(&self, ttl: u32) -> Vec<u8> {
        let data: Option<(u16, Vec<u8>)> = match (self.record_type, self.class) {
            (RecordType::A, CLASS_IN) => Some((1, Ipv4Addr::UNSPECIFIED.octets().to_vec())),
            (RecordType::Aaaa, CLASS_IN) => Some((28, Ipv6Addr::UNSPECIFIED.octets().to_vec())),
            _ => None,
        };
        self.build(Rcode::NoError, data.map(|(kind, rdata)| (kind, ttl, rdata)))
    }

    fn build(&self, rcode: Rcode, answer: Option<(u16, u32, Vec<u8>)>) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.question.len() + 32);
        out.extend_from_slice(&self.header[..2]);
        out.extend_from_slice(&reply_flags(&self.header, rcode));
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&u16::from(answer.is_some()).to_be_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&self.question);
        if let Some((kind, ttl, rdata)) = answer {
            // A pointer to the question's name at offset 12.
            out.extend_from_slice(&[0xC0, 0x0C]);
            out.extend_from_slice(&kind.to_be_bytes());
            out.extend_from_slice(&CLASS_IN.to_be_bytes());
            out.extend_from_slice(&ttl.to_be_bytes());
            out.extend_from_slice(&u16::try_from(rdata.len()).unwrap_or(0).to_be_bytes());
            out.extend_from_slice(&rdata);
        }
        out
    }
}

/// A reply carrying only the header, for queries that could not be parsed.
pub(super) fn error_reply(header: &[u8; HEADER_LEN], rcode: Rcode) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN);
    out.extend_from_slice(&header[..2]);
    out.extend_from_slice(&reply_flags(header, rcode));
    out.extend_from_slice(&[0; 8]);
    out
}

/// QR and AA set, opcode and RD echoed, RA clear, the given rcode.
fn reply_flags(header: &[u8; HEADER_LEN], rcode: Rcode) -> [u8; 2] {
    let opcode_and_rd = header[2] & 0x79;
    [0x80 | 0x04 | opcode_and_rd, rcode as u8]
}

#[cfg(test)]
pub(crate) mod tests;
