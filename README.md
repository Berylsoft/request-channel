# request-channel

A simple channel on Tokio that supports request-response by MPSC and oneshot channel. Formerly forked from [bmrng](https://github.com/oguzbilgener/bmrng) to trait-ify but nearly completely refactored.

```rust
use request_channel::{Req, ReqPayload, unbounded::channel};

#[derive(Debug)]
struct Request;
#[derive(Debug)]
struct Response;

impl Req for Request {
    type Res = Response;
}

#[tokio::main]
async fn main() {
    let (req_tx, mut req_rx) = channel::<Request>();
    tokio::spawn(async move {
        while let Ok(ReqPayload { req, res_tx }) = req_rx.recv().await {
            println!("(spawned thread) recv {:?}", req);
            res_tx.send(Response).unwrap();
        }
    });
    println!("recv {:?}", req_tx.send_recv(Request).await.unwrap());
}
```
