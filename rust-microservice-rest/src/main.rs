use std::fmt;
use std::sync::{Arc, Mutex};
use slab::Slab;
use futures::{future, Future};
use hyper::{Body, Response, Server, Error, Method, Request, StatusCode};
use hyper::service::service_fn;
use regex::Regex;
use lazy_static::lazy_static;

const INDEX_PAGE: &str = r#"
<!doctype html>
<html>
    <head>
        <title>Rust minimal microservice example</title>
    </head>
    <body>
    <h2>Rust minimal microservice example</h2>
    </body>
</html>
"#;

type ProductId = u64;
struct ProductData;

impl fmt::Display for ProductData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("{}")
    }
}

type ProductDb = Arc<Mutex<Slab<ProductData>>>;

// Regex routings
lazy_static! {
    static ref INDEX_PATH: Regex = Regex::new("^/(index\\.html?)?$").unwrap();
    static ref PRODUCT_PATH: Regex = Regex::new("^/product/((?P<product_id>\\d+?)/?)?$").unwrap();
    static ref PRODUCTS_PATH: Regex = Regex::new("^/products/?$").unwrap();
}

fn req_handler(req: Request<Body>, product_db: &ProductDb) -> impl Future<Item=Response<Body>, Error=Error>
{

    let response = {

    let method = req.method();
    let path = req.uri().path();
    let mut products = product_db.lock().unwrap();

    if INDEX_PATH.is_match(path) {
        if method == &Method::GET {
            Response::new(INDEX_PAGE.into())
        } else {
            response_with_code(StatusCode::METHOD_NOT_ALLOWED)
        }
    } else if PRODUCTS_PATH.is_match(path) {
        if method == &Method::GET {
            let list = products.iter()
            .map(|(id, _)| id.to_string())
            .collect::<Vec<String>>()
            .join(",");
        Response::new(list.into())
        } else {
            response_with_code(StatusCode::METHOD_NOT_ALLOWED)
        }
    } else if let Some(cap) = PRODUCT_PATH.captures(path) {
        let product_id = cap.name("product_id").and_then(|p| {
            p.as_str()
            .parse::<ProductId>()
            .ok()
            .map(|x| x as usize)
        });
        // detect the HTTP method
        match (method, product_id) {
            (&Method::GET, Some(id)) => {
                if let Some(data) = products.get(id) {
                    Response::new(data.to_string().into())
                } else {
                    response_with_code(StatusCode::NOT_FOUND)
                }
            },
            (&Method::POST, None) => {
                let id = products.insert(ProductData);
                Response::new(id.to_string().into())
            },
            (&Method::POST, Some(_)) => {
                response_with_code(StatusCode::BAD_REQUEST)
            },
            (&Method::PUT, Some(id)) => {
                if let Some(product) = products.get_mut(id) {
                    *product = ProductData;
                    response_with_code(StatusCode::OK)
                } else {
                    response_with_code(StatusCode::NOT_FOUND)
                }
            },
            (&Method::DELETE, Some(id)) => {
                if products.contains(id) {
                    products.remove(id);
                    response_with_code(StatusCode::OK)
                } else {
                    response_with_code(StatusCode::NOT_FOUND)
                }
            },
            _ => {
                response_with_code(StatusCode::METHOD_NOT_ALLOWED)
            },
        }
    } else {
        response_with_code(StatusCode::NOT_FOUND)
    }  

    };
    future::ok(response)

}

fn response_with_code(status_code: StatusCode) -> Response<Body> {
    Response::builder()
    .status(status_code)
    .body(Body::empty())
    .unwrap()
}


fn main() {

// create a socket address
let addr = ([127, 0, 0, 1], 8080).into();

let builder = Server::bind(&addr);

let product_db = Arc::new(Mutex::new(Slab::new()));

let server = builder.serve(move || {
    let product_db = product_db.clone();
    service_fn(move |req| req_handler(req, &product_db))
    });

let server = server.map_err(drop);

hyper::rt::run(server);

}
