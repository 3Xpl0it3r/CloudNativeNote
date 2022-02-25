use futures::prelude::*;
use tokio::process::*;
async fn app(){
    todo!()
}

fn main() {
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let future = app();
    rt.block_on(future);
}

/*
 * 这段代码等于上面那段代码 
#[tokio::main]
async fn main(){

}
*/
