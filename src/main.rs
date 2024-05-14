use std::io::Write;
use std::net::{Shutdown, TcpListener, TcpStream};

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream);
                println!("accepted new connection");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
    stream.shutdown(Shutdown::Both).unwrap()
}
