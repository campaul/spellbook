```
extern crate hyper;
extern crate spellbook;

use std::rc::Rc;

use spellbook::{
    Request,
    Response,
    Result,
    Router,
    Spellbook
};

#[derive(Clone)]
struct MyApp {
    title: String,
}

fn index(app: Rc<MyApp>, _req: Rc<Request>) -> Result {
    let body = format!("<h1>{}</h1>", app.title);

    Ok(Response::new()
        .with_header(hyper::header::ContentLength(body.len() as u64))
        .with_body(body))
}

fn main() {
    let app = MyApp {
        title: String::from("My App"),
    };

    let router = Router::new()
        .get("/", index);

    Spellbook::new(app, router).serve("127.0.0.1:3000");
}
```
