mod http_server;

use std::env;

use crate::http_server::Application;

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let mut app = Application::new(4221);

    if args.len() > 1 {
        if args[1] == "--directory" {
            app.static_files("/files", &args[2]);
        }
    }

    app.get("/", |_request, mut response| {
        response.code(200).send();
    });

    app.get("/echo/:str", |request, mut response| {
        response.send_text(&request.params["str"]);
    });

    app.get("/user-agent", |req, mut res| {
        res.send_text(&req.headers["User-Agent"]);
    });

    app.listen(|app| {
        println!("Server run on port: {}", app.port);
    });
}
