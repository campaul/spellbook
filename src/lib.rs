extern crate futures;
extern crate hyper;

use futures::future::Future;

use std::error::Error;

use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;

pub type Request = hyper::Request<hyper::Body>;
pub type Response = hyper::Response;
pub type Result = std::result::Result<hyper::Response, Box<Error>>;
pub type Next<'a, A> = &'a Fn(Rc<Context<A>>) -> Result;
pub type Handler<A> = fn(Rc<Context<A>>) -> Result;
pub type Tween<A> = fn(Rc<Context<A>>, Next<A>) -> Result;

#[derive(Clone)]
pub struct Router<A: Clone> {
    handlers: Vec<Handler<A>>,
    tweens: Vec<Tween<A>>,
}

impl<A: Clone + 'static> Router<A> {
    pub fn new() -> Router<A> {
        Router {
            handlers: vec![],
            tweens: vec![],
        }
    }

    pub fn get(mut self, _path: &str, handler: Handler<A>) -> Router<A> {
        self.handlers.push(handler);
        self
    }

    pub fn with(mut self, tween: Tween<A>) -> Router<A> {
        self.tweens.insert(0, tween);
        self
    }

    fn handle(&self, app: Rc<A>, req: Rc<Request>) -> Result {
        // TODO: this is dummy code
        let handler = self.handlers[0].clone();
        let route = Rc::new(Route::new("/users/:user_id", req.uri()));
        let context = Rc::new(Context {
            app: app,
            route: route,
            req: req.clone(),
        });

        let next = Box::new(move |ctx: Rc<Context<A>>| handler(ctx));
        let chain = build_chain(context.clone(), self.tweens.clone(), next);
        chain(context)
    }
}

// TODO: clone tweens before mutating
fn build_chain<A: Clone + 'static>(
    context: Rc<Context<A>>,
    mut tweens: Vec<Tween<A>>,
    next: Box<Fn(Rc<Context<A>>) -> Result>
) -> Box<Fn(Rc<Context<A>>) -> Result> {
    if tweens.len() == 0 {
        return next;
    }

    let tween = tweens.pop().unwrap();
    let chain = build_chain(context, tweens.clone(), next);
    return Box::new(move |ctx: Rc<Context<A>>| tween(ctx, &*chain))
}

#[derive(Clone)]
pub struct Spellbook<A: Clone> {
    router: Router<A>,
    app: Rc<A>,
}

impl<A: Clone + 'static> Spellbook<A> {
    pub fn new(app: A, router: Router<A>) -> Spellbook<A> {
        return Spellbook {
            router: router,
            app: Rc::new(app),
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

impl<A: Clone + 'static> hyper::server::Service for Spellbook<A> {
    type Request = hyper::server::Request;
    type Response = hyper::server::Response;
    type Error = hyper::Error;

    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: hyper::server::Request) -> Self::Future {
        let res = self.router.handle(self.app.clone(), Rc::new(req));

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

pub struct Context<A: Clone> {
    pub app: Rc<A>,
    pub route: Rc<Route>,
    pub req: Rc<Request>,
}

pub mod prelude {
    pub use {Context, Next, Response, Result, Router, Spellbook};
}

// TODO: users shouldn't have to import hyper to build a response
