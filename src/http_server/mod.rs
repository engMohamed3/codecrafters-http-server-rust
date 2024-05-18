mod thread_pool;

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::fs;

use self::thread_pool::ThreadPool;

#[derive(Debug, Clone)]
struct Handler {
    pub handler: fn(Request, Response),
    pub method: String,
    regex: regex::Regex,
}

#[derive(Debug, Clone)]
pub struct Application {
    pub port: u16,
    pub body_limit: usize,
    handlers: Vec<Handler>,
    static_dir: Option<String>,
}

impl Application {
    pub fn new(port: u16) -> Application {
        Application {
            port,
            handlers: vec![],
            static_dir: None,
            body_limit: 100 * 1024,
        }
    }

    pub fn listen(self, handler: fn(&Application)) {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port)).unwrap();
        let pool = ThreadPool::new(4);
        handler(&self);
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let self_clone = self.clone();
                    pool.execute(move || self_clone.route(stream));
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }

    pub fn get(&mut self, path: &str, handler: fn(Request, Response)) {
        self.handlers.push(Handler {
            handler,
            method: "GET".to_string(),
            regex: Application::parse_path(path),
        });
    }

    pub fn post(&mut self, path: &str, handler: fn(Request, Response)) {
        self.handlers.push(Handler {
            handler,
            method: "POST".to_string(),
            regex: Application::parse_path(path),
        });
    }

    pub fn static_files(&mut self, path: &str, dir: &str) {
        if self.static_dir.is_some() {
            eprintln!("Only one static dir is allowed");
            return;
        }
        let paths = fs::read_dir(dir);
        match paths {
            Ok(paths) => {
                self.static_dir = Some(dir.to_string());
                for p in paths {
                    let p = p.unwrap().file_name().to_str().unwrap().to_string();
                    self.get(
                        format!("{}/{}", path, p).as_str(),
                        Application::handle_static,
                    );
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    fn route(&self, mut stream: TcpStream) {
        let mut buf = vec![0; self.body_limit];
        let read_buf = stream.read(&mut buf).unwrap();

        let mut request = Application::parse_request(&String::from_utf8_lossy(&buf[..read_buf]));
        request.static_dir = self.static_dir.clone();
        let mut response = Response::new(200, stream, request.clone());

        match request.method.as_str() {
            "GET" => {
                let gets = self
                    .handlers
                    .iter()
                    .enumerate()
                    .filter(|(_, h)| h.method == "GET".to_string())
                    .map(|(_, h)| h)
                    .collect::<Vec<_>>();

                Application::router(gets, request, response);
            }
            "POST" => {
                let posts = self
                    .handlers
                    .iter()
                    .enumerate()
                    .filter(|(_, h)| h.method == "POST".to_string())
                    .map(|(_, h)| h)
                    .collect::<Vec<_>>();

                Application::router(posts, request, response);
            }
            _ => {
                response.code(404).send();
            }
        }
    }

    fn parse_request(buf: &Cow<str>) -> Request {
        let request_line = buf.lines().next().unwrap().split(" ").collect::<Vec<_>>();
        if request_line.len() < 3 {
            panic!("Invalid Request Line");
        }
        let mut headers = HashMap::new();
        for line in buf.lines().skip(1) {
            if line.is_empty() {
                break;
            }
            match line.split(": ").collect::<Vec<&str>>().as_slice() {
                [s1, s2] => {
                    headers.insert(s1.to_string().to_lowercase(), s2.to_string());
                }
                _ => {
                    println!("Invalid Value for archive. line : `{}`", line);
                }
            }
        }

        let method = request_line[0].to_string();
        let path = request_line[1].to_string();
        let protocol = request_line[2].to_string();
        let body = Some(buf.lines().last().unwrap().as_bytes().to_vec());

        Request::new(method, path, protocol, headers, body)
    }

    fn router(handlers: Vec<&Handler>, mut request: Request, mut response: Response) {
        for hd in handlers {
            if hd.regex.is_match(&request.path) {
                let captures = hd.regex.captures(&request.path).unwrap();
                request.params = hd
                    .regex
                    .capture_names()
                    .flatten()
                    .filter_map(|x| Some((x.to_string(), captures.name(x)?.as_str().to_string())))
                    .collect::<HashMap<String, String>>();
                (hd.handler)(request, response);
                return;
            }
        }
        response.code(404).send();
    }

    fn parse_path(path: &str) -> regex::Regex {
        let mut regex_str = "^".to_string();
        if path == "/" {
            if path.starts_with("/:") {
                regex_str.push_str(&format!(r"/(?P<{}>.+)", path.get(2..).unwrap()))
            } else {
                regex_str.push_str("/")
            }
        } else {
            for str in path.split("/") {
                if !str.is_empty() {
                    if str.starts_with(":") {
                        regex_str.push_str(&format!(r"/(?P<{}>.+)", str.get(1..).unwrap()))
                    } else {
                        regex_str.push_str(&format!("/{}", str))
                    }
                }
            }
        }
        regex_str.push_str("$");
        regex::Regex::new(&regex_str).unwrap()
    }

    fn handle_static(req: Request, mut res: Response) {
        let path = format!(
            "{}/{}",
            req.static_dir.unwrap(),
            req.path.split("/").last().unwrap()
        );
        let mut file = fs::File::open(path).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        res.code(200).send_binary(&buf[..]);
    }
}

#[derive(Debug, Default, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub protocol: String,
    headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub static_dir: Option<String>,
}

impl Request {
    pub fn new(
        method: String,
        path: String,
        protocol: String,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Request {
        Request {
            method,
            path,
            protocol,
            headers,
            params: HashMap::new(),
            body,
            static_dir: None,
        }
    }

    pub fn get_header(&self, key: &str) -> Option<String> {
        self.headers.get(&key.to_lowercase()).map(|x| x.to_string())
    }
    pub fn get_header_valus(&self, key: &str) -> Option<Vec<String>> {
        self.headers
            .get(&key.to_lowercase())
            .map(|x| x.to_string())
            .map(|x| {
                x.split(",")
                    .map(|x| x.trim().to_string())
                    .collect::<Vec<String>>()
            })
    }
}

#[derive(Debug)]
pub struct Response {
    code: u16,
    stream: TcpStream,
    headers: Vec<(String, String)>,
    request: Request,
}

impl Response {
    pub fn new(code: u16, stream: TcpStream, request: Request) -> Response {
        Response {
            code,
            stream,
            headers: vec![],
            request,
        }
    }

    pub fn code(&mut self, code: u16) -> &mut Self {
        self.code = code;
        self
    }

    pub fn header(&mut self, header: (String, String)) -> &mut Self {
        self.headers.push(header);
        self
    }

    pub fn send_text(&mut self, data: &str) {
        self.write_res_code();
        self.set_encoding();
        self.header(("Content-Type".to_string(), "text/plain".to_string()))
            .header(("Content-Length".to_string(), data.len().to_string()));

        for (x, y) in self.headers.iter() {
            self.stream
                .write(format!("{}: {}\r\n", x, y).as_bytes())
                .unwrap();
        }
        self.stream.write("\r\n".as_bytes()).unwrap();
        self.stream.write(data.as_bytes()).unwrap();
        self.stream.shutdown(Shutdown::Both).unwrap();
    }

    pub fn send(&mut self) {
        self.write_res_code();
        self.set_encoding();
        for (x, y) in self.headers.iter() {
            self.stream
                .write(format!("{}: {}\r\n", x, y).as_bytes())
                .unwrap();
        }
        self.stream.write("\r\n".as_bytes()).unwrap();
        self.stream.shutdown(Shutdown::Both).unwrap();
    }

    pub fn send_binary(&mut self, data: &[u8]) {
        self.write_res_code();
        self.set_encoding();
        self.header((
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        ))
        .header(("Content-Length".to_string(), data.len().to_string()));
        for (x, y) in self.headers.iter() {
            self.stream
                .write(format!("{}: {}\r\n", x, y).as_bytes())
                .unwrap();
        }
        self.stream.write("\r\n".as_bytes()).unwrap();
        self.stream.write(data).unwrap();
        self.stream.shutdown(Shutdown::Both).unwrap();
    }

    fn write_res_code(&mut self) {
        match self.code {
            200 => {
                self.stream.write("HTTP/1.1 200 OK\r\n".as_bytes()).unwrap();
            }
            201 => {
                self.stream
                    .write("HTTP/1.1 201 Created\r\n".as_bytes())
                    .unwrap();
            }
            404 => {
                self.stream
                    .write("HTTP/1.1 404 Not Found\r\n".as_bytes())
                    .unwrap();
            }
            500 | _ => {
                self.stream
                    .write("HTTP/1.1 500 Internal Server Error\r\n".as_bytes())
                    .unwrap();
            }
        }
    }

    fn set_encoding(&mut self) {
        if let Some(values) = self.request.get_header_valus("Accept-Encoding") {
            for value in values {
                if ["gzip", "br"].contains(&value.as_str()) {
                    self.header(("Content-Encoding".to_string(), value));
                    break;
                }
            }
        }
    }
}
