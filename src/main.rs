mod http_server;

use std::{env, fs, io::Write};

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

    app.post("/echo", |request, mut response| {
        let str = request
            .body
            .unwrap()
            .iter()
            .map(|x| *x as char)
            .collect::<String>();

        response.send_text(&str);
    });

    app.post("/files/:fileName", |request, mut response| {
        let args = env::args().collect::<Vec<String>>();
        if args.len() > 1 && args[1] == "--directory" {
            let dir = &args[2];
            let mut file =
                fs::File::create(format!("{}/{}", dir, request.params["fileName"])).unwrap();
            file.write_all(&request.body.unwrap()).unwrap();
            response.code(201).send();
        } else {
            response.code(400).send_text("No directory provided");
        }
    });

    app.get("/echo/:str", |request, mut response| {
        response.send_text(&request.params["str"]);
    });

    app.get("/user-agent", |req, mut res| {
        res.send_text(&req.get_header("User-Agent").unwrap());
    });

    app.listen(|app| {
        println!("Server run on port: {}", app.port);
    });
}
