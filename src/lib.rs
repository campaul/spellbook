#![crate_name = "spellbook"]

extern crate futures;
extern crate hyper;
extern crate serde;
extern crate serde_urlencoded;
#[cfg(test)] #[macro_use] extern crate serde_derive;

mod router;
pub use router::Router;

use futures::future::Future;

use std::error::Error;

use std::rc::Rc;
use std::collections::HashMap;
use std::str::FromStr;
use std::result::Result as StdResult;

pub type Request = hyper::Request<hyper::Body>;
pub type Response = hyper::Response;
pub type Result = std::result::Result<hyper::Response, Box<Error>>;
pub type Next<'a, S> = &'a Fn(Context<S>) -> Result;
pub type Handler<S> = fn(Context<S>) -> Result;
pub type Tween<S> = fn(Context<S>, Next<S>) -> Result;

#[derive(Clone)]
pub struct Server<S: Clone> {
    router: Router<S>,
    state: S,
}

impl<S: Clone + 'static> Server<S> {
    pub fn new(state: S, router: Router<S>) -> Server<S> {
        return Server {
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

impl<S: Clone + 'static> hyper::server::Service for Server<S> {
    type Request = hyper::server::Request;
    type Response = hyper::server::Response;
    type Error = hyper::Error;

    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: hyper::server::Request) -> Self::Future {
        let res = self.router.handle(self.state.clone(), Rc::new(req));

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
    fn new() -> Route {
        Route { params: HashMap::new() }
    }

    /// Creates a Route from a params map.
    /// This is useful for testing.
    ///
    /// # Arguments
    ///
    /// * `params` - A String to String map of request params
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// use spellbook::Route;
    ///
    /// let mut map = HashMap::new();
    /// map.insert(String::from("name"), String::from("Walt"));
    /// map.insert(String::from("age"), String::from("42"));
    ///
    /// let route = Route::from_params(map);
    ///
    /// assert_eq!(route.get::<String>("name").unwrap(), "Walt");
    /// assert_eq!(route.get::<u32>("age").unwrap(), 42);
    pub fn from_params(params: HashMap<String, String>) -> Route{
        Route {
            params: params,
        }
    }

    pub fn params<P>(&self) -> StdResult<P, serde_urlencoded::de::Error> where for<'a> P: serde::Deserialize<'a> {
        serde_urlencoded::from_str(
            serde_urlencoded::to_string(&self.params).unwrap().as_str(),
        )
    }

    /// Returns the value of a request param.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of a request param
    /// ```
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
pub struct Context<S: Clone> {
    pub state: S,
    pub route: Rc<Route>,
    pub req: Rc<Request>,
}

impl<S: Clone> Context<S> {
    /// Creates a new Context with the same route and req as the original
    /// Context, but with the given state.
    ///
    /// # Arguments
    ///
    /// * `state` - Some arbitrary state
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// use spellbook::Context;
    /// use spellbook::Request;
    /// use spellbook::Route;
    ///
    /// let ctx1 = Context::empty("one");
    /// let ctx2 = ctx1.with("two");
    ///
    /// assert_eq!(ctx1.req.uri(), ctx2.req.uri());
    /// assert_eq!(ctx2.state, "two");
    /// ```
    pub fn with(&self, state: S) -> Context<S> {
        Context {
            state: state,
            route: self.route.clone(),
            req: self.req.clone(),
        }
    }

    /// Creates a Context with the route "/", no params, and the given state.
    /// This is useful for testing.
    ///
    /// # Arguments
    ///
    /// * `state` - Some arbitrary state
    ///
    /// # Example
    ///
    /// ```
    /// use spellbook::Context;
    ///
    /// let ctx = Context::empty(());
    ///
    /// assert_eq!(ctx.req.uri().path(), "/");
    /// assert_eq!(ctx.state, ());
    /// ```
    pub fn empty(state: S) -> Context<S> {
        Context {
            req: Rc::new(Request::new(
                hyper::Method::Get,
                hyper::Uri::from_str("/").unwrap()
            )),
            route: Rc::new(Route { params: HashMap::new() }),
            state: state,
        }
    }

    pub fn route_params<P>(&self) -> StdResult<P, serde_urlencoded::de::Error>
        where for<'a> P: serde::Deserialize<'a> {
        self.route.params()
    }

    pub fn query_params<P>(&self) -> StdResult<P, Box<Error>>
        where for<'a> P: serde::Deserialize<'a> {
        let query_params_string = self.req.query().ok_or("no query params")?;
        let query_params: P = serde_urlencoded::from_str(query_params_string)?;
        Ok(query_params)
    }
}

#[cfg(test)]
mod tests {
    extern crate hyper;

    use super::Context;
    use super::Next;
    use super::Response;
    use super::Result;
    use super::Router;
    use std::rc::Rc;
    use std::str::FromStr;

    #[derive(Clone)]
    struct State {
        name: Option<String>,
    }

    fn name_middleware(context: Context<State>, next: Next<State>) -> Result {
        let new_state = State {
            name: Some(String::from("Walt Longmire")),
        };
        next(context.with(new_state))
    }

    fn index(context: Context<State>) -> Result {
        let body = match context.state.name {
            Some(name) => format!("Hello {}!", name),
            None => String::from("Hello World!"),
        };

        Ok(Response::new()
            .with_body(body))
    }

    fn foo(_: Context<State>) -> Result {
        Ok(Response::new().with_body("foo"))
    }

    #[derive(Deserialize)]
    struct BarVals {
        val: u32,
    }

    fn bar(context: Context<State>) -> Result {
        let val: u32 = context.route.get("val")?;
        let bar_vals: BarVals = context.route_params()?;
        assert_eq!(val, bar_vals.val);
        Ok(Response::new().with_body(format!("bar:{}", val)))
    }

    fn baz(_: Context<State>) -> Result {
        Ok(Response::new().with_body("baz"))
    }

    #[derive(Deserialize, Debug)]
    struct QueryParamTest {
        foo: String,
        bar: u32,
    }

    fn query_param_test(context: Context<State>) -> Result {
        let params: QueryParamTest = context.query_params()?;
        Ok(Response::new().with_body(format!("{:?}", params)))
    }

    fn do_test(router: &Router<State>, path: &str, expected_body: String) {
        let state = State {
            name: None,
        };

        let result = router.handle(
            state,
            Rc::new(hyper::Request::new(hyper::Method::Get, hyper::Uri::from_str(path).unwrap()))
        );

        let response = result.unwrap();
        let expected_response = Response::new()
            .with_header(hyper::header::ContentLength(expected_body.len() as u64))
            .with_body(expected_body);

        assert_eq!(
            format!("{:?}", response.body()),
            format!("{:?}", expected_response.body()),
        );
    }

    #[test]
    fn test_simple_handler() {
        let router = Router::new()
            .get("/", index);

        do_test(
            &router,
            "http://localhost/",
            String::from("Hello World!")
        );
    }

    #[test]
    fn test_middleware() {
        let router = Router::new()
            .with(name_middleware)
            .get("/", index);

        do_test(
            &router,
            "http://localhost/",
            String::from("Hello Walt Longmire!")
        );
    }

    #[test]
    fn test_routing() {
        let router = Router::new()
            .get("/foo", foo)
            .get("/bar/:val", bar)
            .get("/baz/*", baz)
            .get("/query_param_test", query_param_test);

        do_test(
            &router,
            "http://localhost/foo",
            String::from("foo")
        );

        do_test(
            &router,
            "http://localhost/bar/42",
            String::from("bar:42")
        );

        do_test(
            &router,
            "http://localhost/baz/quux/x/y/z",
            String::from("baz")
        );

        do_test(
            &router,
            "http://localhost/query_param_test?foo=thing&bar=42",
            String::from("QueryParamTest { foo: \"thing\", bar: 42 }"),
        )
    }
}
