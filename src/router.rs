use Context;
use Handler;
use Request;
use Result;
use Response;
use Middleware;

use std;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;

pub struct Path {
    params: HashMap<String, String>,
}

impl Path {
    fn new(pattern: &String, path: &str) -> Option<Path> {
        let match_segments: Vec<&str> = pattern.split("/").collect();
        let path_segments: Vec<&str> = path.split("/").collect();

        if match_segments.len() != path_segments.len() {
            return None;
        }

        let mut params = HashMap::new();
        let mut matches = true;
        for i in 0..match_segments.len() {
            let match_segment = match_segments[i];
            let path_segment = path_segments[i];

            if match_segment.starts_with(":") {
                let mut match_string = String::from(match_segment);
                match_string.remove(0);
                let path_string = String::from(path_segment);
                params.insert(match_string, path_string);
                continue;
            } else if match_segment == path_segment {
                continue;
            } else {
                matches = false;
            }
        }

        if matches {
            return Some(Path {
                params: params
            });
        } else {
            return None
        }
    }

    pub fn get<T: FromStr>(&self, key: &str) -> std::result::Result<T, &'static str> {
        // TODO: Return a ValidationError instead of a str
        match self.params.get(key) {
            Some(s) => match s.parse() {
                Ok(v) => Ok(v),
                Err(_) => Err("value wrong type"),
            },
            None => Err("value does not exist"),
        }
    }
}

#[derive(Clone)]
struct Route<S: Clone> {
    pattern: String,
    handler: Handler<S>,
}

#[derive(Clone)]
pub struct Router<S: Clone> {
    routes: Vec<Route<S>>,
    middlewares: Vec<Middleware<S>>,
}

impl<S: Clone + 'static> Router<S> {
    pub fn new() -> Router<S> {
        Router {
            routes: vec![],
            middlewares: vec![],
        }
    }

    pub fn get(mut self, pattern: &str, handler: Handler<S>) -> Router<S> {
        self.routes.push(Route {
            pattern: String::from(pattern),
            handler: handler,
        });
        self
    }

    pub fn with(mut self, middleware: Middleware<S>) -> Router<S> {
        self.middlewares.insert(0, middleware);
        self
    }

    pub(crate) fn handle(&self, state: S, req: Rc<Request>) -> Result {
        for route in self.routes.iter() {
            if let Some(path) = Path::new(&route.pattern, req.path()) {
                let context = Context {
                    state: state,
                    path: Rc::new(path),
                    req: req.clone(),
                };
                let chain = build_chain(self.middlewares.clone(), Box::new(route.handler.clone()));
                return chain(context);
            }
        }

        let context = Context {
            state: state,
            path: Rc::new(Path {
                params: HashMap::new(),
            }),
            req: req.clone(),
        };

        let chain = build_chain(vec![], Box::new(not_found));
        chain(context)
    }
}

fn build_chain<S: Clone + 'static>(
    mut middlewares: Vec<Middleware<S>>,
    next: Box<Fn(Context<S>) -> Result>,
) -> Box<Fn(Context<S>) -> Result> {
    if middlewares.len() == 0 {
        return next;
    }

    let middleware = middlewares.pop().unwrap();
    let chain = build_chain(middlewares.clone(), next);
    return Box::new(move |ctx: Context<S>| middleware(ctx, &*chain));
}

fn not_found<S: Clone>(_: Context<S>) -> Result {
    // TODO: set status code
    Ok(Response::new().with_body("404 not found"))
}
