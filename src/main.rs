use std::{task::{Poll, Context, ready}, pin::{Pin, pin}};
use futures::{stream::{Stream,StreamExt}, FutureExt, pin_mut, Future};
use pin_project::*;
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
async fn my_async_transform(v: usize) -> usize {
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

#[pin_project]
pub struct TransformedStream<Source,F>
where 
    Source : Stream, //<Item = Item>,
    F : Future<Output = usize>
{
    #[pin]
    // stream : dyn Stream<Item=usize> // This needs to be Pin<>
    stream : Source, //dyn Stream<Item=usize>, // This needs to be Pin<>

    #[pin]
    pending : Option<F>,
}

impl<Source,F> Stream for TransformedStream<Source,F> 
where
    Source : Stream<Item = usize> ,
    F: Future<Output = usize>
{
    type Item = usize;

    fn poll_next(self : Pin<&mut Self>, cx : &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let project = self.project();
        let stream = project.stream;
        let pending = project.pending;
        if pending.is_some() {
            let mut fut = pending.unwrap();
            pin_mut!(fut);
            let p = ready!(fut.poll_unpin(cx));
            Poll::Ready(Some(p))    
        } else {

            match stream.poll_next(cx) {
                Poll::Ready(Some(v)) => {
                    let mut fut = my_async_transform(v);
                    let mut fut = pin!(fut);

                    match fut.poll(cx) {
                        Poll::Ready(v) => Poll::Ready(Some(v)),
                        Poll::Pending => {
                            // not sure how to retain this in the self.pending
                            project.pending = Pin::new(fut);
                            Poll::Pending
                        }
                    }
                },
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------

#[async_std::main]
async fn main() {

    let streamA = TestStream::default();

    // let sA : dyn Stream<Item=usize> = &mut streamA;
    // let sA : dyn Stream<Item=usize> = streamA;

    let mut streamB = TransformedStream {
        stream : streamA
    };

    pin_mut!(streamB);
    while let Some(item) = streamB.next().await {
        println!("{}", item);
    }
}