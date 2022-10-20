use std::time::Duration;
use tokio::sync::mpsc::{channel as _req_channel, Sender as _ReqTx, Receiver as _ReqRx};
use crate::{Req, ReqPayload, ResTx, ResRx, ReqError, ReqSendError, ResRecvError, _res_channel};

pub struct ReqTx<R: Req> {
    req_tx: _ReqTx<ReqPayload<R>>,
    timeout: Option<Duration>,
}

pub struct ReqRx<R: Req> {
    req_rx: _ReqRx<ReqPayload<R>>,
}

impl<R: Req> ReqTx<R> {
    pub async fn send(&self, req: R) -> Result<ResRx<R::Res>, ReqSendError<R>> {
        let (res_tx, res_rx) = _res_channel::<R::Res>();
        self.req_tx
            .send(ReqPayload { req, res_tx: ResTx { res_tx } }).await
            .map_err(|payload| ReqSendError(payload.0.req))?;
        let res_rx = ResRx { res_rx: Some(res_rx), timeout: self.timeout };
        Ok(res_rx)
    }

    pub async fn send_recv(&self, request: R) -> Result<R::Res, ReqError<R>> {
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

impl<R: Req> Clone for ReqTx<R> {
    fn clone(&self) -> Self {
        ReqTx {
            req_tx: self.req_tx.clone(),
            timeout: self.timeout,
        }
    }
}

impl<R: Req> ReqRx<R> {
    pub async fn recv(&mut self) -> Result<ReqPayload<R>, ReqError<R>> {
        match self.req_rx.recv().await {
            Some(payload) => Ok(payload),
            None => Err(ReqError::RecvError),
        }
    }

    pub fn close(&mut self) {
        self.req_rx.close()
    }
}

pub fn channel<R: Req>(buffer: usize) -> (ReqTx<R>, ReqRx<R>) {
    let (req_tx, req_rx) = _req_channel::<ReqPayload<R>>(buffer);
    (ReqTx { req_tx, timeout: None }, ReqRx { req_rx })
}

pub fn channel_with_timeout<R: Req>(buffer: usize, timeout: Duration) -> (ReqTx<R>, ReqRx<R>) {
    let (req_tx, req_rx) = _req_channel::<ReqPayload<R>>(buffer);
    (ReqTx { req_tx, timeout: Some(timeout) }, ReqRx { req_rx })
}
