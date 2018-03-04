use build_chain;
use Context;
use Handler;
use Request;
use Result;
use Tween;

use hyper;

use std::collections::HashMap;
use std::rc::Rc;
use std::result;
use std::str::FromStr;

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
}

pub fn handle<A: Clone + 'static>(router: &Router<A>, app: Rc<A>, req: Rc<Request>) -> Result {
    // TODO: this is dummy code
    let handler = router.handlers[0].clone();
    let route = Rc::new(Route::new("/users/:user_id", req.uri()));
    let context = Rc::new(Context {
        app: app,
        route: route,
        req: req.clone(),
    });

    let next = Box::new(move |ctx: Rc<Context<A>>| handler(ctx));
    let chain = build_chain(context.clone(), router.tweens.clone(), next);
    chain(context)
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
    pub fn get<T: FromStr>(&self, key: &str) -> result::Result<T, &'static str> {
        match self.params.get(key) {
            Some(s) => match s.parse() {
                Ok(v) => Ok(v),
                Err(_) => Err("value wrong type"),
            },
            None => Err("value does not exist"),
        }
    }
}
