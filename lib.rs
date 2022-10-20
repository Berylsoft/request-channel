use tokio::{
    sync::{
        mpsc::{unbounded_channel as _req_channel, UnboundedSender as _ReqTx, UnboundedReceiver as _ReqRx},
        oneshot::{channel as _res_channel, Sender as _ResTx, Receiver as _ResRx},
    },
    time::{timeout, Duration}
};

pub trait Req {
    type Res;
}

pub struct ReqPayload<R: Req> {
    pub req: R,
    pub res_tx: ResTx<R::Res>,
}

pub struct ReqTx<R: Req> {
    req_tx: _ReqTx<ReqPayload<R>>,
    timeout: Option<Duration>,
}

pub struct ReqRx<R: Req> {
    req_rx: _ReqRx<ReqPayload<R>>,
}

pub struct ResTx<Res> {
    res_tx: _ResTx<Res>,
}

pub struct ResRx<Res> {
    res_rx: Option<_ResRx<Res>>,
    timeout: Option<Duration>,
}

impl<R: Req> ReqTx<R> {
    pub fn send(&self, req: R) -> Result<ResRx<R::Res>, ReqSendError<R>> {
        let (res_tx, res_rx) = _res_channel::<R::Res>();
        self.req_tx
            .send(ReqPayload { req, res_tx: ResTx { res_tx } })
            .map_err(|payload| ReqSendError(payload.0.req))?;
        let res_rx = ResRx { res_rx: Some(res_rx), timeout: self.timeout };
        Ok(res_rx)
    }

    pub async fn send_recv(&self, request: R) -> Result<R::Res, ReqError<R>> {
        let mut res_rx = self.send(request)
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

impl<Res> ResTx<Res> {
    pub fn send(self, response: Res) -> Result<(), ResSendError<Res>> {
        self.res_tx.send(response).map_err(ResSendError)
    }

    pub fn is_closed(&self) -> bool {
        self.res_tx.is_closed()
    }
}

impl<Res> ResRx<Res> {
    pub async fn recv(&mut self) -> Result<Res, ResRecvError> {
        match self.res_rx.take() {
            Some(res_rx) => match self.timeout {
                Some(duration) => match timeout(duration, res_rx).await {
                    Ok(response_result) => response_result.map_err(|_| ResRecvError::RecvError),
                    Err(..) => Err(ResRecvError::RecvTimeout),
                },
                None => Ok(res_rx.await.map_err(|_| ResRecvError::RecvError)?),
            },
            None => Err(ResRecvError::RecvError),
        }
    }
}

pub fn channel<R: Req>() -> (ReqTx<R>, ReqRx<R>) {
    let (req_tx, req_rx) = _req_channel::<ReqPayload<R>>();
    (ReqTx { req_tx, timeout: None }, ReqRx { req_rx })
}

pub fn channel_with_timeout<R: Req>(timeout: Duration) -> (ReqTx<R>, ReqRx<R>) {
    let (req_tx, req_rx) = _req_channel::<ReqPayload<R>>();
    (ReqTx { req_tx, timeout: Some(timeout) }, ReqRx { req_rx })
}

pub struct ReqSendError<R: Req>(pub R);

pub struct ResSendError<Res>(pub Res);

pub enum ResRecvError {
    RecvError,
    RecvTimeout,
}

pub enum ReqError<R: Req> {
    SendError(R),
    RecvError,
    RecvTimeout,
}
