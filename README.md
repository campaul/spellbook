```rust
extern crate hyper;
extern crate spellbook;

use spellbook::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
struct MyApp {
    title: &'static str,
}

fn user_handler(context: Rc<Context<MyApp>>) -> Result {
    let body = format!("<h1>Welcome to {}</h1>", context.app.title);

    Ok(Response::new()
        .with_header(hyper::header::ContentLength(body.len() as u64))
        .with_body(body))
}

fn main() {
    let app = MyApp {
        title: "My App",
    };

    let router = Router::new()
        .get("/", user_handler);

    Spellbook::new(app, router).serve("127.0.0.1:3000");
}
```
