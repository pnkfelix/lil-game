use lambda_runtime::{handler_fn, Context};
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::Error as _;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Deserialize)]
struct Request {
    path: String,
}

#[derive(Serialize)]
struct Response {
    body: DoublyEncode<ResponseBody>,
    // This should be a u32, but API Gateway actually expects a String that looking like an int for some reason.
    statusCode: String
}

struct DoublyEncode<T>(pub T);

impl<T:Serialize> Serialize for DoublyEncode<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let first_encoded = serde_json::to_string(&self.0).map_err(|err| S::Error::custom(err))?;
        serializer.serialize_str(&first_encoded)
    }
}

#[derive(Serialize)]
struct ResponseBody {
    hello: String
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(my_handler);
    lambda_runtime::run(func).await?;
    Ok(())
}

pub(crate) async fn my_handler(event: Request, ctx: Context) -> Result<Response, Error> {

    let resp = Response {
        body: DoublyEncoade(ResponseBody {hello: String::from("world")}),
        statusCode: String::from("200")
    };

    Ok(resp)
}