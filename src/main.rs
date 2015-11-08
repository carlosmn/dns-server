use std::borrow::Cow;
use std::io::{self, Write};
use std::net::UdpSocket;

#[derive(Debug)]
enum QR {
    Query,
    Response,
}

#[derive(Debug)]
enum Opcode {
    Query,
    IQuery,
    Status,
    Reserved,
    Notify,
    Update,
    Invalid,
}

#[derive(Debug)]
struct Header {
    id: u16,
    qr: QR,
    opcode: Opcode,
    aa: bool,
    tc: bool,
    rd: bool,
    // a few missing
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
}

impl Header {
    fn parse(buf: &[u8]) -> Header {
        Header {
            id: parse_u16(buf),
            qr: parse_qr(buf[2]),
            opcode: parse_opcode(buf[2]),
            aa: parse_authoritative(buf[2]),
            tc: parse_truncated(buf[2]),
            rd: parse_recursion(buf[2]),
            qdcount: parse_u16(&buf[4..]),
            ancount: parse_u16(&buf[6..]),
            nscount: parse_u16(&buf[8..]),
            arcount: parse_u16(&buf[10..]),
        }
    }
}

fn parse_u16(buf: &[u8]) -> u16 {
    let higher = buf[0] as u16;
    let lower = buf[1] as u16;

    ((higher << 8) | lower)
}

fn parse_qr(n: u8) -> QR {
    if n & 0b10000000 == 0 { QR::Query } else { QR::Response }
}

fn parse_opcode(n: u8) -> Opcode {
    match n & 0b01111000 {
        0 => Opcode::Query,
        1 => Opcode::IQuery,
        2 => Opcode::Status,
        3 => Opcode::Reserved,
        4 => Opcode::Notify,
        5 => Opcode::Update,
        _ => Opcode::Invalid,
    }
}

fn parse_authoritative(n: u8) -> bool {
    n & 0b00000100 == 1
}

fn parse_truncated(n: u8) -> bool {
    n & 0b00000010 == 1
}

fn parse_recursion(n: u8) -> bool {
    n & 0b00000001 == 1
}

#[derive(Debug)]
struct Question<'a> {
    qname: Vec<Cow<'a, str>>,
    qtype: u16,
    qclass: u16,
    // Length of the record in the buffer
    len: usize,
}

fn parse_question_part(buf: &[u8]) -> (usize, Option<Cow<str>>) {
    let len = buf[0] as usize;

    if len == 0 {
        (0, None)
    } else {
        (len, Some(String::from_utf8_lossy(&buf[1..len+1])))
    }
}

fn parse_question(buf: &[u8]) -> Question {
    let mut v = Vec::new();
    let mut off: usize = 0;

    loop {
        let (n, maybe_s) = parse_question_part(&buf[off..]);
        if let Some(s) = maybe_s {
            off += n + 1;
            v.push(s);
        } else {
            break;
        }
    }

    Question {
        qname: v,
        qtype: parse_u16(&buf[off+1..]),
        qclass: parse_u16(&buf[off+3..]),
        len: off + 4,
    }
}

fn main() {
    let socket = match UdpSocket::bind("127.0.0.1:1234") {
        Ok(s) => s,
        Err(e) => {
            io::stderr().write(format!("failed to create socket: {}", e).as_bytes());
            return;
        }
    };

    loop {
        let mut buf = [0; 1024];
        let (amt, src) = match socket.recv_from(&mut buf) {
            Ok((a, s)) => (a, s),
            Err(e) => {
                io::stderr().write(b"failed to read from socket");
                continue;
            }
        };

        println!("Got a packet of size {}", amt);

        let header = Header::parse(&buf);
        println!("header {:?}", header);

        let (_, s) = parse_question_part(&buf[12..]);
        println!("found q {}", s.unwrap());

        let s = parse_question(&buf[12..]);
        println!("found q {:?}", s);

        // Send a reply to the socket we received data from
        //let buf = &mut buf[..amt];
        //buf.reverse();
        //try!(socket.send_to(buf, &src));
    }
}