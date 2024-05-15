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
    let (method, path) = parse_request_line(&request);

    match method.as_str() {
        "GET" => get_path(&stream, &request, path.as_str()),
        _ => {
            response(&stream, 500);
        }
    }
}

fn get_path(stream: &TcpStream, request: &Cow<str>, path: &str) {
    match path {
        "/" => {
            response(&stream, 200);
        }
        p if p.starts_with("/echo") => {
            let data = p.split("/").collect::<Vec<_>>()[2];
            response_with_data(&stream, 200, data);
        }
        "/user-agent" => {
            let user_agent = get_header(&request, "User-Agent");
            response_with_data(&stream, 200, user_agent.unwrap().as_str());
        }
        _ => {
            response(&stream, 404);
        }
    }
}

fn parse_request_line(request: &Cow<str>) -> (String, String) {
    let request_line = request
        .lines()
        .next()
        .unwrap()
        .split(" ")
        .collect::<Vec<_>>();

    (request_line[0].to_string(), request_line[1].to_string())
}

fn get_header(request: &Cow<str>, header: &str) -> Option<String> {
    request
        .lines()
        .find(|line| line.starts_with(header))
        .map(|line| line.split(": ").collect::<Vec<_>>()[1].to_string())
}

fn write_res_code(mut stream: &TcpStream, code: u16) {
    match code {
        200 => {
            stream.write("HTTP/1.1 200 OK\r\n".as_bytes()).unwrap();
        }
        404 => {
            stream
                .write("HTTP/1.1 404 Not Found\r\n".as_bytes())
                .unwrap();
        }
        500 | _ => {
            stream
                .write("HTTP/1.1 500 Internal Server Error\r\n".as_bytes())
                .unwrap();
        }
    }
}

fn response_with_data(mut stream: &TcpStream, code: u16, data: &str) {
    write_res_code(stream, code);
    stream
        .write("Content-Type: text/plain\r\n".as_bytes())
        .unwrap();
    stream
        .write(format!("Content-Length: {}\r\n", data.len()).as_bytes())
        .unwrap();
    stream.write("\r\n".as_bytes()).unwrap();
    stream.write(data.as_bytes()).unwrap();
    stream.shutdown(Shutdown::Both).unwrap();
}

fn response(mut stream: &TcpStream, code: u16) {
    write_res_code(stream, code);
    stream.write("\r\n".as_bytes()).unwrap();
    stream.shutdown(Shutdown::Both).unwrap();
}
