use build_chain;
use Context;
use Handler;
use Request;
use Result;
use Route;
use Tween;

use std::rc::Rc;

#[derive(Clone)]
pub struct Router<S: Clone> {
    handlers: Vec<Handler<S>>,
    tweens: Vec<Tween<S>>,
}

impl<S: Clone + 'static> Router<S> {
    pub fn new() -> Router<S> {
        Router {
            handlers: vec![],
            tweens: vec![],
        }
    }

    pub fn get(mut self, _path: &str, handler: Handler<S>) -> Router<S> {
        self.handlers.push(handler);
        self
    }

    pub fn with(mut self, tween: Tween<S>) -> Router<S> {
        self.tweens.insert(0, tween);
        self
    }
}

pub fn handle<S: Clone + 'static>(router: &Router<S>, state: S, req: Rc<Request>) -> Result {
    // TODO: this is dummy code
    let handler = router.handlers[0].clone();
    let route = Rc::new(Route::new("/users/:user_id", req.uri()));
    let context = Context {
        state: state,
        route: route,
        req: req.clone(),
    };

    let chain = build_chain(router.tweens.clone(), Box::new(handler));
    chain(context)
}
