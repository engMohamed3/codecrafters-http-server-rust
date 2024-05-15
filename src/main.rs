mod http_server;

use std::env;

use crate::http_server::Application;

fn main() {
    println!("Logs from your program will appear here!");

    let args = env::args().collect::<Vec<String>>();
    println!("{:?}", args);

    let mut app = Application::new(4221);

    app.get("/", |_request, mut response| {
        response.code(200).send();
    });

    app.get("/echo/:str", |request, mut response| {
        response.send_text(&request.params["str"]);
    });

    app.listen(|app| {
        println!("Server run on port: {}", app.port);
    });
}
