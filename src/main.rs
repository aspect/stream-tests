use std::{task::{Poll, Context}, pin::Pin};
use futures::stream::{Stream,StreamExt};

// A simple source stream

#[derive(Default)]
pub struct TestStream {
    value : usize
}

impl Stream for TestStream {
    type Item = usize;

    fn poll_next(mut self : Pin<&mut Self>, _cx : &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.value < 10 {
            let result = Poll::Ready(Some(self.value));
            self.value += 1;
            result
        } else {
            Poll::Ready(None)
        }
    }
}

// some async transform function
async fn my_transform(v: usize) -> usize {
    v * 100
}

// -------------------------------------------------------------------------------------------------
//
//   TransformedStream needs to source data from the source stream and transform it via
//   the async function.  There is no blocking spawn() functionality as this must
//   run in the browser (also no tokio, async_std only).
//
//   Secon issue: how to encapsulre the source stream which needs to be pinned...
//
//   Goal: As an API, I need to provide user with an async iterator (simple, just make a stream),
//   however, I need the incoming data to be transformed via the async function.
//   The resulting API still needs to be a stream:
//
//  let mut stream = some_api.get_some_stream().await;  // this also needs to be async but that is irrelvant
//  while let Some(item) = stream.next().await {
//    println!("{}", item);
//  }
//
//

pub struct TransformedStream {
    stream : Box<dyn Stream<Item=usize>> // This needs to be Pin<>
}

impl Stream for TransformedStream {
    type Item = usize;

    fn poll_next(self : Pin<&mut Self>, cx : &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.stream.poll_next(cx) {
            Poll::Ready(Some(v)) => {
                
                let v = my_transform(v).await; // how to handle this
                Poll::Ready(Some(v))
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

// -------------------------------------------------------------------------------------------------

#[async_std::main]
async fn main() {

    let streamA = TestStream::default();

    let mut streamB = TransformedStream {
        stream : Box::new(streamA)
    };

    while let Some(item) = streamB.next().await {
        println!("{}", item);
    }
}