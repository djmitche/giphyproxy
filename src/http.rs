use anyhow::{anyhow, Error, Result};
use nom::{
    bytes::streaming::{tag, take_while, take_while1},
    character::{is_alphanumeric, is_digit},
    combinator::{map_res, value},
    multi::many0,
    sequence::tuple,
};
use nom::{Err, IResult};

#[derive(Debug)]
pub enum ParseHeadResult {
    /// Successful parse
    Connect { host: String, port: u16 },

    /// Unrecoverable error
    Err(Error),

    /// Valid so far, but incomplete
    Incomplete,
}

use ParseHeadResult::*;

impl PartialEq for ParseHeadResult {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Incomplete, Incomplete) => true,
            (Connect { host: h1, port: p1 }, Connect { host: h2, port: p2 })
                if h1 == h2 && p1 == p2 =>
            {
                true
            }
            // note that errors always compare inequal (anyhow::Error does not support PartialEq)
            _ => false,
        }
    }
}

/// Parse an HTTP request head.
///
/// This is *severely* limited to accept HTTP/1.1 CONNECT requests, allowing but ignoring simple
/// headers, and nothing else.  Depending on requirements, this could easily be expanded to be more
/// permissive.
pub fn parse_head(input: &[u8]) -> ParseHeadResult {
    match parse_connect(input) {
        IResult::Ok((remaining, output)) if remaining.len() == 0 => Connect {
            host: output.0.to_owned(),
            port: output.1,
        },
        IResult::Ok(_) => Err(anyhow!("extra bytes in head")),
        IResult::Err(Err::Incomplete(_)) => Incomplete,
        IResult::Err(Err::Failure(e)) => Err(anyhow!(
            "bad request: {:?} (input: {})",
            e,
            String::from_utf8_lossy(e.input)
        )),
        IResult::Err(Err::Error(e)) => Err(anyhow!(
            "bad request: {:?} (input: {})",
            e,
            String::from_utf8_lossy(e.input)
        )),
    }
}

/// Recognize a full CONNECT request head (see notes for `parse_head`)
fn parse_connect(input: &[u8]) -> IResult<&[u8], (&str, u16)> {
    fn to_tuple<'h>(input: (&[u8], (&'h str, u16), &[u8], (), (), ())) -> Result<(&'h str, u16)> {
        Ok(input.1)
    }
    map_res(
        tuple((
            tag(b"CONNECT "),
            hostport,
            tag(b" HTTP/1.1"),
            rn,
            headers,
            rn,
        )),
        to_tuple,
    )(input)
}

/// Recognize a hostname:port pair.  This is rather conservative, since for this use the only valid
/// value is `api.giphy.com:443`
fn hostport(input: &[u8]) -> IResult<&[u8], (&str, u16)> {
    fn to_tuple<'h>(input: (&'h str, &[u8], u16)) -> Result<(&'h str, u16)> {
        Ok((input.0, input.2))
    }
    map_res(tuple((hostname, tag(":"), port)), to_tuple)(input)
}

/// Parse a hostname as part of a CONNECT request
fn hostname(input: &[u8]) -> IResult<&[u8], &str> {
    fn to_str(input: &[u8]) -> Result<&str> {
        Ok(std::str::from_utf8(input)?)
    }
    fn hostname_char(c: u8) -> bool {
        is_alphanumeric(c) || c == b'.' || c == b'-'
    }
    map_res(take_while(hostname_char), to_str)(input)
}

/// Parse a port number into a u16
fn port(input: &[u8]) -> IResult<&[u8], u16> {
    fn to_u16<'h>(input: &[u8]) -> Result<u16> {
        // note: unwrap is safe since we've confirmed input is just ascii digits
        Ok(std::str::from_utf8(input).unwrap().parse()?)
    }
    map_res(take_while(is_digit), to_u16)(input)
}

/// Parse and ignore zero or more headers
fn headers(input: &[u8]) -> IResult<&[u8], ()> {
    value((), many0(header))(input)
}

/// Parse and ignore a header.  This does not parse the full generality of headers!
fn header(input: &[u8]) -> IResult<&[u8], ()> {
    fn not_newline(c: u8) -> bool {
        c != b'\r' && c != b'\n'
    }
    value((), tuple((take_while1(not_newline), rn)))(input)
}

/// Recognize a \r\n sequence
fn rn(input: &[u8]) -> IResult<&[u8], ()> {
    value((), tag(b"\r\n"))(input)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty() {
        assert_eq!(parse_head(b""), Incomplete);
    }

    #[test]
    fn test_prefix() {
        assert_eq!(parse_head(b"CONNECT foo."), Incomplete);
    }

    #[test]
    fn test_bad_prefix() {
        assert!(matches!(parse_head(b"GET"), Err(_)));
    }

    #[test]
    fn test_bad_port_too_large() {
        assert!(matches!(
            parse_head(b"CONNECT foo.com:9999999 HTTP/1.1\r\n\r\n"),
            Err(_)
        ));
    }

    #[test]
    fn test_bad_invalid_hostname() {
        assert!(matches!(
            parse_head(b"CONNECT fo/o.c/om:10 HTTP/1.1\r\n\r\n"),
            Err(_)
        ));
    }

    #[test]
    fn test_good_no_headers() {
        assert_eq!(
            parse_head(b"CONNECT foo.com:1234 HTTP/1.1\r\n\r\n"),
            Connect {
                host: "foo.com".to_owned(),
                port: 1234u16
            }
        );
    }

    #[test]
    fn test_good_headers() {
        assert_eq!(
            parse_head(b"CONNECT foo.com:1234 HTTP/1.1\r\nProxy-Connection: Keep-Alive\r\n\r\n"),
            Connect {
                host: "foo.com".to_owned(),
                port: 1234u16
            }
        );
    }

    #[test]
    fn test_extra_chars() {
        assert!(matches!(
            parse_head(b"CONNECT foo.com:1234 HTTP/1.1\r\n\r\nUHOH"),
            Err(_)
        ));
    }
}
