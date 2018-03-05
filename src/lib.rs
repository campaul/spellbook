extern crate futures;
extern crate hyper;

mod router;
pub use router::Router;

use futures::future::Future;

use std::error::Error;

use std::rc::Rc;
use std::collections::HashMap;
use std::str::FromStr;

pub type Request = hyper::Request<hyper::Body>;
pub type Response = hyper::Response;
pub type Result = std::result::Result<hyper::Response, Box<Error>>;
pub type Next<'a, S> = &'a Fn(Context<S>) -> Result;
pub type Handler<S> = fn(Context<S>) -> Result;
pub type Tween<S> = fn(Context<S>, Next<S>) -> Result;

fn build_chain<S: Clone + 'static>(
    context: Context<S>,
    mut tweens: Vec<Tween<S>>,
    next: Box<Fn(Context<S>) -> Result>,
) -> Box<Fn(Context<S>) -> Result> {
    if tweens.len() == 0 {
        return next;
    }

    let tween = tweens.pop().unwrap();
    let chain = build_chain(context, tweens.clone(), next);
    return Box::new(move |ctx: Context<S>| tween(ctx, &*chain));
}

#[derive(Clone)]
pub struct Spellbook<S: Clone> {
    router: Router<S>,
    state: S,
}

impl<S: Clone + 'static> Spellbook<S> {
    pub fn new(state: S, router: Router<S>) -> Spellbook<S> {
        return Spellbook {
            router: router,
            state: state,
        };
    }

    pub fn serve(self, address: &'static str) {
        self.serve_until(address, futures::empty());
    }

    /// Execute the server until the given future, `shutdown_signal`, resolves.
    pub fn serve_until<F>(self, address: &'static str, shutdown_signal: F)
    where
        F: Future<Item = (), Error = ()>,
    {
        let addr = address.parse().unwrap();
        let server = hyper::server::Http::new()
            .bind(&addr, move || Ok(self.clone()))
            .unwrap();
        println!("Server running at {}", address);
        server.run_until(shutdown_signal).unwrap();
    }
}

impl<S: Clone + 'static> hyper::server::Service for Spellbook<S> {
    type Request = hyper::server::Request;
    type Response = hyper::server::Response;
    type Error = hyper::Error;

    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: hyper::server::Request) -> Self::Future {
        let res = router::handle(&self.router, self.state.clone(), Rc::new(req));

        let body = match res {
            Ok(body) => body,
            Err(e) => {
                let message = format!("{}", e);
                Response::new()
                    .with_header(hyper::header::ContentLength(message.len() as u64))
                    .with_status(hyper::StatusCode::InternalServerError)
                    .with_body(message)
            }
        };

        Box::new(futures::future::ok(body))
    }
}

pub struct Route {
    params: HashMap<String, String>,
}

impl Route {
    fn new(_pattern: &str, _uri: &hyper::Uri) -> Route {
        let mut params = HashMap::new();

        // TODO: this is dummy code
        params.insert(String::from("user_id"), String::from("42"));

        Route { params: params }
    }

    // TODO: Return a ValidationError instead of a str
    pub fn get<T: FromStr>(&self, key: &str) -> std::result::Result<T, &'static str> {
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
pub struct Context<S: Clone> {
    pub state: S,
    pub route: Rc<Route>,
    pub req: Rc<Request>,
}

impl<S: Clone> Context<S> {
    pub fn with(&self, state: S) -> Context<S> {
        Context {
            state: state,
            route: self.route.clone(),
            req: self.req.clone(),
        }
    }
}

pub mod prelude {
    pub use {Context, Next, Response, Result, Router, Spellbook};
}

// TODO: users shouldn't have to import hyper to build a response
