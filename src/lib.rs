extern crate futures;
extern crate hyper;

use futures::future::Future;

use std::error::Error;

use std::rc::Rc;

pub type Request = hyper::Request<hyper::Body>;
pub type Response = hyper::Response;
pub type Result = std::result::Result<hyper::Response, Box<Error>>;
pub type Next<'a> = &'a Fn() -> Result;

#[derive(Clone)]
pub struct Router<A: Clone> {
    handlers: Vec<fn(Rc<A>, Rc<Request>) -> Result>,
    tweens: Vec<fn(Rc<A>, Rc<Request>, &Fn() -> Result) -> Result>,
    phantom: std::marker::PhantomData<A>,
}

impl<A: Clone + 'static> Router<A> {
    pub fn new() -> Router<A> {
        Router {
            handlers: vec![],
            tweens: vec![],
            phantom: std::marker::PhantomData,
        }
    }

    pub fn get(mut self, _path: &str, handler: fn(Rc<A>, Rc<Request>) -> Result) -> Router<A> {
        self.handlers.push(handler);
        self
    }

    pub fn with(mut self, tween: fn(Rc<A>, Rc<Request>, &Fn() -> Result) -> Result) -> Router<A> {
        self.tweens.insert(0, tween);
        self
    }

    fn handle(&self, app: Rc<A>, req: Rc<Request>) -> Result {
        let handler = self.handlers[0].clone();
        let req_clone = req.clone();
        let app_clone = app.clone();

        let next = Box::new(move || handler(app_clone.clone(), req_clone.clone()));

        let chain = build_chain(app, req, self.tweens.clone(), next);
        chain()
    }
}

fn build_chain<A: Clone + 'static>(
    app: Rc<A>,
    req: Rc<Request>,
    mut tweens: Vec<fn(Rc<A>, Rc<Request>, &Fn() -> Result) -> Result>,
    next: Box<Fn() -> Result>
) -> Box<Fn() -> Result> {
    if tweens.len() == 0 {
        return next;
    }

    let tween = tweens.pop().unwrap();
    let chain = build_chain(app.clone(), req.clone(), tweens.clone(), next);
    return Box::new(move || tween(app.clone(), req.clone(), &*chain))
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
        }
    }

    pub fn serve(self, address: &'static str) {
        let addr = address.parse().unwrap();
        let server = hyper::server::Http::new().bind(
            &addr, move || Ok(self.clone())
        ).unwrap();
        println!("Server running at {}", address);
        server.run().unwrap();
    }
}

impl<A: Clone + 'static> hyper::server::Service for Spellbook<A> {
    type Request = hyper::server::Request;
    type Response = hyper::server::Response;
    type Error = hyper::Error;

    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: hyper::server::Request) -> Self::Future {
        let res = self.router.handle(self.app.clone(), Rc::new(req));

        let body = res.unwrap();

        Box::new(futures::future::ok(
            body
        ))
    }
}
