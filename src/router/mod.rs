mod tree;

use crate::Context;
use crate::Handler;
use crate::Request;
use crate::Result;
use crate::Route;
use crate::Tween;

use std::rc::Rc;

use hyper::Response;
use hyper::StatusCode;

#[derive(Clone)]
pub struct Router<S: Clone> {
    handlers: tree::Tree<S>,
    tweens: Vec<Tween<S>>,
}

impl<S: Clone + 'static> Router<S> {
    pub fn new() -> Router<S> {
        Router {
            handlers: tree::Tree::new(),
            tweens: vec![],
        }
    }

    // TODO: add more methods
    pub fn get(self, pattern: &str, handler: Handler<S>) -> Router<S> {
        self.register("GET", pattern, handler)
    }

    pub fn register(mut self, method: &str, pattern: &str, handler: Handler<S>) -> Router<S> {
        let trimmed = format!("{}/{}", method, trim_path(pattern));
        let segments = trimmed.split("/");
        let mut current = 0;

        for segment in segments {
            if segment.starts_with(":") || segment.starts_with("*") {
                current = self
                    .handlers
                    .node_set_wildcard(current, String::from(segment));
            } else {
                current = self.handlers.node_add_child(current, String::from(segment));
            }
        }

        self.handlers.node_set_handler(current, handler);

        self
    }

    pub fn with(mut self, tween: Tween<S>) -> Router<S> {
        self.tweens.insert(0, tween);
        self
    }

    pub(crate) fn handle(&self, state: S, req: Rc<Request>) -> Result {
        let trimmed = format!("{}/{}", req.method(), trim_path(req.uri().path()));
        let segments = trimmed.split("/");
        let mut current = 0;
        let mut route = Route::new();

        for segment in segments {
            match self.handlers.node_get_child(current, String::from(segment)) {
                Some(child) => {
                    current = *child;
                }
                None => match self.handlers.node_get_wildcard(current) {
                    Some(wildcard) => {
                        current = wildcard.1;
                        if wildcard.0.starts_with(":") {
                            let mut wildcard_string = String::from(wildcard.0);
                            wildcard_string.remove(0);
                            route.params.insert(wildcard_string, String::from(segment));
                        } else {
                            break;
                        }
                    }
                    None => {
                        current = 0;
                        break;
                    }
                },
            }
        }

        if let Some(handler) = self.handlers.node_get_handler(current) {
            let context = Context {
                state: state,
                route: Rc::new(route),
                req: Some(req.clone()),
            };
            let chain = build_chain(self.tweens.clone(), Box::new(handler));
            return chain(context);
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(String::from("404"))?)
    }
}

fn build_chain<S: Clone + 'static>(
    mut tweens: Vec<Tween<S>>,
    next: Box<dyn Fn(Context<S>) -> Result>,
) -> Box<dyn Fn(Context<S>) -> Result> {
    if tweens.len() == 0 {
        return next;
    }

    let tween = tweens.pop().unwrap();
    let chain = build_chain(tweens.clone(), next);
    return Box::new(move |ctx: Context<S>| tween(ctx, &*chain));
}

fn trim_path(pattern: &str) -> String {
    let mut pattern_string = String::from(pattern);

    // TODO: should it be an error is there is no leading slash?
    if pattern_string.starts_with("/") {
        pattern_string.remove(0);
    }

    if pattern_string.ends_with("/") {
        pattern_string.pop();
    }

    pattern_string
}
