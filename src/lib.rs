extern crate futures;
extern crate hyper;

mod router;
pub use router::Router;
pub use router::Route;

use futures::future::Future;

use std::error::Error;

use std::rc::Rc;

pub type Request = hyper::Request<hyper::Body>;
pub type Response = hyper::Response;
pub type Result = std::result::Result<hyper::Response, Box<Error>>;
pub type Next<'a, A> = &'a Fn(Rc<Context<A>>) -> Result;
pub type Handler<A> = fn(Rc<Context<A>>) -> Result;
pub type Tween<A> = fn(Rc<Context<A>>, Next<A>) -> Result;

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
        let res = router::handle(&self.router, self.app.clone(), Rc::new(req));

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

pub struct Context<A: Clone> {
    pub app: Rc<A>,
    pub route: Rc<Route>,
    pub req: Rc<Request>,
}

pub mod prelude {
    pub use {Context, Next, Response, Result, Router, Spellbook};
}

// TODO: users shouldn't have to import hyper to build a response
