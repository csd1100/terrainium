use std::borrow::Borrow;
use std::cmp::PartialEq;
use std::fmt::Debug;

use anyhow::bail;
use mockall::predicate::eq;
use prost::Message;

use crate::socket::MockSocket;

pub struct Mocket<
    In: Message + Default + Debug + PartialEq + Borrow<In> + 'static,
    Out: Message + Debug + PartialEq + 'static,
> {
    req: Option<Out>,
    res: Option<In>,
}

impl<
    In: Message + Default + Debug + PartialEq + Borrow<In> + 'static,
    Out: Message + Debug + PartialEq + 'static,
> Mocket<In, Out>
{
    pub fn to() -> Self {
        Self {
            req: None,
            res: None,
        }
    }

    pub fn send(mut self, req: Out) -> Self {
        self.req = Some(req);
        self
    }

    pub fn receive(mut self, res: In) -> Self {
        self.res = Some(res);
        self
    }

    pub fn successfully(self) -> MockSocket<In, Out> {
        let Mocket { req, res } = self;

        let req = req.expect("mocket to be setup with request");
        let res = res.expect("mocket to be setup with response");

        let mut client = MockSocket::<In, Out>::default();
        client
            .expect_request()
            .with(eq(req))
            .return_once(move |_| Ok(res))
            .times(1);
        client
    }

    pub fn with_returning_error(self, error: &str) -> MockSocket<In, Out> {
        let Mocket { req, .. } = self;
        let error = error.to_string();

        let req = req.expect("mocket to be setup with request");

        let mut client = MockSocket::<In, Out>::default();
        client
            .expect_request()
            .with(eq(req))
            .return_once(move |_| bail!("{error}"))
            .times(1);

        client
    }
}
