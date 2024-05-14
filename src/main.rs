use std::borrow::Cow;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_connection(&stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: &TcpStream) {
    let mut buf = [0; 1024];
    stream.read(&mut buf).unwrap();

    let request = String::from_utf8_lossy(&buf[..]);
    let (method, path) = parse_request_line(request);

    match method.as_str() {
        "GET" => match path.as_str() {
            "/" => {
                write_response(&stream, 200);
            }
            _ => {
                write_response(&stream, 404);
            }
        },
        _ => {
            stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
            stream.shutdown(Shutdown::Both).unwrap()
        }
    }
}

fn parse_request_line(request: Cow<str>) -> (String, String) {
    let request_line = request
        .lines()
        .next()
        .unwrap()
        .split(" ")
        .collect::<Vec<_>>();

    (request_line[0].to_string(), request_line[1].to_string())
}

fn write_response(mut stream: &TcpStream, code: u16) {
    match code {
        200 => {
            stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
            stream.shutdown(Shutdown::Both).unwrap()
        }
        404 => {
            stream
                .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                .unwrap();
            stream.shutdown(Shutdown::Both).unwrap()
        }
        500 | _ => {
            stream
                .write("HTTP/1.1 500 Internal Server Error\r\n\r\n".as_bytes())
                .unwrap();
            stream.shutdown(Shutdown::Both).unwrap()
        }
    }
}
