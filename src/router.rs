use build_chain;
use Context;
use Handler;
use Request;
use Result;
use Route;
use Tween;

use std::rc::Rc;

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

pub fn handle<A: Clone + 'static>(router: &Router<A>, app: A, req: Rc<Request>) -> Result {
    // TODO: this is dummy code
    let handler = router.handlers[0].clone();
    let route = Rc::new(Route::new("/users/:user_id", req.uri()));
    let context = Context {
        app: app,
        route: route,
        req: req.clone(),
    };

    let next = Box::new(move |ctx: Context<A>| handler(ctx));
    let chain = build_chain(context.clone(), router.tweens.clone(), next);
    chain(context)
}
