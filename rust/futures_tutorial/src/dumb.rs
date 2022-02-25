
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};




use tracing::info;

pub struct DumpFuture {}


impl Future for DumpFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        info!("this is poll");
        Poll::Ready(())
    }
}

