use std::time::Duration;
use tokio::sync::mpsc::{channel as _req_channel, Sender as _ReqTx, Receiver as _ReqRx};
use crate::{ResTx, ResRx, ReqError, ReqSendError, ResRecvError, _res_channel};

pub struct ReqTx<Req, Res> {
    req_tx: _ReqTx<(Req, ResTx<Res>)>,
    timeout: Option<Duration>,
}

pub struct ReqRx<Req, Res> {
    req_rx: _ReqRx<(Req, ResTx<Res>)>,
}

impl<Req, Res> ReqTx<Req, Res> {
    pub async fn send(&self, req: Req) -> Result<ResRx<Res>, ReqSendError<Req>> {
        let (res_tx, res_rx) = _res_channel::<Res>();
        self.req_tx
            .send((req, ResTx { res_tx })).await
            .map_err(|payload| ReqSendError(payload.0.0))?;
        let res_rx = ResRx { res_rx: Some(res_rx), timeout: self.timeout };
        Ok(res_rx)
    }

    pub async fn send_recv(&self, request: Req) -> Result<Res, ReqError<Req>> {
        let mut res_rx = self.send(request).await
            .map_err(|err| ReqError::SendError(err.0))?;
        res_rx.recv().await
            .map_err(|err| match err {
                ResRecvError::RecvError => ReqError::RecvError,
                ResRecvError::RecvTimeout => ReqError::RecvTimeout,
            })
    }

    pub fn is_closed(&self) -> bool {
        self.req_tx.is_closed()
    }
}

impl<Req, Res> Clone for ReqTx<Req, Res> {
    fn clone(&self) -> Self {
        ReqTx {
            req_tx: self.req_tx.clone(),
            timeout: self.timeout,
        }
    }
}

impl<Req, Res> ReqRx<Req, Res> {
    pub async fn recv(&mut self) -> Result<(Req, ResTx<Res>), ReqError<Req>> {
        match self.req_rx.recv().await {
            Some(payload) => Ok(payload),
            None => Err(ReqError::<Req>::RecvError),
        }
    }

    pub fn close(&mut self) {
        self.req_rx.close()
    }
}

pub fn channel<Req, Res>(buffer: usize) -> (ReqTx<Req, Res>, ReqRx<Req, Res>) {
    let (req_tx, req_rx) = _req_channel::<(Req, ResTx<Res>)>(buffer);
    (ReqTx { req_tx, timeout: None }, ReqRx { req_rx })
}

pub fn channel_with_timeout<Req, Res>(buffer: usize, timeout: Duration) -> (ReqTx<Req, Res>, ReqRx<Req, Res>) {
    let (req_tx, req_rx) = _req_channel::<(Req, ResTx<Res>)>(buffer);
    (ReqTx { req_tx, timeout: Some(timeout) }, ReqRx { req_rx })
}
