#![crate_name = "spellbook"]

extern crate futures;
extern crate hyper;
extern crate serde;
extern crate serde_urlencoded;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

mod router;
pub use router::Router;

use std::collections::HashMap;
use std::error::Error;
use std::pin::Pin;
use std::rc::Rc;
use std::result::Result as StdResult;
use std::str::FromStr;

use futures::future::Future;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::Service;
pub use hyper::Response;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub type Request = hyper::Request<Incoming>;
pub type Result = std::result::Result<Response<String>, Box<dyn Error + Send + Sync>>;
pub type Next<'a, S> = &'a dyn Fn(Context<S>) -> Result;
pub type Handler<S> = fn(Context<S>) -> Result;
pub type Tween<S> = fn(Context<S>, Next<S>) -> Result;

// TODO: make handlers take &Context or Rc<Context>
// TODO: make handlers be async
// TODO: make custom body type so we can construct one for tests
// TODO: fix tests
// TODO: cleanup and pin dependencies

#[derive(Clone)]
pub struct Server<S: Clone> {
    router: Router<S>,
    state: S,
}

impl<S: Clone + 'static + Send + Sync> Server<S> {
    pub fn new(state: S, router: Router<S>) -> Server<S> {
        return Server {
            router: router,
            state: state,
        };
    }

    pub fn serve(self, address: SocketAddr) -> std::io::Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { self.serve_until(address).await })
    }

    async fn serve_until(self, addr: SocketAddr) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on http://{}", addr);

        loop {
            let s = self.clone();
            let (stream, _) = listener.accept().await?;

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new().serve_connection(stream, s).await {
                    println!("Failed to serve connection: {:?}", err);
                }
            });
        }
    }
}

impl<S: Clone + 'static> Service<Request> for Server<S> {
    type Response = Response<String>;
    type Error = Box<dyn Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = StdResult<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request) -> Self::Future {
        let res = self.router.handle(self.state.clone(), Rc::new(req));

        let body = match res {
            Ok(body) => body,
            Err(e) => {
                let message = format!("{}", e);
                Response::builder()
                    .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                    .body(message)
                    .unwrap()
            }
        };

        Box::pin(async { Ok(body) })
    }
}

pub struct Route {
    params: HashMap<String, String>,
}

impl Route {
    fn new() -> Route {
        Route {
            params: HashMap::new(),
        }
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
    pub fn from_params(params: HashMap<String, String>) -> Route {
        Route { params: params }
    }

    pub fn params<P>(&self) -> StdResult<P, serde_urlencoded::de::Error>
    where
        for<'a> P: serde::Deserialize<'a>,
    {
        serde_urlencoded::from_str(serde_urlencoded::to_string(&self.params).unwrap().as_str())
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
    pub req: Option<Rc<Request>>,
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
            req: None,
            route: Rc::new(Route {
                params: HashMap::new(),
            }),
            state: state,
        }
    }

    /// Parses route-specified params to a value
    pub fn route_params<P>(&self) -> StdResult<P, serde_urlencoded::de::Error>
    where
        for<'a> P: serde::Deserialize<'a>,
    {
        self.route.params()
    }

    /// Parses query params to a value
    pub fn query_params<P>(&self) -> StdResult<P, serde_urlencoded::de::Error>
    where
        for<'a> P: serde::Deserialize<'a>,
    {
        let req = self.req.as_ref().unwrap();
        let query_params_string = req.uri().query().unwrap_or("");
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
    use std::str::from_utf8;
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

        Ok(Response::builder().body(body).unwrap())
    }

    fn foo(_: Context<State>) -> Result {
        Ok(Response::builder().body(String::from("foo")).unwrap())
    }

    #[derive(Deserialize)]
    struct BarVals {
        val: u32,
    }

    fn bar(context: Context<State>) -> Result {
        let val: u32 = context.route.get("val")?;
        let bar_vals: BarVals = context.route_params()?;
        assert_eq!(val, bar_vals.val);
        Ok(Response::builder().body(format!("bar:{}", val)).unwrap())
    }

    fn baz(_: Context<State>) -> Result {
        Ok(Response::builder().body(String::from("baz")).unwrap())
    }

    #[derive(Deserialize, Debug)]
    struct QueryParamTest {
        foo: Option<String>,
        bar: Option<u32>,
    }

    fn query_param_test(context: Context<State>) -> Result {
        let params: QueryParamTest = context.query_params()?;
        Ok(Response::builder().body(format!("{:?}", params)).unwrap())
    }

    fn do_test(router: &Router<State>, path: &str, expected_body: String) {
        /*
        let state = State {
            name: None,
        };

        let result = router.handle(
            state,
            Rc::new(
                hyper::Request::builder()
                    .method(hyper::Method::GET)
                    .uri(path).body(UNIMPLEMENTED)
                    .unwrap()
            )
        );

        let response_bytes: Vec<u8> = result.unwrap().body().concat2().wait().unwrap().into_iter().collect();
        let response: String = from_utf8(&response_bytes).unwrap().to_string();

        assert_eq!(response, expected_body);
        */
    }

    #[test]
    fn test_simple_handler() {
        let router = Router::new().get("/", index);

        do_test(&router, "http://localhost/", String::from("Hello World!"));
    }

    #[test]
    fn test_middleware() {
        let router = Router::new().with(name_middleware).get("/", index);

        do_test(
            &router,
            "http://localhost/",
            String::from("Hello Walt Longmire!"),
        );
    }

    #[test]
    fn test_routing() {
        let router = Router::new()
            .get("/foo", foo)
            .get("/bar/:val", bar)
            .get("/baz/*", baz);

        do_test(&router, "http://localhost/foo", String::from("foo"));

        do_test(&router, "http://localhost/bar/42", String::from("bar:42"));

        do_test(
            &router,
            "http://localhost/baz/quux/x/y/z",
            String::from("baz"),
        );
    }

    #[test]
    fn test_query_params() {
        let router = Router::new().get("/query_param_test", query_param_test);

        do_test(
            &router,
            "http://localhost/query_param_test?foo=thing&bar=42",
            String::from("QueryParamTest { foo: Some(\"thing\"), bar: Some(42) }"),
        );

        do_test(
            &router,
            "http://localhost/query_param_test",
            String::from("QueryParamTest { foo: None, bar: None }"),
        );
    }
}
