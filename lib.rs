use tokio::time::{timeout, Duration};
pub(crate) use tokio::sync::oneshot::{channel as _res_channel, Sender as _ResTx, Receiver as _ResRx};

pub struct ResTx<Res> {
    res_tx: _ResTx<Res>,
}

pub struct ResRx<Res> {
    res_rx: Option<_ResRx<Res>>,
    timeout: Option<Duration>,
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

#[derive(Debug)]
pub struct ReqSendError<Req>(pub Req);

#[derive(Debug)]
pub struct ResSendError<Res>(pub Res);

#[derive(Debug)]
pub enum ResRecvError {
    RecvError,
    RecvTimeout,
}

#[derive(Debug)]
pub enum ReqError<Req> {
    SendError(Req),
    RecvError,
    RecvTimeout,
}

pub mod bounded;
pub mod unbounded;
