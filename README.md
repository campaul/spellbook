# spellbook

`spellbook` is an experimental web framework written in Rust. It is not in a working state yet.

## Usage

```rust
extern crate spellbook;

use spellbook::{Context, Response, Router, Server};

#[derive(Clone)]
struct State {
    title: &'static str,
}

fn user_handler(context: Context<State>) -> spellbook::Result {
    let body = format!("<h1>Welcome to {}</h1>", context.state.title);
    Ok(Response::new().with_body(body))
}

fn main() {
    let state = State {
        title: "My App",
    };

    let router = Router::new().get("/", user_handler);

    Server::new(state, router).serve("127.0.0.1:3000");
}
```
