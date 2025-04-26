#![allow(unused)]

use std::{error::Error, marker::PhantomData, pin::Pin, task::Poll};

use bytes::Buf;
use hyper::body::{Body, Frame};

pub struct EmptyBody<D: Buf, E> {
    _data: PhantomData<D>,
    _error: PhantomData<E>,
}

impl<D: Buf, E> EmptyBody<D, E> {
    pub fn new() -> Self {
        Self {
            _data: PhantomData,
            _error: PhantomData,
        }
    }
}

impl<D: Buf, E> Body for EmptyBody<D, E> {
    type Data = D;
    type Error = E;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(None)
    }
}

pub struct FullBody<D: Buf, E: Error> {
    data: Option<D>,
    _error: PhantomData<E>,
}

impl<D: Buf, E: Error> FullBody<D, E> {
    pub fn new(data: D) -> Self {
        Self {
            data: Some(data),
            _error: PhantomData,
        }
    }
}

impl<D: Buf + Unpin, E: Error + Unpin> Body for FullBody<D, E> {
    type Data = D;
    type Error = E;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(self.get_mut().data.take().map(|d| Ok(Frame::data(d))))
    }
}
