use bytes::BytesMut;
use frame::Command;
use frame::{Frame, Transmission};
use header::{Header, HeaderList};
use nom::IResult;
use tokio_io::codec::{Decoder, Encoder};

use nom::branch::alt;
use nom::combinator::{map, complete};
use nom::bytes::streaming::{tag, is_a};
use nom::multi::{many_till, many0, many1};
use nom::character::complete::{line_ending, anychar};

fn parse_server_command(i: &[u8]) -> IResult<&[u8], Command>
{
    alt((
       map(tag("CONNECTED"), |_| Command::Connected),
       map(tag("MESSAGE"), |_| Command::Message),
       map(tag("RECEIPT"), |_| Command::Receipt),
       map(tag("ERROR"), |_| Command::Error)
    ))(i)
}

/*
named!(parse_server_command(&[u8]) -> Command,
       alt!(
           map!(tag!("CONNECTED"), |_| Command::Connected) |
           map!(tag!("MESSAGE"), |_| Command::Message) |
           map!(tag!("RECEIPT"), |_| Command::Receipt) |
           map!(tag!("ERROR"), |_| Command::Error)
       )
);
*/

fn parse_header_character(i: &[u8]) -> IResult<&[u8], char>
{
    alt((
       complete(map(tag("\\n"), |_| '\n')),
       complete(map(tag("\\r"), |_| '\r')),
       complete(map(tag("\\c"), |_| ':')),
       complete(map(tag("\\\\"), |_| '\\')),
       anychar
    ))(i)
}

/*
named!(parse_header_character(&[u8]) -> char,
       alt!(
           complete!(map!(tag!("\\n"), |_| '\n')) |
           complete!(map!(tag!("\\r"), |_| '\r')) |
           complete!(map!(tag!("\\c"), |_| ':')) |
           complete!(map!(tag!("\\\\"), |_| '\\')) |
           anychar
       )
);
*/

fn parse_header(i: &[u8]) -> IResult<&[u8], Header>
{
    let (i, k) = many_till(parse_header_character, is_a(":\r\n"))(i)?;
    let (i, v) = many_till(parse_header_character, is_a("\r\n"))(i)?;
    let (i, _) = line_ending(i)?;
    Ok((i, Header::new_raw(k.0.into_iter().collect::<String>(), v.0.into_iter().collect::<String>())))
}

/*
named!(parse_header(&[u8]) -> Header,
       map!(
           do_parse!(
               k: flat_map!(is_not!(":\r\n"), many1!(parse_header_character)) >>
               tag!(":") >>
               v: flat_map!(is_not!("\r\n"), many1!(parse_header_character))>>
               line_ending >>
               (k, v)
           ),
           |(k, v)| {
               Header::new_raw(k.into_iter().collect::<String>(), v.into_iter().collect::<String>())
           }
       )
);
*/
fn get_body<'a, 'b>(bytes: &'a [u8], headers: &'b [Header]) -> ::nom::IResult<&'a [u8], &'a [u8]> {
    let mut content_length = None;
    for header in headers {
        if header.0 == "content-length" {
            trace!("found content-length header");
            match header.1.parse::<u32>() {
                Ok(value) => content_length = Some(value),
                Err(error) => warn!("failed to parse content-length header: {}", error),
            }
        }
    }
    if let Some(content_length) = content_length {
        trace!("using content-length header: {}", content_length);
        take!(bytes, content_length)
    } else {
        trace!("using many0 method to parse body");
        map!(bytes, many0!(is_not!("\0")), |body| {
            if body.len() == 0 {
                &[]
            } else {
                body.into_iter().nth(0).unwrap()
            }
        })
    }
}
fn parse_frame(i: &[u8]) -> IResult<&[u8], Frame>
{
    let (i, cmd) = parse_server_command(i)?;
    let (i, _) = line_ending(i)?;
    let (i, headers) = many0(parse_header)(i)?;
    let (i, _) = line_ending(i)?;
    let (i, body) = get_body(i, &headers)?;
    let (i, _) = tag("\0")(i)?;

    Ok((i, Frame {
        command: cmd,
        headers: HeaderList {headers},
        body: body.into()
    }))
}

named!(parse_frame2(&[u8]) -> Frame,
       map!(
           do_parse!(
               cmd: parse_server_command >>
               line_ending >>
               headers: many0!(parse_header) >>
               line_ending >>
               body: call!(get_body, &headers) >>
               tag!("\0") >>
               (cmd, headers, body)
           ),
           |(cmd, headers, body)| {
               Frame {
                   command: cmd,
                   headers: HeaderList { headers },
                   body: body.into()
               }
           }
       )
);
/*
*/

fn parse_transmission(i: &[u8]) -> IResult<&[u8], Transmission>
{
    alt((
        map(many1(line_ending), |_| Transmission::HeartBeat),
        map(parse_frame, |f| Transmission::CompleteFrame(f)),
    ))(i)
}

/*
named!(parse_transmission(&[u8]) -> Transmission,
       alt!(
           map!(many1!(line_ending), |_| Transmission::HeartBeat) |
           map!(parse_frame, |f| Transmission::CompleteFrame(f))
       )
);
*/
pub struct Codec;

impl Encoder for Codec {
    type Item = Transmission;
    type Error = ::std::io::Error;
    fn encode(
        &mut self,
        item: Transmission,
        buffer: &mut BytesMut,
    ) -> Result<(), ::std::io::Error> {
        item.write(buffer);
        Ok(())
    }
}
impl Decoder for Codec {
    type Item = Transmission;
    type Error = ::std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Transmission>, ::std::io::Error> {
        use std::io::{Error, ErrorKind};

        trace!("decoding data: {:?}", src);
        let (point, data) = match parse_transmission(src) {
            Ok((rest, data)) => (rest.len(), data),
            Err(nom::Err::Incomplete(_)) => return Ok(None),
            Err(e) => {
                warn!("parse error: {:?}", e);
                return Err(Error::new(ErrorKind::Other, format!("parse error: {:?}", e)));
            }
        };
        let len = src.len().saturating_sub(point);
        src.split_to(len);
        Ok(Some(data))
    }
}
